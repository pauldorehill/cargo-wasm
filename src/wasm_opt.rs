use crate::Opt;
use flate2::read::GzDecoder;
use log::{error, info, trace};
use std::{error::Error, fmt::Display, path::Path, process::Command};
use structopt::StructOpt;
use tar::Archive;
// Fixed version for consistent builds
// 97 has linux, windows & macos: only x84_64
const BINDGEN_VERSION: &'static str = "version_97";
const OUT_DIR: &'static str = "target/binaryen";
const FINAL_PATH: &'static str = "target/binaryen/binaryen-version_97/bin/wasm-opt";
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

impl WasmOpt {
    pub(crate) fn try_install(&self) -> Result<(), Box<dyn Error>> {
        if !Path::new(FINAL_PATH).exists() {
            let platform = Platform::try_new()?;
            let name = format!(
                "binaryen-{}-{}-{}.tar",
                BINDGEN_VERSION, ARCH_X86_64, platform
            );
            let url = format!(
                "http://github.com/WebAssembly/binaryen/releases/download/{}/{}.gz",
                BINDGEN_VERSION, name
            );

            info!("Trying to download wasm-opt from: {}", url);
            let client = reqwest::blocking::Client::new();
            // TODO: How many retries?
            let data = match client.get(&url).send().and_then(|r| r.bytes()) {
                Ok(data) => data,
                Err(e) => {
                    error!("First request failed with: '{}'. Retrying...", e);
                    client.get(&url).send()?.bytes()?
                }
            };

            let decompressed = GzDecoder::new(&*data);
            let mut archive = Archive::new(decompressed);
            // TODO: Just get wasm-opt?
            archive.unpack(OUT_DIR)?;
            info!("wasm-opt installed at: {}", FINAL_PATH);
            Ok(())
        } else {
            info!("wasm-opt already installed");
            Ok(())
        }
    }

    fn file_size(raw_size: u64) -> String {
        let kb = 1024;
        let mb = 1_048_576;
        if raw_size == 1 {
            String::from("1 byte")
        } else if raw_size < kb {
            format!("{} bytes", raw_size)
        } else if raw_size < mb {
            format!("{:.2} KiB", raw_size as f64 / kb as f64)
        } else {
            format!("{:.2} MB", raw_size as f64 / mb as f64)
        }
    }
    // TODO: What should the defaults be? What should release trigger?
    // bin/wasm-opt [.wasm or .wat file] [options] [passes]
    pub(crate) fn try_run(&self, wasm: &Path, opt: &Opt) -> Result<(), Box<dyn Error>> {
        let mut cmd = Command::new(FINAL_PATH);
        let wasm_file = std::fs::File::open(wasm)?;

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
        if self.O || !set {
            trace!("wasm-opt using default optimization passes");
            cmd.arg("-O");
        }

        let original_file_size = wasm_file.metadata()?.len();
        let run = crate::run_command(cmd, opt.quiet);

        match run {
            Ok(_) => {
                let final_file_size = wasm_file.metadata()?.len();
                info!(
                    "Ran wasm-opt for {}
Orignal size: {}
  Final size: {}
   Reduction: {:.1} % [{}]",
                    wasm.display(),
                    Self::file_size(original_file_size),
                    Self::file_size(final_file_size),
                    ((original_file_size - final_file_size) as f64 / original_file_size as f64)
                        * 100f64,
                    Self::file_size(original_file_size - final_file_size),
                );
            }
            Err(e) => error!(
                "Failed to run wasm-opt for {}
{}",
                wasm.display(),
                e
            ),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn download_and_run() {
        std::fs::remove_dir_all(OUT_DIR).unwrap_or(());
        let opts = Opt::default();
        let wasm_opt = WasmOpt::default();
        let test_wasm = Path::new("test_crates/test.wasm");
        let template = Path::new("test_crates/_test.wasm");
        std::fs::copy(template, test_wasm).unwrap();
        let template = std::fs::File::open(template).unwrap();
        let test_wasm_file = std::fs::File::open(test_wasm).unwrap();
        wasm_opt.try_install().unwrap();
        wasm_opt.try_run(test_wasm, &opts).unwrap();
        // Check smaller
        assert!(test_wasm_file.metadata().unwrap().len() < template.metadata().unwrap().len());
        std::fs::remove_file(test_wasm).unwrap();
    }

    #[test]
    fn check_file_size() {
        assert_eq!(WasmOpt::file_size(0), "0 bytes");
        assert_eq!(WasmOpt::file_size(1), "1 byte");
        assert_eq!(WasmOpt::file_size(1023), "1023 bytes");
        assert_eq!(WasmOpt::file_size(1024), "1.00 KiB");
        assert_eq!(WasmOpt::file_size(1536), "1.50 KiB");
        assert_eq!(WasmOpt::file_size(2048), "2.00 KiB");
        assert_eq!(WasmOpt::file_size(1_048_576), "1.00 MB");
        assert_eq!(WasmOpt::file_size(1_572_864), "1.50 MB");
    }
}
