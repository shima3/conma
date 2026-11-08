// Run the ConMa VM for wasm32 in Node.js, as conma (vm/src/main.rs) runs:
//   node vm-node.js [-t] [-m] <module_file> <func_name | arg ...>
// vm-wasm.sh uses it when node is found. The VM starts in the current directory, with the environment
// variables of the process, and sees the real file system through its virtual one: a file is read when
// the VM opens it, and a directory is listed when the VM lists it (host_file_needed, host_dir_needed).
// What the VM writes to files stays in memory. Standard input is read when a VProc waits for it.
"use strict";
const fs = require("fs");
const path = require("path");
const ConMaVM = require("./conma.js");

const argv = process.argv.slice(2);
const wasmFile = path.join(__dirname, "wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm");

// Wait for an fd that is not blocking (EAGAIN).
const sleeper = new Int32Array(new SharedArrayBuffer(4));
function retry(f) {
  for (;;) {
    try {
      return f();
    } catch (e) {
      if (e.code !== "EAGAIN") throw e;
      Atomics.wait(sleeper, 0, 0, 5);
    }
  }
}

function writeAll(fd, bytes) {
  let off = 0;
  while (off < bytes.length) {
    try {
      off += retry(() => fs.writeSync(fd, bytes, off, bytes.length - off));
    } catch (e) {
      if (e.code === "EPIPE") process.exit(141); // as the native VM after SIGPIPE
      throw e;
    }
  }
}

let inputEnded = false;
const inbuf = Buffer.alloc(65536);
// Give the next piece of standard input (waiting for it), or its end.
function readInput(vm) {
  if (inputEnded) return;
  let n;
  try {
    n = retry(() => fs.readSync(0, inbuf, 0, inbuf.length, null));
  } catch (e) {
    n = 0; // EOF, a closed or missing standard input
  }
  if (n > 0) {
    vm.input(inbuf.subarray(0, n));
  } else {
    vm.inputEnd();
    inputEnded = true;
  }
}

function fetchFile(vm, p) {
  try {
    const st = fs.statSync(p);
    if (st.isDirectory()) vm.makeDirs(p);
    else vm.putFile(p, fs.readFileSync(p));
  } catch (e) {
    // not in the real file system either
  }
}

function listDir(vm, p) {
  let entries;
  try {
    entries = fs.readdirSync(p, { withFileTypes: true });
  } catch (e) {
    return;
  }
  for (const e of entries) {
    const q = path.join(p, e.name);
    let dir = e.isDirectory();
    if (e.isSymbolicLink()) {
      try {
        dir = fs.statSync(q).isDirectory();
      } catch (err) {
        dir = false;
      }
    }
    if (dir) vm.makeDirs(q);
    else vm.putUnread(q);
  }
}

function main() {
  const wasm = new WebAssembly.Module(fs.readFileSync(wasmFile));
  const vm = ConMaVM.create(wasm, writeAll, readInput, fetchFile, listDir);
  const cwd = process.cwd();
  vm.makeDirs(cwd);
  vm.setCwd(cwd);
  for (const [k, v] of Object.entries(process.env)) vm.setenv(k, v);
  // The module file named in argv, read here as the native VM reads it.
  let i = 0;
  while (i < argv.length && ["-t", "--trace", "-m", "--main"].includes(argv[i])) i++;
  let module = new Uint8Array(0);
  if (i < argv.length) {
    try {
      module = fs.readFileSync(argv[i]);
    } catch (e) {
      writeAll(2, ConMaVM.utf8("Error: [Errno 2] No such file or directory: '" + argv[i] + "'\n"));
      return 1;
    }
  }
  let code = vm.start(module, argv);
  if (code !== 0) return code;
  for (;;) {
    code = vm.run(100000);
    // A program may poll standard input instead of waiting for it: give it input when it found none.
    if (code === ConMaVM.RUNNING && !(vm.wantsInput() && !inputEnded)) continue;
    if (code !== ConMaVM.RUNNING && code !== ConMaVM.IDLE) return code;
    if (!inputEnded) {
      readInput(vm);
      continue;
    }
    if (code === ConMaVM.IDLE) {
      // Input has ended and there are no processes: nothing can wake the VM
      // (the native VM would wait for ever).
      writeAll(2, ConMaVM.utf8("vm-node: every VProc waits, and nothing can wake them\n"));
      return 2;
    }
  }
}

process.exitCode = main();
