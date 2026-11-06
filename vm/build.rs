// The native code of vm/native (src/native.rs): CONMA_NATIVE names the Rust
// file made by vm/native/translate.py; without it, the VM has no native code.
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=CONMA_NATIVE");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("native_gen.rs");
    match env::var("CONMA_NATIVE") {
        Ok(path) if !path.is_empty() => {
            println!("cargo:rerun-if-changed={}", path);
            fs::copy(&path, &out).unwrap_or_else(|e| panic!("CONMA_NATIVE: {}: {}", path, e));
        }
        _ => fs::write(&out, "pub static MODULES: &[Module] = &[];\n").unwrap(),
    }
}
