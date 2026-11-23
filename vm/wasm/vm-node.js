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
// CONMA_HOST, if set, is the path of a JavaScript file (a CommonJS module) whose exports give the
// functions of the host that __Host_call__ calls and the platform features they make available
// ({hostFunctions: {name: f, ...}, hostFeatures: [...]}, the hooks of conma.js; docs/HOST-ja.md).
// Every program also has the functions of a network server (host/server.js, feature "server"; docs/dev/PLATFORM-PLAN-ja.md
// 4.10, stage P6b): a server that is listening keeps Node.js running, as http.createServer().listen() does.
"use strict";
const fs = require("fs");
const path = require("path");
const childProcess = require("child_process");
const os = require("os");
const ConMaVM = require("./conma.js");
const { serverHost } = require("./host/server.js");

const argv = process.argv.slice(2);
const wasmFile = path.join(__dirname, "target/wasm32-unknown-unknown/release/conma_wasm.wasm");

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

// Windows (2026-11-23): the VM has the paths of Windows as the native VM for Windows writes them,
// C:/Users/... (vm/src/sys/windows.rs, path_text), and Node.js gives them with backslashes.
const windows = process.platform === "win32";
// A path of Node.js as the VM has it.
function vmPath(p) {
  if (!windows) return p;
  if (p.startsWith("\\\\?\\UNC\\")) p = "\\\\" + p.slice(8);
  else if (p.startsWith("\\\\?\\")) p = p.slice(4);
  return p.replace(/\\/g, "/");
}
// A path of the VM for Node.js. A backslash is a separator, as on Windows. The VM for wasm32 joins
// paths as Unix does, so that a path of Windows after the current directory stays after it
// (C:/a/C:/b, or C:/a/C:\b from TEMP): it starts at the last drive. /dev/null is the device NUL,
// as in the native VM (open_file).
function hostPath(p) {
  if (!windows) return p;
  p = p.replace(/\\/g, "/");
  // the last drive: a greedy .* takes all but the last one
  const m = /^(?:.*\/)?([A-Za-z]:(?:\/.*)?)$/.exec(p);
  if (m && /^[A-Za-z]:/.test(p)) p = m[1];
  return unixDevice(p) === "/dev/null" ? "NUL" : p;
}
// The path of Unix that p is on Windows (/dev/null for C:/dev/null), or null.
function unixDevice(p) {
  return p.startsWith("/") ? p : /^[A-Za-z]:\//.test(p) ? p.slice(2) : null;
}
// Windows: /dev/stdin, /dev/stdout and /dev/stderr are copies of fds 0, 1 and 2, as in the native
// VM (open_file). Node.js cannot copy an fd: the copy is a number of its own (from STD_COPY) for
// the fd, which read and write use, and which close forgets.
const STD_COPY = 0x40000000;
const stdCopies = new Map();
let nextStdCopy = STD_COPY;
const realFd = (fd) => (stdCopies.has(fd) ? stdCopies.get(fd) : fd);

// The file system of this process, for the VM (hostFs of conma.js): a negative
// result is minus the errno. The errno of Node.js on Windows is that of libuv (ENOENT is -4058):
// it is given as the errno of Unix of the same name, which the VM for wasm32 knows (vm/wasm/src/sys.rs).
const unixErrno = { EPERM: 1, ENOENT: 2, ESRCH: 3, EIO: 5, E2BIG: 7, EBADF: 9, ECHILD: 10, EACCES: 13,
  EEXIST: 17, ENOTDIR: 20, EISDIR: 21, EINVAL: 22, EPIPE: 32, EAGAIN: 35, ENOSYS: 38 };
const errno = (e) => {
  if (windows) return -(unixErrno[e.code] || 5);
  return typeof e.errno === "number" && e.errno < 0 ? e.errno : -5; // EIO
};
// Windows: the errors of Unix for the names on the way to the last one of p, as the native VM
// gives them (check_on_the_way of vm/src/sys/windows.rs): ENOENT if one does not exist, ENOTDIR
// if one is a file (Windows says ENOENT for both, and goes over "name/.." without looking at name).
// Gives the path for Node.js.
function checkedPath(p) {
  p = hostPath(p);
  if (!windows || p === "NUL") return p;
  const parts = p.split("/");
  let onTheWay = parts[0];
  for (let i = 1; i < parts.length - 1; i++) {
    onTheWay += "/" + parts[i];
    if (parts[i] === "" || parts[i] === "." || parts[i] === "..") continue;
    let st;
    try {
      st = fs.statSync(onTheWay);
    } catch (e) {
      throw Object.assign(new Error(p), { code: "ENOENT" });
    }
    if (!st.isDirectory()) throw Object.assign(new Error(p), { code: "ENOTDIR" });
  }
  return p;
}

