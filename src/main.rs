mod templates;
mod wasm_opt;
use cargo_metadata::{self, Error, Metadata, Package};
use std::{
    collections::BTreeSet, io::Write, path::PathBuf, process::Command, str::FromStr, sync::Arc,
};
use structopt::StructOpt;
use wasm_opt::WasmOpt;

const WASM32_UNKNOWN_UNKNOWN: &str = "wasm32-unknown-unknown";
const WASM_BINDGEN: &str = "wasm-bindgen";
const WASM_BINDGEN_CLI: &str = "wasm-bindgen-cli";
const OUT_DIR: &str = "dist/js";

fn path_to_cli(wasm_bindgen_version: &str) -> PathBuf {
    let mut path = PathBuf::from("target");
    path.push(WASM_BINDGEN_CLI);
    path.push(wasm_bindgen_version);
    path.push("bin");
    path.push(WASM_BINDGEN);
    path
}

#[derive(Clone)]
struct Cargo(String);

impl Cargo {
    // TODO: Only run if not there... `exists()` still works, if just the file is deleted?
    // if !Path::new(&path_to_cli(wasm_bindgen_version)).exists() {
    // }
    // TODO: Write test to make sure that correct version is installed: check .crates.toml file
    fn install_wasm_bindgen_cli(&self, wasm_bindgen_version: &str) {
        let mut path = path_to_cli(wasm_bindgen_version);
        if path.exists() {
            println!(
                "{} {} already installed",
                WASM_BINDGEN_CLI, wasm_bindgen_version
            );
        } else {
            path.pop();
            path.pop();
            println!("Installing {}: {}", WASM_BINDGEN_CLI, wasm_bindgen_version);
            let mut cmd = Command::new(&self.0);
            cmd.args(&[
                "install",
                "--root",
                &path.display().to_string(),
                "--version",
                wasm_bindgen_version,
                "--",
                WASM_BINDGEN_CLI,
            ]);
            // TODO: Show this so can see something is happening?
            cmd.output().expect("Unable to install wasm_bindgen_cli");
        }
    }

    fn build_wasm32_unknown_unknown(&self, package_name: &str, opt: &Opt) {
        println!("Building {} for {}", WASM32_UNKNOWN_UNKNOWN, package_name);
        let mut cmd = Command::new(&self.0);
        cmd.args(&[
            "build",
            "--package",
            package_name,
            "--target",
            WASM32_UNKNOWN_UNKNOWN,
        ]);
        if opt.release {
            cmd.arg("--release");
        }
        cmd.output()
            .expect("unable run cargo to build wasm32 target");
    }

    // TODO: is cargo new the best way here? Using for now since it gets the local author.
    fn new_template_project(&self, name: &str, bundler: bool) {
        let mut cmd = Command::new(&self.0);
        cmd.args(&["new", "--lib", name]);
        cmd.output().expect("unable run cargo new");

        let mut path = PathBuf::from(name);

        let cargo_toml = path.join("Cargo.toml");
        // Just ran cargo new so will always exist
        let ct = std::fs::read_to_string(&cargo_toml).unwrap();
        // TODO: What if you run new more than once... quick fix ;-)
        if !ct.contains(r#"crate-type = ["cdylib", "rlib"]"#) {
            std::fs::write(
                cargo_toml,
                ct.replace("[dependencies]", templates::DEPENDENCIES),
            )
            .unwrap();
        }

        let lib = path.join("src/lib.rs");
        std::fs::write(lib, templates::LIB_RS).unwrap();

        let gitignore = path.join(".gitignore");
        std::fs::write(gitignore, templates::GITIGNORE).unwrap();

        path.push("dist");
        std::fs::create_dir_all(&path).unwrap();

        path.push("index.html");
        std::fs::write(path, templates::make_html(name, bundler)).unwrap();
    }
}

#[derive(Clone)]
struct PackageInfo {
    package: Package,
    wasm_bindgen_version: String,
}

impl PackageInfo {
    fn get_package_name(&self) -> String {
        self.package.name.replace("-", "_")
    }

    /// Only return a package if it uses wasm-bindgen
    fn new(metadata: &Metadata, package: Package) -> Option<Self> {
        metadata
            .packages
            .iter()
            .find_map(|d| {
                if d.name == WASM_BINDGEN {
                    Some(d.version.to_string())
                } else {
                    None
                }
            })
            .map(|wasm_bindgen_version| Self {
                package,
                wasm_bindgen_version,
            })
    }

