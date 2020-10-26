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

pub(crate) fn make_html(project_name: &str, bundler: bool) -> String {
    let script = if bundler {
        format!(r#"<script src="./js/{}.js"></script>"#, project_name)
    } else {
        format!(
            r#"<script type="module">
    import init from './js/{}.js';
    init();
</script>"#,
            project_name
        )
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

pub(crate) const GITIGNORE: &str = r#"/target
Cargo.lock
/dist/js"#;
