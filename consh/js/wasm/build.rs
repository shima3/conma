// The VM for wasm32 has no transpiled code (src/transpiled.rs of vm/ includes this file).
use std::{env, fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("transpiled_gen.rs");
    fs::write(&out, "pub static MODULES: &[Module] = &[];\n").unwrap();
}
