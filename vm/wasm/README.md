# ConMa VM for WebAssembly

The Rust VM of `vm/src` built for `wasm32-unknown-unknown`, with the JavaScript that runs it (the host). It runs ConMa modules in Node.js, in the JavaScriptCore shell of macOS and in a browser; ConSh in a browser ([consh/js](../../consh/js/README-ja.md)) is built on it. Its host has JavaScript, so `__JS_call__` and the WebAssembly of SJS (`__Wasm_compile__`, `__Wasm_call__`) work here, unlike in the native VMs.

```
rustup target add wasm32-unknown-unknown   # once
vm/wasm/build.sh
CONMA_VM=wasm test/run_all.sh -j 3
```

* `Cargo.toml`, `src/lib.rs` — the crate `conma-wasm` (a `cdylib`). It uses the files of `vm/src` as they are (`#[path]`), except `sys`, which `src/sys.rs` replaces with a virtual OS in memory and calls of the host. `build.rs` takes the transpiled code (`CONMA_TRANSPILED`) as `vm/build.rs` does.
* `build.sh` — build it (`target/wasm32-unknown-unknown/release/conma_wasm.wasm`). `consh/js/build.sh` calls it and copies the wasm and `conma.js` into the page of ConSh.
* `conma.js` — the host: it loads the wasm, gives it the files, processes, JavaScript and WebAssembly of Node.js, JavaScriptCore or a browser, and runs the scheduler in slices. Some of it serves ConSh only (the JS values of shell variables, `__consh_value`).
* `vm-wasm.sh` — run a module as `vm/target/release/conma` does, with the same arguments, in Node.js (`vm-node.js`) or, without `node` or with `CONMA_WASM_HOST=jsc`, in JavaScriptCore (`vm-jsc.js`). `bin/vm` uses it when `CONMA_VM=wasm`.

The design, and how it was built step by step, is in docs/consh/dev/JS-PLAN-ja.md (in Japanese).
