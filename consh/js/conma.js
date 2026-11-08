// The host side of the ConMa VM for wasm32 (consh/js/wasm). It works in a
// browser (<script src="conma.js">), in Node.js (require) and in the
// JavaScriptCore shell (load()).
//
//   const vm = ConMaVM.create(wasmModule, (fd, bytes) => ..., { ...hooks });
//   vm.start(moduleBytes, ["-m", "consh.module.se", ...]);  // 0 if started
//   for (;;) {
//     const r = vm.run(100000);
//     if (r === ConMaVM.RUNNING) continue;     // let the host breathe, then go on
//     if (r === ConMaVM.IDLE) break;           // wait for input or Ctrl-C, then go on
//     ... r is the exit code ...
//   }
//
// wasmModule is a compiled WebAssembly.Module; write(fd, bytes) receives the
// bytes the VM writes to standard output (1) and standard error (2). The
// hooks, all optional, are these:
// inputNeeded(vm), if given, is called when a read of standard input that
// cannot wait (__OS_get_String__) finds it empty: it may call vm.input or
// vm.inputEnd at once (a browser cannot, and such a read sees the end).
// fileNeeded(vm, path), if given, is called when the VM looks for a file
// (an absolute path) that is not in the virtual file system: it may call
// vm.putFile (or vm.makeDirs for a directory) at once. dirNeeded(vm, path),
// if given, is called when the VM lists a directory for the first time: it
// may call vm.makeDirs and vm.putUnread for its entries at once.
// hostFs, if given, is the file system of the host, for vm.useHostFs(), as
// functions of absolute paths that return minus an errno on an error:
//   open(path, mode)  mode "r", "w" or "a"; a file descriptor (-21 for a directory)
//   read(fd, len)     a Uint8Array (empty at the end)
//   write(fd, bytes)  the number written
//   close(fd)
//   stat(path)        1 for a directory, 2 for another file (links followed)
//   realpath(path)    a string
//   readdir(path)     an array of names
// hostProc, if given, starts processes for __OS_spawn__ (the VM calls them):
//   spawn(argv, env, cwd, stdio, detached)  argv an array of strings, env an
//       object, stdio 3 pairs {kind, n} (see host_proc_spawn in wasm/src/sys.rs);
//       the number of the process, or minus an errno
//   poll(id)       the exit code (minus a signal number), or null while it runs
//   kill(id, sig)  0, or minus an errno
// trapInterrupt(vm), if given, is called when the VM traps SIGINT
// (__OS_trap_interrupt__): from then on, the host calls vm.interrupt() for a
// SIGINT (a browser does it for Ctrl-C anyway).
// The host moves the data of the pipe ends of kinds 2 and 3 with vm.read,
// vm.write and vm.close, and calls vm.childEvent() when a process exits.
// buildModule(source, cwd), if given, makes the module of a ConMa source file
// for __VProc_spawn__, as conma build does: {ok: true, text} with the file
// names in SInfo relative to the directory above the repository, or
// {ok: false, message} with the first line of the error.
// jsDone(vm), if given, is called when the result of a call of JavaScript
// (__JS_call__, the builtin js of ConSh) has been given to the VM: the host
// should run the VM again if it was IDLE. While vm.jsPending() > 0, the host
// must let its event loop run (Promises) when the VM is IDLE.
//
// __JS_call__ code args: code is evaluated as an expression in the global
// scope, where it also sees stdin; if its value is a function, it is called
// with args (strings, and arrays of them); code that is not an expression is
// run as the body of an async function, whose result is the value of its
// return (it sees stdin and args); the result (awaited if it is a
// Promise) is written to the pipe of __JS_call__ as a line, an S-expression
// that __Sexp_read__ reads (see 2.1 and 2.2 of consh/js/PLAN-ja.md); lines
// before it say what else the call needs:
//   (o "text")         console.log (and info, debug): an element of standard output
//   (x "text")         console.error (and warn): an element of standard error
//   (i)                stdin is read: the VM writes the next element of the
//                      standard input to the second Stream of __JS_call__ as
//                      a line, a String in double quotes, or the line end
//   (r s "text")       a string, or a number, boolean, BigInt or object made
//                      into one (String(), JSON.stringify)
//   (r l ("a" ("b")))  an array: its elements, an array as a list, null and
//                      undefined as ""
//   (r n)              undefined or null: nothing
//   (e "message")      an exception: "Name: message" for an Error
//   (r t)              an iterator (any iterable object but an array or a
//                      string, a generator too): then, for each line n that
//                      the VM writes to the second Stream, the host takes the
//                      next element and writes (t "text") or (t (...)) (an
//                      element as one of an array), or (d) at the end, or
//                      (e "message")
// When the VM closes the second Stream (its reader has gone), the host stops
// the call (and the iterator, with its return(); what its finally writes with
// console goes to the host's standard output and error).
// console goes to the call whose code runs now, or, while it awaits, to the
// only call that has not ended; when two or more have not ended, to the
// host's standard output and error (write), and when none, to the console.