const hostFs = {
  open(p, mode) {
    const std = windows ? ["/dev/stdin", "/dev/stdout", "/dev/stderr"].indexOf(unixDevice(hostPath(p))) : -1;
    if (std >= 0) {
      stdCopies.set(nextStdCopy, std);
      return nextStdCopy++;
    }
    try {
      const fd = fs.openSync(checkedPath(p), mode, 0o666);
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
      return buf.subarray(0, retry(() => fs.readSync(realFd(fd), buf, 0, len, null)));
    } catch (e) {
      return errno(e);
    }
  },
  write(fd, bytes) {
    try {
      return retry(() => fs.writeSync(realFd(fd), bytes));
    } catch (e) {
      return errno(e);
    }
  },
  close(fd) {
    if (stdCopies.delete(fd)) return 0;
    try {
      fs.closeSync(fd);
      return 0;
    } catch (e) {
      return errno(e);
    }
  },
  stat(p) {
    try {
      return fs.statSync(checkedPath(p)).isDirectory() ? 1 : 2;
    } catch (e) {
      return errno(e);
    }
  },
  realpath(p) {
    try {
      return vmPath(fs.realpathSync.native(checkedPath(p)));
    } catch (e) {
      return errno(e);
    }
  },
  readdir(p) {
    try {
      return fs.readdirSync(hostPath(p));
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
// The VM traps SIGINT (__OS_trap_interrupt__): the run loop lets events come now and then even
// when nothing outside the VM runs, so that the handler gets Ctrl-C during a long computation
let sigintTrapped = false;
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
// The outputs of processes stopped (stream.pause()) because their pipe in the VM holds more than
// PIPE_LIMIT bytes, as the OS stops a process whose pipe is full: {fd, stream}. The pipes of the VM
// have no limit, and without this `for i in $(yes)` filled the 4 GB of the VM in a second on Linux
// (consh/test step12 case7, 2026-11-22). resumeOutputs lets them go on when the VM has read them.
const PIPE_LIMIT = 65536;
let pausedOutputs = [];

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
  // Windows: as Command of Rust finds it for the native VM: PATH is split at ";", and a name
  // without an extension is name.exe (or, given with its directory, the name itself).
  const names = (p) => (windows && path.extname(p) === "" ? [p + ".exe", p] : [p]);
  if (name.includes("/") || (windows && name.includes("\\"))) {
    let r = -2;
    for (const p of names(name)) {
      r = check(hostPath(p));
      if (r === 0) return hostPath(p);
    }
    return r;
  }
  let found = -2;
  const env_path = windows ? env.PATH || env.Path || "" : env.PATH || "";
  for (const dir of env_path.split(windows ? ";" : ":")) {
    for (const p of names(name).slice(0, 1)) {
      const file = path.join(dir || ".", p);
      const r = check(file);
      if (r === 0) return file;
      if (r === -13) found = -13;
    }
  }
  return found;
}

// Windows: the command line of the process, as the native VM writes it (vm/src/sys/windows.rs,
// add_args, and Command of Rust): an argument goes in double quotes, as the C runtime of Windows
// reads them, when it is empty or has a space, a tab, or a character that the programs of MSYS
// (Git Bash) would take otherwise (a backslash, * ? [ ] { } ' ~); a quote gets a backslash.
function windowsArg(a) {
  const quote = a === "" || /[ \t\\*?[\]{}'~]/.test(a);
  let q = quote ? '"' : "";
  let backslashes = 0;
  for (const c of a) {
    if (c === "\\") {
      backslashes++;
      continue;
    }
    q += "\\".repeat(c === '"' ? backslashes * 2 + 1 : backslashes) + c;
    backslashes = 0;
  }
  return q + "\\".repeat(quote ? backslashes * 2 : backslashes) + (quote ? '"' : "");
}

function makeHostProc(getVm) {
  return {
    spawn(argv, env, cwd, stdio, detached) {
      const vm = getVm();
      const file = findCommand(argv[0], env);
      if (typeof file === "number") return file;
      const spec = stdio.map((s) => (s.kind === 0 || s.kind === 1 ? realFd(s.n) : s.kind === 4 ? "ignore" : "pipe"));
      let child;
      try {
        child = windows
          ? childProcess.spawn(file, argv.slice(1).map(windowsArg), {
              argv0: windowsArg(argv[0]),
              cwd: hostPath(cwd),
              env,
              stdio: spec,
              detached,
              windowsVerbatimArguments: true,
            })
          : childProcess.spawn(file, argv.slice(1), { argv0: argv[0], cwd, env, stdio: spec, detached });
      } catch (e) {
        return errno(e);
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
        // Windows has no signals: a process that kill ended has the number of its signal (as
        // the native VM gives it, exit_code of vm/src/sys/windows.rs), and one that Ctrl-C ended
        // (STATUS_CONTROL_C_EXIT) has SIGINT
        if (windows && rec.signal) rec.code = -rec.signal;
        else if (windows && code === 0xc000013a) rec.code = -2;
        else rec.code = code !== null ? code : -(os.constants.signals[signal] || 0);
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
            const r = vm.write(s.n, d);
            if (r === 0 && vm.buffered(s.n) > PIPE_LIMIT) {
              stream.pause();
              pausedOutputs.push({ fd: s.n, stream, isClosed: () => closed });
            }
            if (r === -32) {
              // The reader in the VM has gone. A process of the native VM gets SIGPIPE when it
              // writes to such a pipe; this one writes to a socket of Node.js, which would give
              // it an error instead: send it SIGPIPE.
              // On Windows, the process gets an error when it writes, as from the native VM.
              if (rec.code === null && !windows) {
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
        if (windows) {
          // Node.js sends only some signals on Windows; every one ends the process
          // (TerminateProcess), as kill_process of the native VM does.
          rec.signal = sig;
          rec.child.kill("SIGKILL");
        } else rec.child.kill(sig);
        return 0;
      } catch (e) {
        return windows ? errno(e) : typeof e.errno === "number" && e.errno < 0 ? e.errno : -3;
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

// Let the stopped outputs go on whose pipe the VM has read (or whose reader has gone: the next
// write gets EPIPE, and the process SIGPIPE).
function resumeOutputs(vm) {
  pausedOutputs = pausedOutputs.filter((o) => {
    if (o.isClosed()) return false;
    if (vm.buffered(o.fd) > PIPE_LIMIT && vm.write(o.fd, new Uint8Array(0)) !== -32) return true;
    o.stream.resume();
    return false;
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
// include directories, after include/ext, or include/core for a module of core ConMa when the
// program runs one); the file names in SInfo relative to the directory above the repository.
function buildModule(wasm, source, cwd, core) {
  if (frontendBytes === null) frontendBytes = fs.readFileSync(frontendFile);
  const out = [];
  const err = [];
  const sub = ConMaVM.create(wasm, (fd, bytes) => (fd === 1 ? out : err).push(Buffer.from(bytes)), { hostFs });
  sub.useHostFs();
  sub.setCwd(cwd);
  for (const [k, v] of Object.entries(process.env)) sub.setenv(k, v);
  sub.inputEnd();
  const include = process.env.CONMA_INCLUDE || vmPath(path.join(root, "include"));
  const first = vmPath(path.join(include, core ? "core" : "ext"));
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
  const prefix = '"' + vmPath(path.dirname(root)) + "/";
  return { ok: true, text: Buffer.concat(out).toString().split(prefix).join('"') };
}

// The VM of main, for the report of process.on("beforeExit") below.
let liveVM = null;

async function main() {
  const wasm = new WebAssembly.Module(fs.readFileSync(wasmFile));
  let vm = null;
  const host = process.env.CONMA_HOST ? require(path.resolve(process.env.CONMA_HOST)) : {};
  // The network server of Node.js (host/server.js, feature "server"; stage P6b), then those of CONMA_HOST
  const server = serverHost();
  vm = ConMaVM.create(wasm, writeAll, {
    hostFunctions: Object.assign({}, server.hostFunctions, host.hostFunctions || {}),
    hostFeatures: server.hostFeatures.concat(Array.isArray(host.hostFeatures) ? host.hostFeatures : []),
    inputNeeded: readInput,
    jsDone: notify,
    hostFs,
    hostProc: makeHostProc(() => vm),
    buildModule: (source, cwd, core) => buildModule(wasm, source, cwd, core),
    // As the native VM: SIGINT ends this process until the VM traps it.
    trapInterrupt: (v) => {
      if (process.listenerCount("SIGINT") > 0) return;
      sigintTrapped = true;
      process.on("SIGINT", () => {
        v.interrupt();
        notify();
      });
    },
  });
  liveVM = vm;
  vm.useHostFs();
  vm.setCwd(vmPath(process.cwd()));
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
    resumeOutputs(vm);
    // A program may poll standard input instead of waiting for it: read it when it found none.
    if (vm.wantsInput()) readInputAsync(vm);
    const outside = vm.jsPending() > 0 || processesActive() || reading;
    if (code === ConMaVM.RUNNING) {
      // Let events (JavaScript, processes, input, and SIGINT once it is trapped) come now and then.
      if ((outside || sigintTrapped) && ++slices % 4 === 0) await new Promise((resolve) => setImmediate(resolve));
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

// A Promise of a function of the host that never settles, and nothing else to do: Node.js would end
// with nothing written and the status 0 (docs/dev/PLATFORM-PLAN-ja.md 4.9, stage P7). Report it as
// a deadlock, as the line above does when nothing at all can wake the VM.
process.on("beforeExit", () => {
  if (!liveVM) return;
  const n = liveVM.hostPending();
  const why = n > 0 ? `for host calls that never finish: ${n}` : "for calls of JavaScript that never finish";
  writeAll(2, ConMaVM.utf8(`vm-node: every VProc waits ${why}\n`));
  process.exit(2);
});

// Exit without waiting for processes that are still running (the native VM leaves them).
// Out of memory: one line, as the native VM writes, and the status of abort (134), as the native VM
main().then(
  (code) => process.exit(code),
  (e) => {
    if (!(e && e.outOfMemory)) throw e;
    writeAll(2, ConMaVM.utf8(`${e.message}\n`));
    process.exit(134);
  },
);
