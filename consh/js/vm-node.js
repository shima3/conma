// Run the ConMa VM for wasm32 in Node.js, as conma (vm/src/main.rs) runs:
//   node vm-node.js [-t] [-m] <module_file> <func_name | arg ...>
// vm-wasm.sh uses it when node is found. The VM starts in the current directory, with the environment
// variables of the process, and uses the files of this process (hostFs, conma_use_host_fs): it reads
// and writes them as the native VM does. __OS_spawn__ starts processes (hostProc, child_process): a
// file or the interpreter's own standard input, output or error goes to the process as it is, and this
// process moves the data between a pipe of the VM and the process. Standard input is read when a VProc
// waits for it. The builtin js of ConSh (__JS_call__) evaluates JavaScript in this process (conma.js).
// __VProc_spawn__ makes the module of its source file as conma build does: another instance of the
// VM runs the ConMa frontend in the repository (vm/frontend.module.se) to its end.
"use strict";
const fs = require("fs");
const path = require("path");
const childProcess = require("child_process");
const os = require("os");
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

// ── Waking the loop of main ──
// Something happened outside the VM (a result of JavaScript, data of a process, an exit, input).
let wake = null;
let dirty = false;
function notify() {
  dirty = true;
  if (wake) {
    const w = wake;
    wake = null;
    w();
  }
}

// Standard input read without blocking the loop, while a VProc waits for it.
let reading = false;
function readInputAsync(vm) {
  if (reading || inputEnded) return;
  reading = true;
  fs.read(0, inbuf, 0, inbuf.length, null, (e, n) => {
    reading = false;
    if (inputEnded) return;
    if (e && e.code === "EAGAIN") {
      setTimeout(() => readInputAsync(vm), 5);
      return;
    }
    if (!e && n > 0) {
      vm.input(inbuf.subarray(0, n));
    } else {
      vm.inputEnd();
      inputEnded = true;
    }
    notify();
  });
}

// ── Processes (hostProc of conma.js) ──
const procs = new Map(); // id -> {child, code}
let nextProc = 0;
// Pipes of the VM that feed the standard input of a process: {fd, stream}
let feeds = [];
// Output of processes not yet given to the VM (ends of pipes still open)
let openOutputs = 0;

// The file to run for name, as execvp finds it in PATH; or minus an errno.
function findCommand(name, env) {
  const check = (p) => {
    try {
      if (fs.statSync(p).isDirectory()) return -13;
      fs.accessSync(p, fs.constants.X_OK);
      return 0;
    } catch (e) {
      return e.code === "EACCES" ? -13 : -2;
    }
  };
  if (name.includes("/")) {
    const r = check(name);
    return r === 0 ? name : r;
  }
  let found = -2;
  for (const dir of (env.PATH || "").split(":")) {
    const p = path.join(dir || ".", name);
    const r = check(p);
    if (r === 0) return p;
    if (r === -13) found = -13;
  }
  return found;
}

function makeHostProc(getVm) {
  return {
    spawn(argv, env, cwd, stdio, detached) {
      const vm = getVm();
      const file = findCommand(argv[0], env);
      if (typeof file === "number") return file;
      const spec = stdio.map((s) => (s.kind === 0 || s.kind === 1 ? s.n : s.kind === 4 ? "ignore" : "pipe"));
      let child;
      try {
        child = childProcess.spawn(file, argv.slice(1), { argv0: argv[0], cwd, env, stdio: spec, detached });
      } catch (e) {
        return typeof e.errno === "number" && e.errno < 0 ? e.errno : -5;
      }
      const id = ++nextProc;
      const rec = { child, code: null };
      procs.set(id, rec);
      child.on("error", () => {
        if (rec.code === null) rec.code = 127;
        vm.childEvent();
        notify();
      });
      child.on("exit", (code, signal) => {
        rec.code = code !== null ? code : -(os.constants.signals[signal] || 0);
        vm.childEvent();
        notify();
      });
      [child.stdin, child.stdout, child.stderr].forEach((stream, i) => {
        const s = stdio[i];
        if (s.kind === 2) {
          stream.on("error", () => {}); // EPIPE when the process has gone
          feeds.push({ fd: s.n, stream, child });
        } else if (s.kind === 3) {
          openOutputs++;
          let closed = false;
          const close = () => {
            if (closed) return;
            closed = true;
            openOutputs--;
            vm.close(s.n);
            vm.childEvent();
            notify();
          };
          stream.on("data", (d) => {
            if (closed) return;
            if (vm.write(s.n, d) === -32) {
              // The reader in the VM has gone. A process of the native VM gets SIGPIPE when it
              // writes to such a pipe; this one writes to a socket of Node.js, which would give
              // it an error instead: send it SIGPIPE.
              if (rec.code === null) {
                try {
                  child.kill("SIGPIPE");
                } catch (e) {
                  // it has just exited
                }
              }
              stream.destroy();
              close();
            }
            notify();
          });
          stream.on("end", close);
          stream.on("close", close);
          stream.on("error", close);
        }
      });
      return id;
    },
    poll(id) {
      const rec = procs.get(id);
      return rec ? rec.code : -10; // ECHILD-like: should not happen
    },
    kill(id, sig) {
      const rec = procs.get(id);
      if (!rec || rec.code !== null) return 0;
      try {
        rec.child.kill(sig);
        return 0;
      } catch (e) {
        return typeof e.errno === "number" && e.errno < 0 ? e.errno : -3;
      }
    },
  };
}

