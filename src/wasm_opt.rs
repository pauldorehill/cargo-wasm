use crate::Opt;
use flate2::read::GzDecoder;
use log::info;
use std::{fmt::Display, path::Path, process::Command};
use structopt::StructOpt;
use tar::Archive;

// Fixed version for consistent builds
// 97 has linux, windows & macos: only x84_64
const BINDGEN_VERSION: &'static str = "version_97";
const OUT_DIR: &'static str = "./target/wasm-opt";
const FINAL_PATH: &'static str = "./target/wasm-opt/binaryen-version_97/bin/wasm-opt";
const ARCH_X86_64: &'static str = "x86_64";

// TODO: Is restricting this to x84_64 correct?
enum Platform {
    Linux,
    Windows,
    MacOS,
}

impl Platform {
    fn try_new() -> Result<Self, String> {
        if std::env::consts::ARCH == ARCH_X86_64 {
            match std::env::consts::OS {
                "linux" => Ok(Self::Linux),
                "macos" => Ok(Self::MacOS),
                "windows" => Ok(Self::Windows),
                _ => Err("Unsupported platform".to_string()),
            }
        } else {
            Err("Prebuilt wasm-opt binaries are only available for x86_64".to_string())
        }
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Linux => f.write_str("linux"),
            Platform::Windows => f.write_str("windows"),
            Platform::MacOS => f.write_str("macos"),
        }
    }
}

// TODO: Have to use long even due to clap always making `short` a single char.. file issue?
// TODO: a macro that will generate this from the souce file file://./../wasm-opt.txt
#[derive(StructOpt, Debug, Default)]
#[allow(non_snake_case)]
pub(crate) struct WasmOpt {
    /// execute default optimization passes
    #[structopt(long = "O")]
    O: bool,
    /// execute no optimization passes
    #[structopt(long = "O0")]
    O0: bool,
    /// execute -O1 optimization passes (quick&useful opts, useful for iteration builds)
    #[structopt(long = "O1")]
    O1: bool,
    /// execute -O2 optimization passes (most opts, generally gets most perf)
    #[structopt(long = "O2")]
    O2: bool,
    /// execute -O3 optimization passes (spends potentially a lot of time optimizing)
    #[structopt(long = "O3")]
    O3: bool,
    /// execute -O4 optimization passes (also flatten the IR, which can take a lot more time and memory, but is useful on more nested / complex / less-optimized input)
    #[structopt(long = "O4")]
    O4: bool,
    /// execute default optimization passes, focusing on code size
    #[structopt(long = "Os")]
    Os: bool,
    /// execute default optimization passes, super-focusing on code size
    #[structopt(long = "Oz")]
    Oz: bool,
}

// TODO: Do something better with errors
impl WasmOpt {
    fn try_install() -> Result<(), String> {
        if !Path::new(FINAL_PATH).exists() {
            let platform = Platform::try_new()?;
            let name = format!(
                "binaryen-{}-{}-{}.tar",
                BINDGEN_VERSION, ARCH_X86_64, platform
            );
            let url = format!(
                "https://github.com/WebAssembly/binaryen/releases/download/{}/{}.gz",
                BINDGEN_VERSION, name
            );
            info!("Getting wasm-opt from: {}", url);

            let rep = reqwest::blocking::get(&url).map_err(|e| e.to_string())?;
            let data = rep.bytes().map_err(|e| e.to_string())?;
            let decompressed = GzDecoder::new(&*data);
            let mut archive = Archive::new(decompressed);
            // TODO: Just get wasm-opt?
            archive.unpack(OUT_DIR).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Ok(())
        }
    }

    // TODO: What should the defaults be? What should release trigger?
    // TODO: Better size output logging
    // bin/wasm-opt [.wasm or .wat file] [options] [passes]
    pub fn try_run(&self, wasm: &Path, opt: &Opt) -> Result<(), String> {
        info!("Running wasm-opt for {}", wasm.display());
        let mut cmd = Command::new(FINAL_PATH);
        let wasm_file = std::fs::File::open(wasm).map_err(|e| e.to_string())?;
        let original_file_size = wasm_file.metadata().map_err(|e| e.to_string())?.len();
        info!("Source wasm size: {} bytes", original_file_size);

        // [WASM File]
        cmd.arg(wasm);

        // [OPTIONS]
        cmd.args(&["--output", &wasm.display().to_string()]);

        if opt.reference_types {
            cmd.arg("--enable-reference-types");
        }

        // [PASSES]
        let mut set = false;
        if self.O0 {
            cmd.arg("-O0");
            set = true;
        }
        if self.O1 {
            cmd.arg("-O1");
            set = true;
        }
        if self.O2 {
            cmd.arg("-O2");
            set = true;
        }
        if self.O3 {
            cmd.arg("-O3");
            set = true;
        }
        if self.O4 {
            cmd.arg("-O4");
            set = true;
        }
        if self.Os {
            cmd.arg("-Os");
            set = true;
        }
        if self.Oz {
            cmd.arg("-Oz");
            set = true;
        }
        // Set this as the default
        if self.O || !set {
            cmd.arg("-O");
        }

        cmd.status().expect("failed to run wasm-opt");

        let final_file_size = wasm_file.metadata().map_err(|e| e.to_string())?.len();
        info!("Final wasm size: {} bytes", final_file_size);

        info!(
            "Size reduction: {:.1} %",
            final_file_size as f64 / original_file_size as f64 * 100f64
        );

        Ok(())
    }

    pub(crate) fn try_install_and_run(&self, path_to_wasm: &Path, opt: &Opt) -> Result<(), String> {
        Self::try_install()?;
        self.try_run(path_to_wasm, opt)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn download_and_run() {
        std::fs::remove_dir_all("target/wasm-opt").unwrap_or(());
        let opts = Opt::default();
        let wasm_opt = WasmOpt::default();
        let test_wasm = Path::new("test_crates/test.wasm");
        let template = Path::new("test_crates/_test.wasm");
        std::fs::copy(template, test_wasm).unwrap();
        let template = std::fs::File::open(template).unwrap();
        let test_wasm_file = std::fs::File::open(test_wasm).unwrap();
        wasm_opt.try_install_and_run(test_wasm, &opts).unwrap();
        // Check smaller
        assert!(test_wasm_file.metadata().unwrap().len() < template.metadata().unwrap().len());
        std::fs::remove_file(test_wasm).unwrap();
    }
}
