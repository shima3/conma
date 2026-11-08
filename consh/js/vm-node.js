// Run the ConMa VM for wasm32 in Node.js, as conma (vm/src/main.rs) runs:
//   node vm-node.js [-t] [-m] <module_file> <func_name | arg ...>
// vm-wasm.sh uses it when node is found. The VM starts in the current directory, with the environment
// variables of the process, and uses the files of this process (hostFs, conma_use_host_fs): it reads
// and writes them as the native VM does. Standard input is read when a VProc waits for it.
// The builtin js of ConSh (__JS_call__) evaluates JavaScript in this process (conma.js).
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

// The file system of this process, for the VM (hostFs of conma.js): a negative
// result is minus the errno.
const errno = (e) => (typeof e.errno === "number" && e.errno < 0 ? e.errno : -5); // EIO
const hostFs = {
  open(p, mode) {
    try {
      const fd = fs.openSync(p, mode, 0o666);
      if (fs.fstatSync(fd).isDirectory()) {
        fs.closeSync(fd);
        return -21; // EISDIR, as the native VM finds it
      }
      return fd;
    } catch (e) {
      return errno(e);
    }
  },
  read(fd, len) {
    const buf = Buffer.alloc(len);
    try {
      return buf.subarray(0, retry(() => fs.readSync(fd, buf, 0, len, null)));
    } catch (e) {
      return errno(e);
    }
  },
  write(fd, bytes) {
    try {
      return retry(() => fs.writeSync(fd, bytes));
    } catch (e) {
      return errno(e);
    }
  },
  close(fd) {
    try {
      fs.closeSync(fd);
      return 0;
    } catch (e) {
      return errno(e);
    }
  },
  stat(p) {
    try {
      return fs.statSync(p).isDirectory() ? 1 : 2;
    } catch (e) {
      return errno(e);
    }
  },
  realpath(p) {
    try {
      return fs.realpathSync.native(p);
    } catch (e) {
      return errno(e);
    }
  },
  readdir(p) {
    try {
      return fs.readdirSync(p);
    } catch (e) {
      return errno(e);
    }
  },
};

// Resolves when a call of JavaScript has given its result to the VM.
let wake = null;
function jsDone() {
  if (wake) {
    const w = wake;
    wake = null;
    w();
  }
}

async function main() {
  const wasm = new WebAssembly.Module(fs.readFileSync(wasmFile));
  const vm = ConMaVM.create(wasm, writeAll, { inputNeeded: readInput, jsDone, hostFs });
  vm.useHostFs();
  vm.setCwd(process.cwd());
  for (const [k, v] of Object.entries(process.env)) vm.setenv(k, v);
  // The module file named in argv, read here as the native VM reads it.
  let i = 0;
  // As conma does: options before the module file, up to "--"
  while (i < argv.length && ["-t", "--trace", "-m", "--main"].includes(argv[i])) i++;
  if (argv[i] === "--") i++;
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
  let slices = 0;
  for (;;) {
    code = vm.run(100000);
    if (code !== ConMaVM.RUNNING && code !== ConMaVM.IDLE) return code;
    if (vm.jsPending() > 0) {
      // Let the Promises of JavaScript run: wait for one when nothing else can run, and
      // now and then while other VProcs run.
      if (code === ConMaVM.IDLE) {
        await new Promise((resolve) => {
          wake = resolve;
        });
        continue;
      }
      if (++slices % 8 === 0) await new Promise((resolve) => setImmediate(resolve));
    }
    // A program may poll standard input instead of waiting for it: give it input when it found none.
    if (code === ConMaVM.RUNNING && !(vm.wantsInput() && !inputEnded)) continue;
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

main().then((code) => {
  process.exitCode = code;
});
