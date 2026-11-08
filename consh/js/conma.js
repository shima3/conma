// The host side of the ConMa VM for wasm32 (consh/js/wasm). It works in a
// browser (<script src="conma.js">), in Node.js (require) and in the
// JavaScriptCore shell (load()).
//
//   const vm = ConMaVM.create(wasmModule, (fd, bytes) => ...);
//   vm.start(moduleBytes, ["-m", "consh.module.se", ...]);  // 0 if started
//   for (;;) {
//     const r = vm.run(100000);
//     if (r === ConMaVM.RUNNING) continue;     // let the host breathe, then go on
//     if (r === ConMaVM.IDLE) break;           // wait for input or Ctrl-C, then go on
//     ... r is the exit code ...
//   }
//
// wasmModule is a compiled WebAssembly.Module; write(fd, bytes) receives the
// bytes the VM writes to standard output (1) and standard error (2).
// inputNeeded(vm), if given, is called when a read of standard input that
// cannot wait (__OS_get_String__) finds it empty: it may call vm.input or
// vm.inputEnd at once (a browser cannot, and such a read sees the end).
// fileNeeded(vm, path), if given, is called when the VM looks for a file
// (an absolute path) that is not in the virtual file system: it may call
// vm.putFile (or vm.makeDirs for a directory) at once. dirNeeded(vm, path),
// if given, is called when the VM lists a directory for the first time: it
// may call vm.makeDirs and vm.putUnread for its entries at once.
// jsDone(vm), if given, is called when the result of a call of JavaScript
// (__JS_call__, the builtin js of ConSh) has been given to the VM: the host
// should run the VM again if it was IDLE. While vm.jsPending() > 0, the host
// must let its event loop run (Promises) when the VM is IDLE.
//
// __JS_call__ code args: code is evaluated as an expression in the global
// scope; if its value is a function, it is called with args (strings, and
// arrays of them), and the result (awaited if it is a Promise) is written to
// the pipe of __JS_call__ as one line, an S-expression that __Sexp_read__
// reads (see 2.1 and 2.2 of consh/js/PLAN-ja.md):
//   (r s "text")       a string, or a number, boolean, BigInt or object made
//                      into one (String(), JSON.stringify)
//   (r l ("a" ("b")))  an array: its elements, an array as a list, null and
//                      undefined as ""
//   (r n)              undefined or null: nothing
//   (e "message")      an exception: "Name: message" for an Error

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

  function evaluate(code, args) {
    const value = (0, eval)("(" + code + "\n)");
    if (typeof value === "function") return value(...args);
    if (args.length > 0) throw new HostError("arguments given to a value that is not a function");
    return value;
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

  function create(wasmModule, write, inputNeeded, fileNeeded, dirNeeded, jsDone) {
    let x = null;
    let vm = null;
    let jsPending = 0;
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
        host_js_call: (fd, codePtr, codeLen, argsPtr, argsLen) => {
          const code = text(new Uint8Array(x.memory.buffer, codePtr, codeLen));
          const args = JSON.parse(text(new Uint8Array(x.memory.buffer, argsPtr, argsLen)));
          jsPending++;
          // The result is given to the VM later (never inside conma_run, which is running now).
          let result;
          try {
            result = Promise.resolve(evaluate(code, args));
          } catch (e) {
            result = Promise.reject(e);
          }
          result
            .then(resultLine)
            .catch(errorLine)
            .then((line) => {
              pass(line, (p, n) => x.conma_write(fd, p, n));
              x.conma_close(fd);
              jsPending--;
              if (jsDone) jsDone(vm);
            });
        },
      },
    });
    x = instance.exports;

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
