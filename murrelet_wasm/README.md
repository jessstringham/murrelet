


(drafting a README so i don't forget...)

Create a new package and add this as a dependency.

You'll need two scripts:

in `src/bin/murrelet_ship_js.rs` you'll want to add

```
use std::{fs, path::PathBuf};

fn main() {
    let pkg_dir: PathBuf = std::env::args().nth(1).expect("pkg dir").into();

    // shared
    fs::write(pkg_dir.join("murrelet_client.js"), murrelet_wasm::MURRELET_CLIENT_JS).unwrap();

    // per-crate wrapper
    let crate_name = env!("CARGO_PKG_NAME");
    let ident = murrelet_wasm::js_ident_from_pkg_name(crate_name);
    let js = murrelet_wasm::per_crate_client_js(&format!("./{}.js", crate_name), &ident);
    fs::write(pkg_dir.join(format!("{ident}_client.js")), js).unwrap();
}
```
(which generates some more javascript wrappers)

and then when you package, you'll want to run this at least once to copy over the bonus boilerplate javascript. since wasm-pack is going to overwrite pkg, I just have it at the end of the build step.

```
#!/usr/bin/env bash
set -euo pipefail

wasm-pack build --dev --target web && cargo run --bin murrelet_ship_js -- pkg
```


run something like ln -s ../../rust/spoonbill/pkg spoonbill to link it together