    fn build_wasm_js(&self, opt: &Opt) {
        let mut cmd = Command::new(path_to_cli(&self.wasm_bindgen_version));
        let source_wasm = format!(
            "./target/{}/{}/{}.wasm",
            WASM32_UNKNOWN_UNKNOWN,
            if opt.release { "release" } else { "debug" },
            self.get_package_name()
        );
        cmd.arg(source_wasm);

        let target = opt
            .target
            .as_ref()
            .map(|x| x.as_ref())
            .unwrap_or_else(|| "web");

        cmd.args(&["--target", target]);

        if !opt.typescript {
            cmd.arg("--no-typescript");
        }

        if opt.weak_refs {
            cmd.arg("--weak-refs");
        }

        if opt.reference_types {
            cmd.arg("--reference-types");
        }

        if opt.no_demangle {
            cmd.arg("--no-demangle");
        }

        let mut out_wasm = PathBuf::new();
        out_wasm.push(opt.out_dir.as_deref().unwrap_or_else(|| OUT_DIR));
        cmd.args(&["--out-dir", &out_wasm.display().to_string()]);

        println!("Building js glue code for {}", self.get_package_name(),);

        let output = cmd.output().expect("unable to build js glue code");
        std::io::stdout().write_all(&output.stdout).unwrap();
        std::io::stderr().write_all(&output.stderr).unwrap();

        out_wasm.push(format!("{}_bg.wasm", self.get_package_name()));
        if let Some(SubCmds::WASM_OPT(wasm_opt)) = &opt.subs {
            if let Err(e) = wasm_opt.try_install_and_run(&out_wasm, opt) {
                eprint!("Error running wasm-opt: {}", e)
            }
        }
    }
}

#[derive(Clone)]
struct BindgenPackages {
    packages: Vec<PackageInfo>,
    cargo: Cargo,
}

impl BindgenPackages {
    fn new(cargo: Cargo) -> Result<Self, Error> {
        let cmd = cargo_metadata::MetadataCommand::new();
        let metadata = cmd.exec()?;
        let mut packages = Vec::new();
        for package in &metadata.packages {
            if metadata.workspace_members.contains(&package.id) {
                if let Some(p) = PackageInfo::new(&metadata, package.clone()) {
                    packages.push(p)
                }
            }
        }
        Ok(BindgenPackages { packages, cargo })
    }

    fn build_wasm32_unknown_unknown(&self, opt: &Opt) {
        for p in &self.packages {
            self.cargo
                .build_wasm32_unknown_unknown(&p.get_package_name(), opt)
        }
    }

    // TODO: Move this to the global cargo store?
    // TODO: Cargo will fail if they are different...what should approach be?
    // Could download instead...?
    fn install_wasm_bindgen_cli(&self) {
        let bindgen: BTreeSet<&str> = self
            .packages
            .iter()
            .map(|p| p.wasm_bindgen_version.as_str())
            .collect();
        for bg in bindgen {
            self.cargo.install_wasm_bindgen_cli(bg)
        }
    }

    fn build_wasm_js(&self, opt: &Opt) {
        // Must only clean once
        let mut out_dir = PathBuf::new();
        out_dir.push(opt.out_dir.as_deref().unwrap_or_else(|| OUT_DIR));
        if opt.clean {
            println!("Cleaning out-dir: {}", &out_dir.display());
            std::fs::remove_dir_all(&out_dir).unwrap_or(());
        }
        for p in &self.packages {
            p.build_wasm_js(opt)
        }
        // TODO: Here should look for a rollup.config.js file & run it?
        if let Some(WasmTarget::Rollup) = opt.target {
            let bootstrap = templates::rollup_bootstrap_js(self.packages.as_slice(), &out_dir);
            out_dir.push("bootstrap.js");
            std::fs::write(out_dir, bootstrap).unwrap()
        }
    }
}

// TODO: Need to add package.json for node / deno?
#[derive(StructOpt)]
enum WasmTarget {
    Web,
    Rollup,
    // NoModules,
    /// For use with
    Webpack,
    // Nodejs,
    // Deno,
}

impl FromStr for WasmTarget {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "web" => Ok(WasmTarget::Web),
            "webpack" => Ok(WasmTarget::Webpack),
            "rollup" => Ok(WasmTarget::Rollup),
            // "no-modules" => Ok(WasmTarget::NoModules),
            // "nodejs" => Ok(WasmTarget::Nodejs),
            // "deno" => Ok(WasmTarget::Deno),
            _ => Err(format!(
                "'{}' is not an allowed target. Supported options are: web (default), webpack", //, nodejs, deno",
                s
            )),
        }
    }
}

impl AsRef<str> for WasmTarget {
    fn as_ref(&self) -> &str {
        match self {
            WasmTarget::Web => "web",
            WasmTarget::Rollup => "web",
            // WasmTarget::NoModules => "no-modules",
            WasmTarget::Webpack => "bundler",
            // WasmTarget::Nodejs => "nodejs",
            // WasmTarget::Deno => "deno",
        }
    }
}

// TODO: Look at debug options: should '--debug' be the default when not release?
// TODO: Add in all cli options

#[derive(StructOpt, Default)]
struct Opt {
    /// Compile in release mode
    #[structopt(long, short)]
    release: bool,

    /// Generate typescript files
    #[structopt(long, short)]
    typescript: bool,