var ConMaVM = (function () {
  "use strict";

  // UTF-8 bytes of a string (TextEncoder is missing in the JavaScriptCore shell).
  function utf8(s) {
    if (typeof TextEncoder !== "undefined") return new TextEncoder().encode(s);
    const out = [];
    for (let i = 0; i < s.length; i++) {
      let c = s.codePointAt(i);
      if (c > 0xffff) i++;
      if (c >= 0xd800 && c <= 0xdfff) c = 0xfffd; // a lone surrogate
      if (c < 0x80) out.push(c);
      else if (c < 0x800) out.push(0xc0 | (c >> 6), 0x80 | (c & 63));
      else if (c < 0x10000) out.push(0xe0 | (c >> 12), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
      else out.push(0xf0 | (c >> 18), 0x80 | ((c >> 12) & 63), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
    }
    return new Uint8Array(out);
  }

  // A string with a character for each byte.
  function latin1(bytes) {
    let s = "";
    for (let i = 0; i < bytes.length; i += 8192) s += String.fromCharCode.apply(null, bytes.subarray(i, i + 8192));
    return s;
  }

  // The string of UTF-8 bytes (TextDecoder is missing in the JavaScriptCore shell).
  function text(bytes) {
    if (typeof TextDecoder !== "undefined") return new TextDecoder().decode(bytes);
    try {
      return decodeURIComponent(escape(latin1(bytes)));
    } catch (e) {
      return latin1(bytes);
    }
  }

  // ── JavaScript of __JS_call__ ──

  // An error that the host finds, written without the name of an Error.
  function HostError(message) {
    this.message = message;
  }

  const AsyncFunction = async function () {}.constructor;

  // The code as an expression in the global scope (a Function, which is not
  // strict, so that an assignment to an undeclared name makes a global), with
  // stdin as a parameter; if its value is a function, it is called with args.
  // Code that is not an expression is the body of an async function, whose
  // result is the value of its return (it may await), and which sees stdin
  // and args (an array). Code that is neither is the syntax error of the
  // expression.
  function evaluate(code, args, stdin) {
    let expression;
    try {
      expression = new Function("stdin", "args", "return (" + code + "\n)");
    } catch (e) {
      if (!(e instanceof SyntaxError)) throw e;
      let body;
      try {
        body = new AsyncFunction("stdin", "args", code);
      } catch (e2) {
        throw e;
      }
      return body(stdin, args);
    }
    const value = expression(stdin, args);
    if (typeof value === "function") return value(...args);
    if (args.length > 0) throw new HostError("arguments given to a value that is not a function");
    return value;
  }

  // A String in double quotes, as the VM writes the elements of standard input.
  function unquote(line) {
    const m = /^\s*"((?:[^"\\]|\\.)*)"\s*$/s.exec(line);
    if (!m) return line;
    return m[1].replace(/\\(.)/gs, (_, c) => ({ n: "\n", t: "\t", r: "\r" })[c] || c);
  }

  const END = {};

  // A call of JavaScript: the pipe fd its lines go to, and its standard input.
  function Job(fd, emit) {
    this.fd = fd;
    this.emit = emit; // write a line to the pipe
    this.ended = false;
    this.items = []; // elements of standard input not read yet
    this.inputEnded = false;
    this.partial = "";
    this.waiters = [];
    this.asked = false;
  }
  // The next element of standard input (a string), or END.
  Job.prototype.read = function () {
    if (this.items.length > 0) return Promise.resolve(this.items.shift());
    if (this.inputEnded || this.ended) return Promise.resolve(END);
    if (!this.asked) {
      this.asked = true;
      this.emit("(i)\n");
    }
    return new Promise((resolve) => this.waiters.push(resolve));
  };
  Job.prototype.give = function (item) {
    this.asked = false;
    if (this.waiters.length > 0) {
      this.waiters.shift()(item);
      if (this.waiters.length > 0 && item !== END) {
        this.asked = true;
        this.emit("(i)\n");
      }
    } else if (item !== END) {
      this.items.push(item);
    }
    if (item === END) for (const w of this.waiters.splice(0)) w(END);
  };
  Job.prototype.input = function (data) {
    const lines = (this.partial + data).split("\n");
    this.partial = lines.pop();
    for (const l of lines) {
      if (l === "end") this.inputEnd();
      else if (l === "n") {
        if (this.onNext) this.onNext();
      } else this.give(unquote(l));
    }
  };
  // The VM closed the second Stream: the end of standard input, and no reader.
  Job.prototype.closed = function () {
    this.inputEnd();
    if (this.onClosed) this.onClosed();
  };
  Job.prototype.inputEnd = function () {
    this.inputEnded = true;
    this.give(END);
  };

  // stdin of the code: an async iterator of the elements of standard input
  // (strings), and lines(), a Promise of all of them.
  function makeStdin(job) {
    const iterator = {
      next: () => job.read().then((v) => (v === END ? { done: true, value: undefined } : { done: false, value: v })),
      return: () => Promise.resolve({ done: true, value: undefined }),
      [Symbol.asyncIterator]() {
        return this;
      },
    };
    return {
      [Symbol.asyncIterator]: () => iterator,
      lines: async () => {
        const all = [];
        for (;;) {
          const v = await job.read();
          if (v === END) return all;
          all.push(v);
        }
      },
    };
  }

  // ── console ──

  let consoleJob = null; // the call whose code runs now
  const liveJobs = new Set();
  let consoleFallback = null; // write(fd, bytes) of the host
  let consoleOriginal = null;

  function inspect(v) {
    if (typeof v === "string") return v;
    if (typeof v === "object" && v !== null) {
      try {
        const j = JSON.stringify(v);
        if (j !== undefined) return j;
      } catch (e) {
        // a cycle: as String() makes it
      }
    }
    return String(v);
  }

  function installConsole(write) {
    consoleFallback = write;
    if (consoleOriginal || typeof console === "undefined") return;
    consoleOriginal = {};
    for (const [name, tag] of [["log", "o"], ["info", "o"], ["debug", "o"], ["error", "x"], ["warn", "x"]]) {
      const original = console[name];
      consoleOriginal[name] = original;
      console[name] = function (...args) {
        if (liveJobs.size === 0) return original.apply(console, args);
        const text = args.map(inspect).join(" ");
        let job = consoleJob || (liveJobs.size === 1 ? liveJobs.values().next().value : null);
        if (job && job.closing) job = null; // its reader has gone (a finally of a generator that is stopped)
        if (job) job.emit("(" + tag + " " + sexpString(text) + ")\n");
        else consoleFallback(tag === "o" ? 1 : 2, utf8(text + "\n"));
      };
    }
  }

  function sexpString(s) {
    return '"' + s.replace(/[\\"\n\r\t]/g, (c) => ({ "\\": "\\\\", '"': '\\"', "\n": "\\n", "\r": "\\r", "\t": "\\t" })[c]) + '"';
  }

  // A value that is not an array, as a string.
  function scalar(v) {
    if (typeof v === "string") return v;
    if (typeof v === "function") throw new HostError("the result is a function");
    if (typeof v === "object") return JSON.stringify(v);
    return String(v);
  }

  function sexpList(a) {
    return "(" + a.map((e) => (Array.isArray(e) ? sexpList(e) : sexpString(e == null ? "" : scalar(e)))).join(" ") + ")";
  }

  function isIterator(v) {
    return (
      v !== null &&
      typeof v === "object" &&
      !Array.isArray(v) &&
      (typeof v[Symbol.asyncIterator] === "function" || typeof v[Symbol.iterator] === "function")
    );
  }

  // An element of an iterator, as one of an array.
  function elementSexp(e) {
    return Array.isArray(e) ? sexpList(e) : sexpString(e == null ? "" : scalar(e));
  }

  function resultLine(v) {
    if (v === undefined || v === null) return "(r n)\n";
    if (Array.isArray(v)) return "(r l " + sexpList(v) + ")\n";
    return "(r s " + sexpString(scalar(v)) + ")\n";
  }

  function errorLine(e) {
    let msg;
    if (e instanceof HostError) msg = e.message;
    else if (e instanceof Error || (e && typeof e === "object" && "message" in e && "name" in e)) msg = e.name + ": " + e.message;
    else msg = String(e);
    return "(e " + sexpString(msg) + ")\n";
  }

  function create(wasmModule, write, hooks) {
    const { inputNeeded, fileNeeded, dirNeeded, jsDone, hostFs, hostProc, trapInterrupt, buildModule } = hooks || {};
    // The last result of buildModule, until the VM has taken it all (host_build_module)
    let lastBuild = null;
    let x = null;
    let vm = null;
    let jsPending = 0;
    const jobs = new Map();
    installConsole(write);
    // Run the VM again (later: this may be inside conma_run).
    const wake = () => {
      if (jsDone) Promise.resolve().then(() => jsDone(vm));
    };
    const instance = new WebAssembly.Instance(wasmModule, {
      env: {
        host_write: (fd, ptr, len) => write(fd, new Uint8Array(x.memory.buffer, ptr, len).slice()),
        host_input_needed: () => {
          if (inputNeeded) inputNeeded(vm);
        },
        host_file_needed: (ptr, len) => {
          if (fileNeeded) fileNeeded(vm, text(new Uint8Array(x.memory.buffer, ptr, len)));
        },
        host_dir_needed: (ptr, len) => {
          if (dirNeeded) dirNeeded(vm, text(new Uint8Array(x.memory.buffer, ptr, len)));
        },
        host_js_call: (fd, id, codePtr, codeLen, argsPtr, argsLen) => {
          const code = text(new Uint8Array(x.memory.buffer, codePtr, codeLen));
          const args = JSON.parse(text(new Uint8Array(x.memory.buffer, argsPtr, argsLen)));
          jsPending++;
          // Lines go to the pipe at once (the virtual OS may be used inside conma_run), and the VM
          // runs again after them.
          const job = new Job(fd, (line) => {
            if (job.ended) return;
            const errno = pass(line, (p, n) => x.conma_write(fd, p, n));
            wake();
            if (errno === 32 && job.onClosed) job.onClosed(); // EPIPE: the reader has gone
          });
          // End the call, writing line (if not null) as its last.
          const finish = (line) => {
            if (job.ended) return;
            if (line !== null) job.emit(line);
            job.ended = true;
            liveJobs.delete(job);
            jobs.delete(id);
            x.conma_close(fd);
            jsPending--;
            for (const w of job.waiters.splice(0)) w(END);
            wake();
          };
          // Give the elements of an iterator one at a time, on the requests of the VM.
          const iterate = (v) => {
            const it = typeof v[Symbol.asyncIterator] === "function" ? v[Symbol.asyncIterator]() : v[Symbol.iterator]();
            let requested = 0;
            let busy = false;
            job.onClosed = () => {
              if (job.ended || job.closing) return;
              job.closing = true;
              try {
                if (typeof it.return === "function") Promise.resolve(it.return()).catch(() => {});
              } catch (e) {
                // a return() that throws: the call ends anyway
              }
              finish(null);
            };
            job.onNext = async () => {
              requested++;
              if (busy) return;
              busy = true;
              while (requested > 0 && !job.ended) {
                requested--;
                let line;
                try {
                  let step;
                  consoleJob = job;
                  try {
                    step = it.next();
                  } finally {
                    consoleJob = null;
                  }
                  const r = await step;
                  if (r.done) {
                    finish("(d)\n");
                    break;
                  }
                  line = "(t " + elementSexp(await r.value) + ")\n";
                } catch (e) {
                  finish(errorLine(e));
                  break;
                }
                job.emit(line);
              }
              busy = false;
            };
            job.emit("(r t)\n");
          };
          jobs.set(id, job);
          liveJobs.add(job);
          let result;
          consoleJob = job;
          try {
            result = Promise.resolve(evaluate(code, args, makeStdin(job)));
          } catch (e) {
            result = Promise.reject(e);
          } finally {
            consoleJob = null;
          }
          result
            .then((v) => {
              if (!isIterator(v)) return resultLine(v);
              iterate(v);
              return undefined;
            })
            .catch(errorLine)
            .then((line) => {
              if (line !== undefined) finish(line);
            });
        },
        host_fs_open: (ptr, len, mode) => (hostFs ? hostFs.open(str(ptr, len), String.fromCharCode(mode)) : -38),
        host_fs_read: (fd, ptr, len) => {
          if (!hostFs) return -38;
          const r = hostFs.read(fd, len);
          if (typeof r === "number") return r;
          new Uint8Array(x.memory.buffer).set(r, ptr);
          return r.length;
        },
        host_fs_write: (fd, ptr, len) => (hostFs ? hostFs.write(fd, new Uint8Array(x.memory.buffer, ptr, len).slice()) : -38),
        host_fs_close: (fd) => (hostFs ? hostFs.close(fd) : -38),
        host_fs_stat: (ptr, len) => (hostFs ? hostFs.stat(str(ptr, len)) : -38),
        host_fs_realpath: (ptr, len, out, cap) => (hostFs ? give(hostFs.realpath(str(ptr, len)), out, cap) : -38),
        host_fs_readdir: (ptr, len, out, cap) => {
          if (!hostFs) return -38;
          const r = hostFs.readdir(str(ptr, len));
          return give(typeof r === "number" ? r : r.map((n) => n + "\0").join(""), out, cap);
        },
        host_proc_spawn: (argvPtr, argvLen, envPtr, envLen, cwdPtr, cwdLen, stdioPtr, detached) => {
          if (!hostProc) return -38;
          const argv = str(argvPtr, argvLen).split("\0").slice(0, -1);
          const env = {};
          for (const e of str(envPtr, envLen).split("\0").slice(0, -1)) {
            const i = e.indexOf("=");
            env[e.slice(0, i)] = e.slice(i + 1);
          }
          const nums = new Int32Array(x.memory.buffer, stdioPtr, 6);
          const stdio = [0, 1, 2].map((i) => ({ kind: nums[2 * i], n: nums[2 * i + 1] }));
          return hostProc.spawn(argv, env, str(cwdPtr, cwdLen), stdio, detached !== 0);
        },
        host_proc_poll: (id) => {
          if (!hostProc) return 999;
          const c = hostProc.poll(id);
          return c === null ? 999 : c;
        },
        host_proc_kill: (id, sig) => (hostProc ? hostProc.kill(id, sig) : -3),
        host_build_module: (srcPtr, srcLen, cwdPtr, cwdLen, out, cap) => {
          const key = str(srcPtr, srcLen) + "\0" + str(cwdPtr, cwdLen);
          if (!lastBuild || lastBuild.key !== key) {
            let r;
            if (!buildModule) r = "1cannot make a module in this host";
            else {
              const b = buildModule(str(srcPtr, srcLen), str(cwdPtr, cwdLen));
              r = b.ok ? "0" + b.text : "1" + b.message;
            }
            lastBuild = { key, bytes: utf8(r) };
          }
          const bytes = lastBuild.bytes;
          if (bytes.length <= cap) {
            new Uint8Array(x.memory.buffer).set(bytes, out);
            lastBuild = null;
          }
          return bytes.length;
        },
        host_trap_sigint: () => {
          if (trapInterrupt) trapInterrupt(vm);
        },
        host_js_input: (id, ptr, len) => {
          const job = jobs.get(id);
          if (job) job.input(text(new Uint8Array(x.memory.buffer, ptr, len)));
        },
        host_js_input_end: (id) => {
          const job = jobs.get(id);
          if (job) job.closed();
        },
      },
    });
    x = instance.exports;

    // The string at ptr in the memory of the VM.
    function str(ptr, len) {
      return text(new Uint8Array(x.memory.buffer, ptr, len));
    }
    // Write the string s (UTF-8) to out if it fits in cap bytes; its length, or a negative errno.
    function give(s, out, cap) {
      if (typeof s === "number") return s;
      const bytes = utf8(s);
      if (bytes.length <= cap) new Uint8Array(x.memory.buffer).set(bytes, out);
      return bytes.length;
    }

    // Copy bytes (or a string as UTF-8) into the memory of the VM for f(ptr, len).
    function pass(data, f) {
      const bytes = typeof data === "string" ? utf8(data) : data;
      const ptr = x.conma_alloc(bytes.length);
      new Uint8Array(x.memory.buffer).set(bytes, ptr);
      try {
        return f(ptr, bytes.length);
      } finally {
        x.conma_free(ptr, bytes.length);
      }
    }
    function pass2(a, b, f) {
      return pass(a, (pa, la) => pass(b, (pb, lb) => f(pa, la, pb, lb)));
    }

    vm = {
      exports: x,
      // The module text (bytes or a string) and the arguments of conma
      // (vm/src/main.rs) after the module is read (an array of strings, or
      // their UTF-8 bytes separated by NUL). Returns 0, or an exit code.
      start: (module, argv) => pass2(module, Array.isArray(argv) ? argv.join("\0") : argv, x.conma_start),
      // Run about budget steps: RUNNING, IDLE, or the exit code.
      run: (budget) => x.conma_run(budget),
      input: (data) => pass(data, x.conma_input),
      inputEnd: () => x.conma_input_end(),
      // The number of calls of JavaScript whose results the VM has not got yet.
      jsPending: () => jsPending,
      // Whether a VProc found standard input empty since the last call.
      wantsInput: () => x.conma_wants_input() !== 0,
      interrupt: () => x.conma_interrupt(),
      // These return 0, or the errno of the error.
      putFile: (path, data) => pass2(path, data, x.conma_put_file),
      makeDirs: (path) => pass(path, x.conma_make_dirs),
      // Take a directory on the way of a path that fileNeeded does not give to
      // exist (for a host that cannot tell directories).
      assumeDirs: () => x.conma_assume_dirs(),
      // For the pipe ends that hostProc.spawn gets: read up to len bytes without
      // waiting (a Uint8Array, empty at the end, or minus an errno: -35 when
      // there is nothing yet), write bytes (0 or minus an errno), close.
      read: (fd, len) => {
        const ptr = x.conma_alloc(len);
        try {
          const n = x.conma_read(fd, ptr, len);
          return n < 0 ? n : new Uint8Array(x.memory.buffer, ptr, n).slice();
        } finally {
          x.conma_free(ptr, len);
        }
      },
      write: (fd, bytes) => pass(bytes, (p, n) => -x.conma_write(fd, p, n)),
      close: (fd) => x.conma_close(fd),
      // A process exited: the VM polls the processes again.
      childEvent: () => x.conma_child_event(),
      // Use the file system of the host (hostFs) instead of the virtual one.
      useHostFs: () => x.conma_use_host_fs(),
      // A file whose contents fileNeeded gives when the VM opens it.
      putUnread: (path) => pass(path, x.conma_put_unread),
      setCwd: (path) => pass(path, x.conma_set_cwd),
      setenv: (key, value) => pass2(key, value, x.conma_setenv),
      // The bytes of a file of the virtual file system, or null.
      getFile: (path) => {
        const n = pass(path, x.conma_get_file);
        if (n < 0) return null;
        return new Uint8Array(x.memory.buffer, x.conma_result_ptr(), n).slice();
      },
    };
    return vm;
  }

  return { create, utf8, text, latin1, RUNNING: -1, IDLE: -2 };
})();

// Node.js (require) gets it as the module's value.
if (typeof module !== "undefined" && module.exports) module.exports = ConMaVM;
