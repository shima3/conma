// The transpiled code of vm/conma2rust (src/transpiled.rs): CONMA_TRANSPILED names the Rust
// file made by vm/conma2rust/conma2rust.py; without it, the VM has no transpiled code.
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=CONMA_TRANSPILED");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("transpiled_gen.rs");
    match env::var("CONMA_TRANSPILED") {
        Ok(path) if !path.is_empty() => {
            println!("cargo:rerun-if-changed={}", path);
            fs::copy(&path, &out).unwrap_or_else(|e| panic!("CONMA_TRANSPILED: {}: {}", path, e));
        }
        _ => fs::write(&out, "pub static MODULES: &[Module] = &[];\n").unwrap(),
    }
}
