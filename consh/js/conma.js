// The host side of the ConMa VM for wasm32 (consh/js/wasm). It works in a
// browser (<script src="conma.js">) and in the JavaScriptCore shell (load()).
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
// vm.putFile at once.

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

  function create(wasmModule, write, inputNeeded, fileNeeded) {
    let x = null;
    let vm = null;
    const instance = new WebAssembly.Instance(wasmModule, {
      env: {
        host_write: (fd, ptr, len) => write(fd, new Uint8Array(x.memory.buffer, ptr, len).slice()),
        host_input_needed: () => {
          if (inputNeeded) inputNeeded(vm);
        },
        host_file_needed: (ptr, len) => {
          if (fileNeeded) fileNeeded(vm, text(new Uint8Array(x.memory.buffer, ptr, len)));
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
      // Whether a VProc found standard input empty since the last call.
      wantsInput: () => x.conma_wants_input() !== 0,
      interrupt: () => x.conma_interrupt(),
      // These return 0, or the errno of the error.
      putFile: (path, data) => pass2(path, data, x.conma_put_file),
      makeDirs: (path) => pass(path, x.conma_make_dirs),
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