    /// Target to compile the js glue code to: web (default), webpack
    #[structopt(long)]
    target: Option<WasmTarget>,

    /// Default of "./dist/js"
    #[structopt(long)]
    out_dir: Option<String>,

    #[structopt(long, short)]
    /// Run remove_dir_all on out-dir
    clean: bool,

    // https://rustwasm.github.io/docs/wasm-bindgen/reference/cli.html#--weak-refs
    /// Enables usage of the TC39 Weak References proposal, ensuring that all Rust memory is eventually deallocated
    /// regardless of whether you're calling free or not. This is off-by-default while we're waiting for support
    /// to percolate into all major browsers. For more information see the documentation about weak references.
    #[structopt(long)]
    weak_refs: bool,

    // https://rustwasm.github.io/docs/wasm-bindgen/reference/cli.html#--reference-types
    /// Enables usage of the WebAssembly References Types proposal proposal, meaning that the WebAssembly binary
    /// will use externref when importing and exporting functions that work with JsValue. For more information see
    /// the documentation about reference types. Off by default.
    #[structopt(long)]
    reference_types: bool,

    // https://rustwasm.github.io/docs/wasm-bindgen/reference/cli.html#--no-demangle
    /// When post-processing the .wasm binary, do not demangle Rust symbols in the "names" custom section.
    #[structopt(long)]
    no_demangle: bool,

    #[structopt(subcommand)]
    subs: Option<SubCmds>,
}

// TODO: Is giant struct a good idea... or should just use a vec? Use a macro to parse the wasm-opt.txt file...

#[derive(StructOpt, Debug)]
#[allow(non_camel_case_types)]
enum SubCmds {
    /// Runs wasm-opt https://github.com/WebAssembly/binaryen#wasm-opt.
    /// Its runs as `wasm-opt path_to_wasm --o path_to_wasm [FLAGS]`
    /// and uses a default optimization of '--O'.
    /// It will pass through any other relevant flags that are parsed to wasm build: --enable-reference-types
    WASM_OPT(WasmOpt),
}
impl Default for SubCmds {
    fn default() -> Self {
        Self::WASM_OPT(WasmOpt::default())
    }
}

// TODO: Static version of rollup.js for packaging?
// TODO: Sort logging / verbose
// TODO: Normalise how paths to files / folders are done...
#[derive(StructOpt)]
enum CargoWasm {
    /// Compile your project to wasm and generate js glue code
    Build(Opt),

    // TODO: Bundler option
    /// Create a template project for loading in the browser
    New { name: String },
    // TODO
    // Run,
    // Test
}

impl CargoWasm {
    fn run(&self, cargo: Cargo) {
        match self {
            CargoWasm::Build(opt) => match BindgenPackages::new(cargo) {
                Ok(bp) => self.build(bp, opt),
                Err(e) => {
                    eprintln!("Unable to begin running cargo-wasm:");
                    eprintln!("{}", e);
                }
            },
            //TODO: Bundler option
            CargoWasm::New { name } => cargo.new_template_project(name, false),
        }
    }

    fn build(&self, bindgen_packages: BindgenPackages, opt: &Opt) {
        let bindgen_packages = Arc::new(bindgen_packages);
        let bp = Arc::clone(&bindgen_packages);
        // TODO: Is this worth it?
        let handler = std::thread::spawn(move || {
            bp.install_wasm_bindgen_cli();
        });
        bindgen_packages.build_wasm32_unknown_unknown(opt);
        handler.join().unwrap();
        bindgen_packages.build_wasm_js(opt);
    }
}

fn main() {
    let mut args = std::env::args().into_iter();
    // Need to skip one arg: .. /.cargo/bin/cargo-wasm for structopt to work?
    args.next();
    let cargo_wasm = CargoWasm::from_iter(args);
    match std::env::var("CARGO") {
        Ok(cargo) => cargo_wasm.run(Cargo(cargo)),
        Err(e) => eprintln!("{}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installs_correct_bindgen_version() {
        let crates = ".crates.toml";

        let v_1 = "0.2.68";
        let cargo_1 = Cargo(std::env::var("CARGO").unwrap());
        let mut path_1 = PathBuf::from("target/wasm-bindgen-cli");
        path_1.push(v_1);
        path_1.push(crates);

        let v_2 = "0.2.67";
        let cargo_2 = cargo_1.clone();
        let mut path_2 = PathBuf::from("target/wasm-bindgen-cli");
        path_2.push(v_2);
        path_2.push(crates);

        let handler = std::thread::spawn(move || {
            cargo_1.install_wasm_bindgen_cli("0.2.68");
            let file_1 = std::fs::read_to_string(path_1).unwrap();
            assert!(file_1.contains(v_1))
        });

        cargo_2.install_wasm_bindgen_cli("0.2.67");
        let file_2 = std::fs::read_to_string(path_2).unwrap();
        assert!(file_2.contains(v_2));
        handler.join().unwrap();
    }
}