// Move what the VM wrote to the pipes for the standard input of processes.
function feedProcesses(vm) {
  feeds = feeds.filter((f) => {
    for (;;) {
      if (f.child.exitCode !== null || f.child.signalCode !== null || f.stream.destroyed) {
        vm.close(f.fd); // the writers in the VM get EPIPE
        return false;
      }
      const r = vm.read(f.fd, 65536);
      if (typeof r === "number") {
        if (r === -35) return true; // nothing yet
        vm.close(f.fd);
        f.stream.end();
        return false;
      }
      if (r.length === 0) {
        vm.close(f.fd);
        f.stream.end();
        return false;
      }
      f.stream.write(r);
    }
  });
}

function processesActive() {
  if (feeds.length > 0 || openOutputs > 0) return true;
  for (const rec of procs.values()) if (rec.code === null) return true;
  return false;
}

// ── __VProc_spawn__ (buildModule of conma.js) ──
const root = path.resolve(__dirname, "..", "..");
const frontendFile = path.join(root, "vm", "frontend.module.se");
let frontendBytes = null;

// The module of the ConMa source file (relative to cwd), made by the frontend in another VM, as
// conma build does (the standard library, CONMA_INCLUDE or include/ of the repository, among the
// include directories, after include/fast, or include/ref for a module of concise ConMa when the
// program runs one); the file names in SInfo relative to the directory above the repository.
function buildModule(wasm, source, cwd, concise) {
  if (frontendBytes === null) frontendBytes = fs.readFileSync(frontendFile);
  const out = [];
  const err = [];
  const sub = ConMaVM.create(wasm, (fd, bytes) => (fd === 1 ? out : err).push(Buffer.from(bytes)), { hostFs });
  sub.useHostFs();
  sub.setCwd(cwd);
  for (const [k, v] of Object.entries(process.env)) sub.setenv(k, v);
  sub.inputEnd();
  const include = process.env.CONMA_INCLUDE || path.join(root, "include");
  const first = path.join(include, concise ? "ref" : "fast");
  const dirs = fs.existsSync(first) ? ["--include", first, "--include", include] : ["--include", include];
  let code = sub.start(frontendBytes, ["--main", "conma/vm/frontend.module.se", ...dirs, source]);
  if (code === 0) {
    do code = sub.run(1000000);
    while (code === ConMaVM.RUNNING);
  }
  if (code !== 0) {
    const text = Buffer.concat(err).toString().trim();
    return { ok: false, message: code === ConMaVM.IDLE ? "the frontend waits" : (text.split("\n")[0] || "").trim() };
  }
  const prefix = '"' + path.dirname(root) + "/";
  return { ok: true, text: Buffer.concat(out).toString().split(prefix).join('"') };
}

async function main() {
  const wasm = new WebAssembly.Module(fs.readFileSync(wasmFile));
  let vm = null;
  vm = ConMaVM.create(wasm, writeAll, {
    inputNeeded: readInput,
    jsDone: notify,
    hostFs,
    hostProc: makeHostProc(() => vm),
    buildModule: (source, cwd, concise) => buildModule(wasm, source, cwd, concise),
    // As the native VM: SIGINT ends this process until the VM traps it.
    trapInterrupt: (v) => {
      if (process.listenerCount("SIGINT") > 0) return;
      process.on("SIGINT", () => {
        v.interrupt();
        notify();
      });
    },
  });
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
    dirty = false;
    code = vm.run(100000);
    if (code !== ConMaVM.RUNNING && code !== ConMaVM.IDLE) return code;
    feedProcesses(vm);
    // A program may poll standard input instead of waiting for it: read it when it found none.
    if (vm.wantsInput()) readInputAsync(vm);
    const outside = vm.jsPending() > 0 || processesActive() || reading;
    if (code === ConMaVM.RUNNING) {
      // Let events (JavaScript, processes, input) come now and then.
      if (outside && ++slices % 4 === 0) await new Promise((resolve) => setImmediate(resolve));
      continue;
    }
    // IDLE: every VProc waits for something outside the VM.
    if (dirty) continue;
    if (!outside && !inputEnded) readInputAsync(vm);
    if (!outside && !reading) {
      // Input has ended, and no process or JavaScript can wake the VM
      // (the native VM would wait for ever).
      writeAll(2, ConMaVM.utf8("vm-node: every VProc waits, and nothing can wake them\n"));
      return 2;
    }
    await new Promise((resolve) => {
      wake = resolve;
    });
  }
}

// Exit without waiting for processes that are still running (the native VM leaves them).
main().then((code) => process.exit(code));
