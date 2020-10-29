use crate::{PackageInfo, WasmTarget};
use std::fmt::Write;
use std::path::{Path, PathBuf};

pub(crate) const ROLLUP_TEMPLATE: &str = include_str!("../templates/rollup/rollup.config.js");
pub(crate) const ROLLUP_PACKAGE_JSON: &str = include_str!("../templates/rollup/package.json");
pub(crate) const NODE_GITIGNORE: &str = include_str!("../templates/rollup/.gitignore");
pub(crate) const GITIGNORE: &str = include_str!("../templates/web/.gitignore");

pub(crate) const LIB_RS: &str = r#"use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    web_sys::console::log_1(&"rust says hello from wasm".into());
}"#;

pub(crate) const DEPENDENCIES: &str = r#"[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["console"] }"#;

pub(crate) fn make_html(project_name: &str, target: &WasmTarget) -> String {
    let script = match target {
        WasmTarget::Web => format!(
            r#"<script type="module">
        import init from './js/{}.js';
        init();
    </script>"#,
            project_name
        ),
        WasmTarget::Rollup | WasmTarget::Webpack => {
            format!(r#"<script src="js/index.js"></script>"#)
        }
    };

    format!(
        r#"<!DOCTYPE html>
<html>

<head>
    <meta charset="utf-8">
</head>

<body>
<div>Hello from rust + wasm. Check the browser console.</div>
</body>
{}
</html>"#,
        script
    )
}

pub(crate) fn rollup_bootstrap_js(names: &[PackageInfo], out_dir: &Path) -> String {
    let mut output = String::new();
    for n in names {
        writeln!(
            output,
            r#"import * as {0} from "./{0}.js""#,
            n.get_package_name()
        )
        .unwrap()
    }
    for n in names {
        let mut path = PathBuf::new();
        if let Some(p) = out_dir.file_name() {
            path.push(p)
        }
        let name = n.get_package_name();
        path.push(&format!("{}_bg.wasm", name));
        writeln!(
            output,
            r#"{}.default("{}").catch((e) => {{ console.log("Failed to load wasm file: {1}") }})"#,
            name,
            path.display()
        )
        .unwrap()
    }
    output
}
