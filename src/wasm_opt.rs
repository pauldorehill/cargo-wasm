use crate::Opt;
use flate2::read::GzDecoder;
use std::{fmt::Display, path::Path, process::Command};
use tar::Archive;

// TODO: Should this be fixed? Easier for consistient builds etc.
// 97 has linux, windows & macos
// It looks like different versions can have different paths eg. a bin folder or not..
pub(crate) struct WasmOpt;

impl WasmOpt {
    const BINDGEN_VERSION: &'static str = "version_97";
    const OUT_DIR: &'static str = "./target/wasm-opt";
    const FINAL_PATH: &'static str = "./target/wasm-opt/binaryen-version_97/bin/wasm-opt";

    pub(crate) fn new() -> Option<WasmOpt> {
        if !Path::new(Self::FINAL_PATH).exists() {
            let platform = Platform::new()?;
            let name = format!("binaryen-{}-x86_64-{}.tar", Self::BINDGEN_VERSION, platform);
            let url = format!(
                "https://github.com/WebAssembly/binaryen/releases/download/{}/{}.gz",
                Self::BINDGEN_VERSION,
                name
            );

            // TODO: Actually handle errors
            let rep = reqwest::blocking::get(&url).ok()?;
            let data = rep.bytes().unwrap();
            let decompresed = GzDecoder::new(&*data);
            let mut archive = Archive::new(decompresed);
            // TODO: Just get wasm-opt?
            archive.unpack(Self::OUT_DIR).unwrap();
            Some(WasmOpt)
        } else {
            Some(WasmOpt)
        }
    }

    // TODO: What should the defaults be? What should release trigger?
    // bin/wasm-opt [.wasm or .wat file] [options] [passes]
    pub(crate) fn run(&self, path_to_wasm: &str, opt: &Opt) {
        let mut cmd = Command::new(Self::FINAL_PATH);

        // WASM File
        cmd.arg(path_to_wasm);

        // OPTIONS
        // Output
        cmd.args(&["-o", path_to_wasm]);
        if opt.reference_types {
            cmd.arg("--enable-reference-types");
        }

        // PARSES: use standard for now
        cmd.arg("-O");

        let _ = cmd.output().expect("failed to run wasm-opt");
        // std::io::stdout().write_all(&output.stdout).unwrap();
        // std::io::stderr().write_all(&output.stderr).unwrap();
    }
}

// TODO: need to care about arch eg. x84_64?
enum Platform {
    Linux,
    Windows,
    MacOS,
}

impl Platform {
    fn new() -> Option<Self> {
        match std::env::consts::OS {
            "linux" => Some(Self::Linux),
            "macos" => Some(Self::MacOS),
            "windows" => Some(Self::Windows),
            _ => None,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn download_and_run() {
        std::fs::remove_dir_all("target/wasm-opt").unwrap_or(());
        let wasm_opt = WasmOpt::new().unwrap();
        let test_wasm = "test_crates/test.wasm";
        let template = "test_crates/_test.wasm";
        std::fs::copy(template, test_wasm).unwrap();
        wasm_opt.run(test_wasm, &Opt::default());
        let template = std::fs::File::open(template).unwrap();
        let test_wasm_file = std::fs::File::open(test_wasm).unwrap();
        // Check smaller
        assert!(test_wasm_file.metadata().unwrap().len() < template.metadata().unwrap().len());
        std::fs::remove_file(test_wasm).unwrap();
    }
}
