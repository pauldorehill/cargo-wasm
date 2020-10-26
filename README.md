# cargo-wasm 🍯 🦡
A simple cargo subcommand that wraps the `wasm-bindgen-cli` and syncs it with the `wasm-bindgen` version. You might not need this 😆 as all the heavy lifting is done by `wasm-bindgen`... but it can make life a little easier.

Its a work in progress so I will break things, make mistakes, and could abandon it... but it works (for me). I'm always learning - so please feel free to teach me where I've gone wrong & could do better.

Not on crates.io yet, but you can install from github:
```
cargo install --git https://github.com/pauldorehill/cargo-wasm
```
There are currently two commands (add an `-h` arg for more info):

`cargo wasm new`

`cargo wasm build`

Works for both single crates & workspaces. When using with workspaces you will get a single directory at the workspace root containing all the wasm and js glue code. Note it currently installs the `wasm-bindgen-cli` using cargo & locally to the crate... so first run can take a bit longer.

## TODO
- Add `run` & `test` commands
- Bundler stuff
- node / deno?
- Write some tests
- Sort out logging / stdout from `Command`

## Notes

### Install a local version from source
```
cargo install --path ./
```

### wasm-bindgen-cli
https://rustwasm.github.io/docs/wasm-bindgen/reference/cli.html
```
cargo install --version 0.2.68 -- wasm-bindgen-cli
```

Make sure `wasm-bindgen` in the crate and `wasm-bindgen-cli` are the same.
To use:
```
cargo build --lib --target wasm32-unknown-unknown
wasm-bindgen ./target/wasm32-unknown-unknown/debug/package_name.wasm --out-dir ./dist/js
```

### wasm-bindgen
https://github.com/rustwasm/wasm-bindgen/releases

### wasm-opt
Releases are here:
https://github.com/WebAssembly/binaryen/releases
These are quite large: best to download on order?

### Rollup

[Rollup](https://rollupjs.org/guide/en/)
[Rollup plugin](https://github.com/wasm-tool/rollup-plugin-rust/blob/master/index.js)

