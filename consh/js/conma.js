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

  // ── JS values of shell variables (2.5.4 of consh/js/PLAN-ja.md) ──
  // A shell variable (js name=, v=$(( ... ))) or an SJS variable holds a JS value that is not a
  // string only as an id: #n for an object or a function, kept in the table below, or the value
  // itself for a primitive (number:3, boolean:true, bigint:5, undefined, null), so that primitives
  // do not fill the table. When the variable is read with $, ConSh asks the host for it as a shell
  // value with __JS_value__ (host_js_value), so that a change of the object is seen. A line
  // (w "name" s "text") or (w "name" j "id") gives the value of a variable that the code assigned.
  // The JS statements of ConSh, which this served first, were removed in stage A3 of SJS (3.26).

  // js name= code: the result is given as a JS value, (r j "id"), unless it is a string or an
  // iterator (2.5 of consh/js/PLAN-ja.md)
  const KEEP = "//@consh-keep\n";
  const values = new Map(); // objects and functions of shell variables, by number
  let nextValue = 0;
  // The numbers of values that each call (__JS_call__, by its job) made. ConSh keeps the
  // outputStream of the call with the variables that hold them, and the VM closes it when it is
  // dropped (host_js_input_end): then no variable holds them and they are removed (releaseValues).
  const owned = new Map();
  // The id of a JS value (not a string), made by the call owner.
  function idOf(v, owner) {
    if (v === undefined || v === null) return String(v);
    const t = typeof v;
    if (t === "number" || t === "boolean" || t === "bigint") return t + ":" + String(v);
    values.set(++nextValue, v);
    if (!owned.has(owner)) owned.set(owner, []);
    owned.get(owner).push(nextValue);
    return "#" + nextValue;
  }
  function releaseValues(owner) {
    const ns = owned.get(owner);
    if (!ns) return;
    owned.delete(owner);
    for (const n of ns) values.delete(n);
  }
  function valueOf(id) {
    id = String(id);
    if (id.startsWith("#")) {
      const n = Number(id.slice(1));
      if (!values.has(n)) throw new HostError("no JS value " + id);
      return values.get(n);
    }
    if (id === "undefined") return undefined;
    if (id === "null") return null;
    const i = id.indexOf(":");
    const kind = id.slice(0, i);
    const text = id.slice(i + 1);
    if (kind === "number") return Number(text);
    if (kind === "boolean") return text === "true";
    if (kind === "bigint") return BigInt(text);
    throw new HostError("no JS value " + id);
  }
  // A JS value of a shell variable: a function of one is called by ConSh with
  // __consh_value(id) as the code of js.
  globalThis.__consh_value = valueOf;
  // The number of values in the table (for the tests).
  globalThis.__consh_value.count = () => values.size;

  const REGEX_AFTER = new Set(
    "return typeof instanceof in of new delete void throw case do else yield await".split(" ")
  );
  const PUNCTS = [">>>=", "...", "===", "!==", "**=", "<<=", ">>=", ">>>", "&&=", "||=", "??=", "=>", "==", "!=",
    "<=", ">=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "&&", "||", "??", "?.", "++", "--", "**", "<<", ">>"];

  // The tokens of JavaScript code: {type, text, start, end, depth}, type ws (with newlines),
  // comment, ident, num, str, tmpl (a piece of a template), regex or punct. depth is that of
  // the brackets around the token (an opening and its closing bracket have the depth outside).
  function jsTokens(src) {
    const toks = [];
    const stack = [];
    let i = 0;
    let reOk = true;
    const push = (type, start, depth) => toks.push({ type, text: src.slice(start, i), start, end: i, depth });
    // From inside a template to its end or to a ${; true for a ${
    const template = () => {
      while (i < src.length) {
        const c = src[i];
        if (c === "\\") i += 2;
        else if (c === "`") {
          i++;
          return false;
        } else if (c === "$" && src[i + 1] === "{") {
          i += 2;
          return true;
        } else i++;
      }
      return false;
    };
    while (i < src.length) {
      const s = i;
      const c = src[i];
      const depth = stack.length;
      if (/\s/.test(c)) {
        while (i < src.length && /\s/.test(src[i])) i++;
        push("ws", s, depth);
      } else if (c === "/" && src[i + 1] === "/") {
        while (i < src.length && src[i] !== "\n") i++;
        push("comment", s, depth);
      } else if (c === "/" && src[i + 1] === "*") {
        const e = src.indexOf("*/", i + 2);
        i = e < 0 ? src.length : e + 2;
        push("comment", s, depth);
      } else if (/[A-Za-z_$\u0080-\uffff]/.test(c)) {
        while (i < src.length && /[\w$\u0080-\uffff]/.test(src[i])) i++;
        push("ident", s, depth);
        reOk = REGEX_AFTER.has(src.slice(s, i));
      } else if (/[0-9]/.test(c) || (c === "." && /[0-9]/.test(src[i + 1] || ""))) {
        i++;
        while (i < src.length && /[\w.]/.test(src[i])) i++;
        push("num", s, depth);
        reOk = false;
      } else if (c === "'" || c === '"') {
        i++;
        while (i < src.length && src[i] !== c && src[i] !== "\n") i += src[i] === "\\" ? 2 : 1;
        if (src[i] === c) i++;
        push("str", s, depth);
        reOk = false;
      } else if (c === "`") {
        i++;
        if (template()) stack.push("T");
        push("tmpl", s, depth);
        reOk = true;
      } else if (c === "/" && reOk) {
        i++;
        let inClass = false;
        while (i < src.length && src[i] !== "\n") {
          const d = src[i];
          if (d === "\\") i++;
          else if (d === "[") inClass = true;
          else if (d === "]") inClass = false;
          else if (d === "/" && !inClass) break;
          i++;
        }
        i++;
        while (i < src.length && /[\w$]/.test(src[i])) i++;
        push("regex", s, depth);
        reOk = false;
      } else if ("([{".includes(c)) {
        i++;
        stack.push(c);
        push("punct", s, depth);
        reOk = true;
      } else if (")]}".includes(c)) {
        i++;
        const open = stack.pop();
        if (open === "T") {
          // The } of a ${ in a template: the rest of the template
          if (template()) stack.push("T");
          push("tmpl", s, stack.length);
          reOk = false;
        } else {
          push("punct", s, stack.length);
          reOk = c === "}";
        }
      } else {
        const p = PUNCTS.find((q) => src.startsWith(q, i)) || c;
        i += p.length;
        push("punct", s, depth);
        reOk = !(p === "++" || p === "--");
      }
    }
    return toks;
  }

  // A JS value as a shell value (__JS_value__).
  function shellValue(v) {
    if (v === undefined || v === null) return "n";
    if (typeof v === "function") return "f";
    try {
      return Array.isArray(v) ? "l " + sexpList(v) : "s " + sexpString(scalar(v));
    } catch (e) {
      return "s " + sexpString(String(v));
    }
  }

  // ── SJS: (( )) and $(( )) of ConSh (consh/SAFEJS-PLAN-ja.md) ──
  // ConSh calls __JS_call__ with SJS and the code of an SJS expression, and as args the SJS variables
  // that the code names, [names, kinds, values] (kinds s and j). The code is
  // first checked to be in SJS and rewritten (sjsCheck), then evaluated with only the SJS variables
  // and the builtins of SJS_BUILTINS as global names (sjsScope). Before the result, the host writes
  // (w ...) for each SJS variable that was assigned; the result is
  // (r s "text") for a string and (r j "id") for any other value, a Promise too (not waited for).
  // This is not a sandbox: the check keeps the code in SJS, so that it can later be translated into
  // ConMa, and closes the known ways out to the host, but that is not proved (5 of SAFEJS-PLAN-ja.md).
  const SJS = "//@consh-sjs\n";
  // An SJS function called as a command of ConSh: SJSCALL and its id, and the arguments after the
  // variables (2.2.3 of SAFEJS-PLAN-ja.md)
  const SJSCALL = "//@consh-sjscall\n";
  // A function of the host for SJS: its own frozen function, with the given static members, so that
  // SJS cannot change the host's (Number.parseInt = ...)
  function sjsFunction(f, statics) {
    const g = (...a) => f(...a);
    for (const k of statics) g[k] = f[k];
    return Object.freeze(g);
  }
  function sjsObject(o, keys) {
    const r = Object.create(null);
    for (const k of keys) r[k] = o[k];
    return Object.freeze(r);
  }
  const SJS_BUILTINS = Object.freeze(Object.assign(Object.create(null), {
    undefined: undefined,
    NaN: NaN,
    Infinity: Infinity,
    Math: sjsObject(Math, Object.getOwnPropertyNames(Math)),
    JSON: sjsObject(JSON, ["parse", "stringify"]),
    Number: sjsFunction(Number, ["isInteger", "isFinite", "isNaN", "isSafeInteger", "parseFloat", "parseInt",
      "MAX_SAFE_INTEGER", "MIN_SAFE_INTEGER", "MAX_VALUE", "MIN_VALUE", "EPSILON"]),
    String: sjsFunction(String, ["fromCharCode", "fromCodePoint", "raw"]),
    Boolean: sjsFunction(Boolean, []),
    Array: sjsFunction(Array, ["from", "of", "isArray"]),
    Object: sjsObject(Object, ["keys", "values", "entries", "fromEntries", "assign"]),
    parseInt: parseInt,
    parseFloat: parseFloat,
    isNaN: isNaN,
    isFinite: isFinite,
    console: Object.freeze({
      log: (...a) => console.log(...a),
      info: (...a) => console.info(...a),
      debug: (...a) => console.debug(...a),
      error: (...a) => console.error(...a),
      warn: (...a) => console.warn(...a),
    }),
  }));
  // The SJS variables of the evaluation that runs now: {vars, written, shell}; shell holds the shell
  // variables of the $names (2.2.7 of SAFEJS-PLAN-ja.md), by their names with the $. A function made by SJS reads
  // and writes the SJS variables of the evaluation that calls it, not of the one that made it
  // (2.2 of SAFEJS-PLAN-ja.md), since its global names are looked up here when it runs
  let sjsCurrent = null;
  const sjsScope = new Proxy(Object.create(null), {
    // Every name is looked up here (no global of the host), except the parameter __sjs
    has: (t, k) => typeof k === "string" && k !== "__sjs",
    get: (t, k) => {
      if (typeof k !== "string") return undefined;
      if (k.startsWith("$")) {
        if (sjsCurrent && Object.prototype.hasOwnProperty.call(sjsCurrent.shell, k)) return sjsCurrent.shell[k];
        throw new ReferenceError("undefined variable: " + k.slice(1));
      }
      if (k in SJS_BUILTINS) return SJS_BUILTINS[k];
      if (sjsCurrent && Object.prototype.hasOwnProperty.call(sjsCurrent.vars, k)) return sjsCurrent.vars[k];
      throw new ReferenceError(k + " is not defined");
    },
    set: (t, k, v) => {
      if (k in SJS_BUILTINS) throw new TypeError("cannot assign to " + String(k));
      if (String(k).startsWith("$")) throw new TypeError("cannot assign to the shell variable " + String(k));
      if (!sjsCurrent) throw new TypeError("an SJS variable assigned outside its evaluation: " + String(k));
      sjsCurrent.vars[k] = v;
      sjsCurrent.written.add(k);
      return true;
    },
  });
  // async and await are left out in stage A: after an await, the global names of a function could be
  // looked up in the SJS variables of another evaluation (sjsCurrent), and without them SJS makes no
  // Promise (SAFEJS-REVIEW-ja.md 3.1.1)
  const SJS_NAMES = new Set(("this new class super import export with eval arguments debugger yield async await " +
    "globalThis window self document Function").split(" "));
  const SJS_PROPERTIES = new Set(["constructor", "prototype", "caller", "callee", "arguments"]);
  function sjsBadProperty(s) {
    return SJS_PROPERTIES.has(s) || s.startsWith("__");
  }
  // The key of a computed member (a[k]): a number, or the string of k if it is not a property that
  // leads out of SJS (constructor ...). The string is returned, so that k is made a string only once.
  function sjsKey(k) {
    if (typeof k === "number") return k;
    if (typeof k === "symbol") throw new TypeError("a symbol as a key is not in SJS");
    const s = String(k);
    if (sjsBadProperty(s)) throw new TypeError("the property " + s + " is not in SJS");
    return s;
  }
  // The number of a shell variable $name next to an operator (2.2.7 of SAFEJS-PLAN-ja.md): a number
  // as it is, or a string in the form of a decimal number; anything else is an error, not NaN
  function sjsNumber(v, name) {
    if (typeof v === "number") return v;
    if (typeof v === "string" && /^[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?$/.test(v)) return Number(v);
    throw new TypeError("not a number: " + name + (typeof v === "string" ? " (" + JSON.stringify(v) + ")" : ""));
  }
  // The code, checked to be in SJS (3.2 of SAFEJS-PLAN-ja.md) and made again from its syntax tree
  // (sjsParse, sjsGen), so that the operators that need numbers check their operands at run time
  // (3.1), a computed member a[k] checks its key, and $name next to such an operator is a number
  // (2.2.7). Throws a HostError for code that is not in SJS.
  function sjsCheck(src) {
    const toks = [];
    let nl = false;
    let prev = null;
    for (const t of jsTokens(src)) {
      if (t.type === "ws" || t.type === "comment") {
        if (t.text.includes("\n")) nl = true;
        continue;
      }
      if (t.type === "ident") {
        const afterDot = prev !== null && prev.type === "punct" && (prev.text === "." || prev.text === "?.");
        if (afterDot ? sjsBadProperty(t.text) : SJS_NAMES.has(t.text) || t.text.startsWith("__")) {
          throw new HostError("not in SJS: " + (afterDot ? "." : "") + t.text);
        }
      } else if (t.type === "regex") {
        throw new HostError("not in SJS: a regular expression");
      } else if (t.type === "punct" && (t.text === "==" || t.text === "!=")) {
        throw new HostError("not in SJS: " + t.text + " (use " + t.text + "=)");
      }
      toks.push({ type: t.type, text: t.text, nl });
      nl = false;
      prev = t;
    }
    return sjsGen(sjsParse(toks));
  }

  // ── The syntax of SJS ──
  // A small recursive-descent parser of the expressions of JavaScript and of the statements in the
  // bodies of functions, enough for SJS (3.1 of SAFEJS-PLAN-ja.md). A node is {t: kind, ...}.
  const SJS_BINARY = new Map([
    ["??", 1], ["||", 2], ["&&", 3], ["|", 4], ["^", 5], ["&", 6], ["===", 7], ["!==", 7],
    ["<", 8], [">", 8], ["<=", 8], [">=", 8], ["in", 8], ["instanceof", 8],
    ["<<", 9], [">>", 9], [">>>", 9], ["+", 10], ["-", 10], ["*", 11], ["/", 11], ["%", 11], ["**", 12],
  ]);
  const SJS_ASSIGN = new Set(["=", "+=", "-=", "*=", "/=", "%=", "**=", "<<=", ">>=", ">>>=", "&=", "|=", "^=",
    "&&=", "||=", "??="]);
  const SJS_KEYWORDS = new Set(("break case catch const continue default delete do else false finally for function if in " +
    "instanceof let null return switch throw true try typeof var void while of").split(" "));
  function sjsParse(toks) {
    let i = 0;
    const peek = (k = 0) => toks[i + k] || { type: "eof", text: "", nl: true };
    const is = (text, k = 0) => {
      const t = peek(k);
      return (t.type === "punct" || t.type === "ident") && t.text === text;
    };
    const fail = (what) => {
      const t = peek();
      const msg = what || "unexpected " + (t.type === "eof" ? "end of the expression" : "'" + t.text + "'");
      throw new HostError(msg.startsWith("not in SJS") ? msg : "SyntaxError: " + msg);
    };
    const next = () => {
      const t = peek();
      if (t.type === "eof") fail();
      i++;
      return t;
    };
    const expect = (text) => {
      if (!is(text)) fail("expected '" + text + "' but found " + (peek().type === "eof" ? "the end" : "'" + peek().text + "'"));
      return next();
    };
    const eat = (text) => (is(text) ? (i++, true) : false);
    const isName = (t) => t.type === "ident" && !SJS_KEYWORDS.has(t.text);
    const name = () => {
      const t = peek();
      if (!isName(t)) fail("expected a name but found " + (t.type === "eof" ? "the end" : "'" + t.text + "'"));
      i++;
      return t.text;
    };
    // The index of the bracket that closes the one at k
    const closing = (k) => {
      let depth = 0;
      for (let m = k; m < toks.length; m++) {
        const t = toks[m];
        if (t.type === "punct" && "([{".includes(t.text)) depth++;
        else if (t.type === "punct" && ")]}".includes(t.text)) {
          depth--;
          if (depth === 0) return m;
        }
      }
      return -1;
    };
    // The end of a statement: ; or before }, the end, or a new line
    const semi = () => {
      if (eat(";")) return;
      if (is("}") || peek().type === "eof" || peek().nl) return;
      fail();
    };

    // Expressions
    const expression = (noIn) => {
      const e = assign(noIn);
      if (!is(",")) return e;
      const list = [e];
      while (eat(",")) list.push(assign(noIn));
      return { t: "seq", list };
    };
    const assign = (noIn) => {
      if (isName(peek()) && is("=>", 1)) {
        const p = { t: "id", name: next().text };
        next();
        return arrowBody([p]);
      }
      if (is("(")) {
        const close = closing(i);
        if (close >= 0 && toks[close + 1] && toks[close + 1].type === "punct" && toks[close + 1].text === "=>") {
          next();
          const params = paramList(")");
          expect("=>");
          return arrowBody(params);
        }
      }
      const left = conditional(noIn);
      const op = peek();
      if (op.type === "punct" && SJS_ASSIGN.has(op.text)) {
        next();
        const target = op.text === "=" ? toPattern(left) : simpleTarget(left);
        return { t: "assign", op: op.text, target, value: assign(noIn) };
      }
      return left;
    };
    const arrowBody = (params) => {
      if (is("{")) return { t: "arrow", params, body: block() };
      return { t: "arrow", params, expr: assign() };
    };
    const simpleTarget = (e) => {
      if (e.t === "id" || (e.t === "member" && !e.optional)) return e;
      fail("cannot assign to this expression");
    };
    // An expression on the left of = as a pattern
    const toPattern = (e) => {
      if (e.t === "id" || (e.t === "member" && !e.optional)) return e;
      if (e.t === "array") {
        return {
          t: "arrayPattern",
          items: e.items.map((x, k) => {
            if (x === null) return null;
            if (x.t === "spread") {
              if (k !== e.items.length - 1) fail("a rest element must be the last");
              return { t: "rest", target: toPattern(x.arg) };
            }
            return toPatternDefault(x);
          }),
        };
      }
      if (e.t === "object") {
        return {
          t: "objectPattern",
          props: e.props.map((p, k) => {
            if (p.t === "spread") {
              if (k !== e.props.length - 1) fail("a rest property must be the last");
              return { t: "rest", target: toPattern(p.arg) };
            }
            if (p.method) fail("a method in a pattern");
            return { key: p.key, computed: p.computed, value: toPatternDefault(p.value) };
          }),
        };
      }
      if (e.t === "paren") return toPattern(e.expr);
      fail("cannot assign to this expression");
    };
    const toPatternDefault = (x) => {
      if (x.t === "assign" && x.op === "=") return { t: "default", target: x.target, value: x.value };
      if (x.t === "shorthandDefault") return { t: "default", target: { t: "id", name: x.name }, value: x.value };
      return toPattern(x);
    };
    const conditional = (noIn) => {
      const test = binary(0, noIn);
      if (!eat("?")) return test;
      const yes = assign();
      expect(":");
      return { t: "cond", test, yes, no: assign(noIn) };
    };
    const binary = (min, noIn) => {
      let left = unary();
      for (;;) {
        const t = peek();
        if (t.type !== "punct" && t.type !== "ident") break;
        const prec = SJS_BINARY.get(t.text);
        if (prec === undefined || prec <= min || (noIn && t.text === "in")) break;
        if (t.type === "ident" && t.text !== "in" && t.text !== "instanceof") break;
        next();
        // ** is right-associative
        const right = t.text === "**" ? binary(prec - 1, noIn) : binary(prec, noIn);
        left = { t: "binary", op: t.text, left, right };
      }
      return left;
    };
    const unary = () => {
      const t = peek();
      if ((t.type === "punct" && ["!", "-", "+", "~"].includes(t.text)) || (t.type === "ident" && ["typeof", "void", "delete"].includes(t.text))) {
        next();
        return { t: "unary", op: t.text, arg: unary() };
      }
      if (is("++") || is("--")) {
        next();
        return { t: "update", op: t.text, prefix: true, target: simpleTarget(unary()) };
      }
      const e = postfix();
      if ((is("++") || is("--")) && !peek().nl) {
        return { t: "update", op: next().text, prefix: false, target: simpleTarget(e) };
      }
      return e;
    };
    const postfix = () => {
      let e = primary();
      for (;;) {
        if (eat(".")) {
          const t = next();
          if (t.type !== "ident") fail("expected a property name after '.'");
          e = { t: "member", object: e, name: t.text };
        } else if (eat("?.")) {
          if (eat("[")) {
            const key = expression();
            expect("]");
            e = { t: "member", object: e, key, optional: true };
          } else if (eat("(")) {
            e = { t: "call", callee: e, args: argList(), optional: true };
          } else {
            const t = next();
            if (t.type !== "ident") fail("expected a property name after '?.'");
            e = { t: "member", object: e, name: t.text, optional: true };
          }
        } else if (is("[")) {
          next();
          const key = expression();
          expect("]");
          e = { t: "member", object: e, key };
        } else if (is("(")) {
          next();
          e = { t: "call", callee: e, args: argList() };
        } else if (peek().type === "tmpl" && peek().text.startsWith("`")) {
          e = { t: "tagged", tag: e, quasi: template() };
        } else return e;
      }
    };
    const argList = () => {
      const args = [];
      while (!is(")")) {
        if (eat("...")) args.push({ t: "spread", arg: assign() });
        else args.push(assign());
        if (!eat(",")) break;
      }
      expect(")");
      return args;
    };
    const template = () => {
      const parts = [];
      const exprs = [];
      let t = next();
      for (;;) {
        const text = t.text;
        // A piece begins with ` or with the } of a ${, and ends with ${ or `
        if (text.endsWith("${")) {
          parts.push(text.slice(1, -2));
          exprs.push(expression());
          t = next();
          if (t.type !== "tmpl") fail("expected the rest of a template");
          continue;
        }
        parts.push(text.slice(1, -1));
        return { t: "template", parts, exprs };
      }
    };
    const primary = () => {
      const t = peek();
      if (t.type === "num") return next(), { t: "num", text: t.text };
      if (t.type === "str") return next(), { t: "str", text: t.text };
      if (t.type === "tmpl") {
        if (!t.text.startsWith("`")) fail();
        return template();
      }
      if (t.type === "ident") {
        if (["true", "false", "null"].includes(t.text)) return next(), { t: "lit", text: t.text };
        if (t.text === "function") return functionExpr();
        return { t: "id", name: name() };
      }
      if (eat("(")) {
        const e = expression();
        expect(")");
        return { t: "paren", expr: e };
      }
      if (eat("[")) {
        const items = [];
        while (!is("]")) {
          if (is(",")) {
            next();
            items.push(null);
            continue;
          }
          items.push(eat("...") ? { t: "spread", arg: assign() } : assign());
          if (!eat(",")) break;
        }
        expect("]");
        return { t: "array", items };
      }
      if (eat("{")) {
        const props = [];
        while (!is("}")) {
          props.push(property());
          if (!eat(",")) break;
        }
        expect("}");
        return { t: "object", props };
      }
      fail();
    };
    const propertyKey = () => {
      const t = peek();
      if (eat("[")) {
        const key = assign();
        expect("]");
        return { computed: true, key };
      }
      if (t.type === "ident" || t.type === "str" || t.type === "num") {
        next();
        if (t.type === "str" && (t.text.slice(1, -1) === "__proto__")) fail("not in SJS: the property __proto__");
        return { computed: false, key: t };
      }
      fail();
    };
    const property = () => {
      if (eat("...")) return { t: "spread", arg: assign() };
      const t = peek();
      if (t.type === "ident" && (t.text === "get" || t.text === "set") && !is(",", 1) && !is(":", 1) && !is("(", 1) && !is("}", 1)) {
        fail("not in SJS: a getter or a setter");
      }
      const { computed, key } = propertyKey();
      if (eat(":")) return { computed, key, value: assign() };
      if (is("(")) {
        next();
        const params = paramList(")");
        return { computed, key, method: true, value: { t: "function", name: null, params, body: block() } };
      }
      if (computed || key.type !== "ident" || SJS_KEYWORDS.has(key.text)) fail();
      if (eat("=")) return { computed, key, value: { t: "shorthandDefault", name: key.text, value: assign() } };
      return { computed, key, value: { t: "id", name: key.text }, shorthand: true };
    };
    const functionExpr = () => {
      expect("function");
      if (is("*")) fail("not in SJS: a generator");
      const fname = isName(peek()) ? name() : null;
      expect("(");
      const params = paramList(")");
      return { t: "function", name: fname, params, body: block() };
    };
    // Parameters up to close (after the opening bracket)
    const paramList = (close) => {
      const params = [];
      while (!is(close)) {
        if (eat("...")) {
          params.push({ t: "rest", target: bindingTarget() });
          break;
        }
        params.push(bindingElement());
        if (!eat(",")) break;
      }
      expect(close);
      return params;
    };
    const bindingElement = () => {
      const target = bindingTarget();
      if (eat("=")) return { t: "default", target, value: assign() };
      return target;
    };
    const bindingTarget = () => {
      if (eat("[")) {
        const items = [];
        while (!is("]")) {
          if (is(",")) {
            next();
            items.push(null);
            continue;
          }
          if (eat("...")) {
            items.push({ t: "rest", target: bindingTarget() });
            break;
          }
          items.push(bindingElement());
          if (!eat(",")) break;
        }
        expect("]");
        return { t: "arrayPattern", items };
      }
      if (eat("{")) {
        const props = [];
        while (!is("}")) {
          if (eat("...")) {
            props.push({ t: "rest", target: { t: "id", name: name() } });
            break;
          }
          const { computed, key } = propertyKey();
          if (eat(":")) props.push({ key, computed, value: bindingElement() });
          else {
            if (computed || key.type !== "ident") fail();
            const target = { t: "id", name: key.text };
            props.push({ key, computed, value: eat("=") ? { t: "default", target, value: assign() } : target, shorthand: true });
          }
          if (!eat(",")) break;
        }
        expect("}");
        return { t: "objectPattern", props };
      }
      const n = name();
      if (n.startsWith("$")) fail("not in SJS: a declaration of " + n + " (a name that begins with $ is a shell variable)");
      return { t: "id", name: n };
    };

    // Statements (in the bodies of functions)
    const block = () => {
      expect("{");
      const body = [];
      while (!is("}")) body.push(statement());
      expect("}");
      return { t: "block", body };
    };
    const declaration = (noIn) => {
      const kind = next().text;
      const decls = [];
      do {
        const target = bindingTarget();
        decls.push({ target, init: eat("=") ? assign(noIn) : null });
      } while (eat(","));
      return { t: "decl", kind, decls };
    };
    const statement = () => {
      const t = peek();
      if (is("{")) return block();
      if (eat(";")) return { t: "empty" };
      if (t.type === "ident") {
        switch (t.text) {
          case "let":
          case "const":
          case "var": {
            const d = declaration(false);
            semi();
            return d;
          }
          case "function": {
            const f = functionExpr();
            if (!f.name) fail("expected the name of a function");
            return { t: "fundecl", fn: f };
          }
          case "if": {
            next();
            expect("(");
            const test = expression();
            expect(")");
            const yes = statement();
            return { t: "if", test, yes, no: eat("else") ? statement() : null };
          }
          case "while": {
            next();
            expect("(");
            const test = expression();
            expect(")");
            return { t: "while", test, body: statement() };
          }
          case "do": {
            next();
            const body = statement();
            expect("while");
            expect("(");
            const test = expression();
            expect(")");
            eat(";");
            return { t: "dowhile", body, test };
          }
          case "for":
            return forStatement();
          case "break":
          case "continue": {
            next();
            semi();
            return { t: t.text };
          }
          case "return": {
            next();
            const value = is(";") || is("}") || peek().type === "eof" || peek().nl ? null : expression();
            semi();
            return { t: "return", value };
          }
          case "throw": {
            next();
            if (peek().nl) fail("a new line after throw");
            const value = expression();
            semi();
            return { t: "throw", value };
          }
          case "try": {
            next();
            const body = block();
            let param = null;
            let handler = null;
            let finalizer = null;
            if (eat("catch")) {
              if (eat("(")) {
                param = bindingTarget();
                expect(")");
              }
              handler = block();
            }
            if (eat("finally")) finalizer = block();
            if (!handler && !finalizer) fail("expected catch or finally");
            return { t: "try", body, param, handler, finalizer };
          }
          case "switch": {
            next();
            expect("(");
            const disc = expression();
            expect(")");
            expect("{");
            const cases = [];
            while (!is("}")) {
              let test = null;
              if (eat("default")) expect(":");
              else {
                expect("case");
                test = expression();
                expect(":");
              }
              const body = [];
              while (!is("case") && !is("default") && !is("}")) body.push(statement());
              cases.push({ test, body });
            }
            expect("}");
            return { t: "switch", disc, cases };
          }
        }
      }
      const e = expression();
      semi();
      return { t: "expr", expr: e };
    };
    const forStatement = () => {
      expect("for");
      expect("(");
      let init = null;
      if (is("let") || is("const") || is("var")) {
        const kind = peek().text;
        if ((is("of", 2) || is("in", 2)) && isName(peek(1))) {
          next();
          const target = bindingTarget();
          const of = next().text;
          const right = of === "of" ? assign() : expression();
          expect(")");
          return { t: "forof", of, kind, target, right, body: statement() };
        }
        // A pattern before of or in
        if (is("[", 1) || is("{", 1)) {
          const save = i;
          next();
          const target = bindingTarget();
          if (is("of") || is("in")) {
            const of = next().text;
            const right = of === "of" ? assign() : expression();
            expect(")");
            return { t: "forof", of, kind, target, right, body: statement() };
          }
          i = save;
        }
        init = declaration(true);
      } else if (!is(";")) {
        if (isName(peek()) && (is("of", 1) || is("in", 1))) {
          const target = { t: "id", name: next().text };
          const of = next().text;
          const right = of === "of" ? assign() : expression();
          expect(")");
          return { t: "forof", of, kind: null, target, right, body: statement() };
        }
        init = { t: "expr", expr: expression(true) };
      }
      expect(";");
      const test = is(";") ? null : expression();
      expect(";");
      const update = is(")") ? null : expression();
      expect(")");
      return { t: "for", init, test, update, body: statement() };
    };

    const e = expression();
    if (peek().type !== "eof") fail();
    return e;
  }

  // ── Making the code again ──
  // The operators that need numbers (2.2.7 and 3.1 of SAFEJS-PLAN-ja.md), by the names of the
  // functions of __sjs (SJS_RUNTIME) that check them
  const SJS_CHECKED = new Map([
    ["+", "add"], ["-", "sub"], ["*", "mul"], ["/", "div"], ["%", "mod"], ["**", "pow"],
    ["<<", "shl"], [">>", "shr"], [">>>", "ushr"], ["&", "band"], ["|", "bor"], ["^", "bxor"],
    ["<", "lt"], ["<=", "le"], [">", "gt"], [">=", "ge"],
  ]);
  const SJS_CHECKED_UNARY = new Map([["-", "neg"], ["+", "pos"], ["~", "bnot"]]);
  function sjsGen(node) {
    const unparen = (e) => (e.t === "paren" ? unparen(e.expr) : e);
    const isShell = (e) => {
      const u = unparen(e);
      return u.t === "id" && u.name.startsWith("$");
    };
    // An operand of an operator that needs numbers: $name is a number
    const operand = (e) => {
      if (isShell(e)) {
        const n = unparen(e).name;
        return "__sjs.num(" + n + ", " + JSON.stringify(n) + ")";
      }
      return gen(e);
    };
    const isNumLiteral = (e) => {
      const u = unparen(e);
      return u.t === "num" || (u.t === "unary" && u.op === "-" && unparen(u.arg).t === "num");
    };
    // A key of a member: a string literal for .name, or the checked key
    const memberKey = (m) => (m.name !== undefined ? JSON.stringify(m.name) : "__sjs.key(" + gen(m.key) + ")");
    const chain = (e) => (["member", "call", "id", "paren", "array", "object", "template", "str", "lit", "tagged"].includes(e.t) ? gen(e) : "(" + gen(e) + ")");
    const params = (ps) => "(" + ps.map(pattern).join(", ") + ")";
    const pattern = (p) => {
      if (p === null) return "";
      switch (p.t) {
        case "id":
          return p.name;
        case "member":
          return gen(p);
        case "default":
          return pattern(p.target) + " = " + gen(p.value);
        case "rest":
          return "..." + pattern(p.target);
        case "arrayPattern":
          return "[" + p.items.map(pattern).join(", ") + (p.items.length && p.items[p.items.length - 1] === null ? "," : "") + "]";
        case "objectPattern":
          return "{" + p.props.map((q) => (q.t === "rest" ? "..." + pattern(q.target) : propKey(q) + ": " + pattern(q.value))).join(", ") + "}";
      }
      throw new HostError("SJS: a pattern of " + p.t);
    };
    const propKey = (q) => (q.computed ? "[" + gen(q.key) + "]" : q.key.text);
    const stmt = (s) => {
      switch (s.t) {
        case "block":
          return "{" + s.body.map(stmt).join(" ") + "}";
        case "empty":
          return ";";
        case "expr":
          return "(" + gen(s.expr) + ");";
        case "decl":
          return s.kind + " " + s.decls.map((d) => pattern(d.target) + (d.init ? " = " + gen(d.init) : "")).join(", ") + ";";
        case "fundecl":
          return "function " + s.fn.name + params(s.fn.params) + " " + stmt(s.fn.body);
        case "if":
          return "if (" + gen(s.test) + ") " + stmt(s.yes) + (s.no ? " else " + stmt(s.no) : "");
        case "while":
          return "while (" + gen(s.test) + ") " + stmt(s.body);
        case "dowhile":
          return "do " + stmt(s.body) + " while (" + gen(s.test) + ");";
        case "for":
          return "for (" + (s.init ? (s.init.t === "decl" ? stmt(s.init).slice(0, -1) : gen(s.init.expr)) : "") + "; " +
            (s.test ? gen(s.test) : "") + "; " + (s.update ? gen(s.update) : "") + ") " + stmt(s.body);
        case "forof":
          return "for (" + (s.kind ? s.kind + " " : "") + pattern(s.target) + " " + s.of + " " + gen(s.right) + ") " + stmt(s.body);
        case "break":
        case "continue":
          return s.t + ";";
        case "return":
          return "return" + (s.value ? " (" + gen(s.value) + ")" : "") + ";";
        case "throw":
          return "throw (" + gen(s.value) + ");";
        case "try":
          return "try " + stmt(s.body) + (s.handler ? " catch" + (s.param ? " (" + pattern(s.param) + ")" : "") + " " + stmt(s.handler) : "") +
            (s.finalizer ? " finally " + stmt(s.finalizer) : "");
        case "switch":
          return "switch (" + gen(s.disc) + ") {" + s.cases.map((c) => (c.test ? "case " + gen(c.test) + ":" : "default:") + " " + c.body.map(stmt).join(" ")).join(" ") + "}";
      }
      throw new HostError("SJS: a statement of " + s.t);
    };
    const gen = (e) => {
      switch (e.t) {
        case "num":
          return "(" + e.text + ")";
        case "str":
        case "lit":
          return e.text;
        case "id":
          return e.name;
        case "paren":
          return "(" + gen(e.expr) + ")";
        case "seq":
          return "(" + e.list.map(gen).join(", ") + ")";
        case "template":
          return "`" + e.parts.map((p, k) => p + (k < e.exprs.length ? "${" + gen(e.exprs[k]) + "}" : "")).join("") + "`";
        case "tagged":
          return chain(e.tag) + gen(e.quasi);
        case "array":
          return "[" + e.items.map((x) => (x === null ? "" : x.t === "spread" ? "..." + gen(x.arg) : gen(x))).join(", ") +
            (e.items.length && e.items[e.items.length - 1] === null ? "," : "") + "]";
        case "object":
          return "({" + e.props.map((q) => {
            if (q.t === "spread") return "..." + gen(q.arg);
            if (q.method) return propKey(q) + params(q.value.params) + " " + stmt(q.value.body);
            if (q.value.t === "shorthandDefault") throw new HostError("SJS: unexpected '=' in an object");
            return propKey(q) + ": " + gen(q.value);
          }).join(", ") + "})";
        case "spread":
          return "..." + gen(e.arg);
        case "member":
          return chain(e.object) + (e.optional ? "?." : "") + (e.name !== undefined ? (e.optional ? "" : ".") + e.name : "[" + memberKey(e) + "]");
        case "call":
          return chain(e.callee) + (e.optional ? "?.(" : "(") + e.args.map(gen).join(", ") + ")";
        case "function":
          return "(function " + (e.name || "") + params(e.params) + " " + stmt(e.body) + ")";
        case "arrow":
          return "(" + params(e.params) + " => " + (e.body ? stmt(e.body) : "(" + gen(e.expr) + ")") + ")";
        case "cond":
          return "(" + gen(e.test) + " ? " + gen(e.yes) + " : " + gen(e.no) + ")";
        case "unary": {
          const f = SJS_CHECKED_UNARY.get(e.op);
          if (f) return "__sjs." + f + "(" + operand(e.arg) + ")";
          if (e.op === "delete") return "(delete " + gen(e.arg) + ")";
          return "(" + e.op + (/[a-z]/.test(e.op) ? " " : "") + gen(e.arg) + ")";
        }
        case "binary": {
          const f = SJS_CHECKED.get(e.op);
          if (f) return "__sjs." + f + "(" + operand(e.left) + ", " + operand(e.right) + ")";
          if (e.op === "===" || e.op === "!==") {
            // $name compared with a number literal is a number
            const l = isShell(e.left) && isNumLiteral(e.right) ? operand(e.left) : gen(e.left);
            const r = isShell(e.right) && isNumLiteral(e.left) ? operand(e.right) : gen(e.right);
            return "(" + l + " " + e.op + " " + r + ")";
          }
          return "(" + gen(e.left) + " " + e.op + " " + gen(e.right) + ")";
        }
        case "assign": {
          const f = SJS_CHECKED.get(e.op.slice(0, -1));
          if (!f) return "(" + pattern(e.target) + " " + e.op + " " + gen(e.value) + ")";
          // a op= v: the operator checks its operands; a member is read and written once
          if (e.target.t === "id") return "(" + e.target.name + " = __sjs." + f + "(" + operand(e.target) + ", " + operand(e.value) + "))";
          return "__sjs.setOp(" + chain(e.target.object) + ", " + memberKey(e.target) + ", " + JSON.stringify(f) + ", " + operand(e.value) + ")";
        }
        case "update": {
          const d = e.op === "++" ? "1" : "-1";
          if (e.target.t === "id") {
            const n = e.target.name;
            if (e.prefix) return "(" + n + " = __sjs.step(" + n + ", " + d + "))";
            return "__sjs.post(" + n + ", (__sjs_v) => " + n + " = __sjs_v, " + d + ")";
          }
          return "__sjs.stepMember(" + chain(e.target.object) + ", " + memberKey(e.target) + ", " + d + ", " + e.prefix + ")";
        }
        case "shorthandDefault":
          throw new HostError("SJS: unexpected '='");
      }
      throw new HostError("SJS: an expression of " + e.t);
    };
    return gen(node);
  }
  // The functions of __sjs, which the code made by sjsGen calls: the operators that need numbers
  // (3.1 of SAFEJS-PLAN-ja.md). + adds only numbers, and the ordering comparisons take two numbers or
  // two strings; anything else is an error, so that code that runs without an error in SJS gives the
  // same result in JavaScript.
  function sjsDescribe(v) {
    if (typeof v === "string") return "the string " + JSON.stringify(v.length > 20 ? v.slice(0, 20) + "..." : v);
    if (v === null) return "null";
    if (Array.isArray(v)) return "an array";
    if (typeof v === "object") return "an object";
    if (typeof v === "function") return "a function";
    return typeof v === "undefined" ? "undefined" : "the " + typeof v + " " + String(v);
  }
  function sjsNeed(op, a, b) {
    for (const v of b === undefined && arguments.length < 3 ? [a] : [a, b]) {
      if (typeof v !== "number") {
        throw new TypeError(op + " needs numbers, not " + sjsDescribe(v) + (op === "+" || op === "+=" ? " (join strings with a template literal)" : ""));
      }
    }
  }
  function sjsCompare(op, a, b) {
    if ((typeof a === "number" && typeof b === "number") || (typeof a === "string" && typeof b === "string")) return;
    throw new TypeError(op + " compares two numbers or two strings, not " + sjsDescribe(a) + " and " + sjsDescribe(b));
  }
  const SJS_OPS = {
    add: (a, b) => (sjsNeed("+", a, b), a + b),
    sub: (a, b) => (sjsNeed("-", a, b), a - b),
    mul: (a, b) => (sjsNeed("*", a, b), a * b),
    div: (a, b) => (sjsNeed("/", a, b), a / b),
    mod: (a, b) => (sjsNeed("%", a, b), a % b),
    pow: (a, b) => (sjsNeed("**", a, b), a ** b),
    shl: (a, b) => (sjsNeed("<<", a, b), a << b),
    shr: (a, b) => (sjsNeed(">>", a, b), a >> b),
    ushr: (a, b) => (sjsNeed(">>>", a, b), a >>> b),
    band: (a, b) => (sjsNeed("&", a, b), a & b),
    bor: (a, b) => (sjsNeed("|", a, b), a | b),
    bxor: (a, b) => (sjsNeed("^", a, b), a ^ b),
    lt: (a, b) => (sjsCompare("<", a, b), a < b),
    le: (a, b) => (sjsCompare("<=", a, b), a <= b),
    gt: (a, b) => (sjsCompare(">", a, b), a > b),
    ge: (a, b) => (sjsCompare(">=", a, b), a >= b),
    neg: (v) => (sjsNeed("-", v), -v),
    pos: (v) => (sjsNeed("+", v), +v),
    bnot: (v) => (sjsNeed("~", v), ~v),
  };
  const SJS_RUNTIME = Object.freeze(Object.assign(Object.create(null), SJS_OPS, {
    key: sjsKey,
    num: sjsNumber,
    // ++x and --x of a variable
    step: (v, d) => (sjsNeed(d > 0 ? "++" : "--", v), v + d),
    // x++ and x-- of a variable: set is the assignment
    post: (v, set, d) => (sjsNeed(d > 0 ? "++" : "--", v), set(v + d), v),
    // o[k] op= v, reading and writing o[k] once
    setOp: (o, k, f, v) => (o[k] = SJS_OPS[f](o[k], v)),
    // ++o[k], o[k]++ and the same with --
    stepMember: (o, k, d, prefix) => {
      const v = o[k];
      sjsNeed(d > 0 ? "++" : "--", v);
      o[k] = v + d;
      return prefix ? v + d : v;
    },
  }));
  const sjsCompiled = new Map();
  // Evaluate an SJS expression (see above). The result is {sjs: value}, so that a value is never
  // taken for a Promise.
  function evaluateSjs(src, args, emit, owner) {
    const code = sjsCheck(src);
    let fn = sjsCompiled.get(code);
    if (!fn) {
      fn = new Function("__sjs_scope", "__sjs", "with (__sjs_scope) { return (\n" + code + "\n) }");
      sjsCompiled.set(code, fn);
    }
    return sjsRun(args, emit, owner, () => fn(sjsScope, SJS_RUNTIME));
  }
  // Call the SJS function of id with the arguments after the variables in args (SJSCALL).
  function callSjs(id, args, emit, owner) {
    const f = valueOf(id);
    if (typeof f !== "function") throw new HostError("not a function");
    return sjsRun(args, emit, owner, () => f(...args[6]));
  }
  // Run thunk with the SJS variables and the shell variables of args as those of the evaluation
  // (sjsCurrent), then write (w ...) for the SJS variables that were assigned.
  function sjsRun(args, emit, owner, thunk) {
    const [names, kinds, vals, shellNames, shellKinds, shellVals] = args;
    const vars = Object.create(null);
    names.forEach((name, k) => {
      vars[name] = kinds[k] === "j" ? valueOf(vals[k]) : vals[k];
    });
    const shell = Object.create(null);
    shellNames.forEach((name, k) => {
      shell[name] = shellKinds[k] === "j" ? valueOf(shellVals[k]) : shellVals[k];
    });
    const frame = { vars, written: new Set(), shell };
    const outer = sjsCurrent;
    sjsCurrent = frame;
    try {
      return { sjs: thunk() };
    } finally {
      sjsCurrent = outer;
      // The variables assigned before an error are kept, as in a shell
      for (const name of frame.written) {
        const v = vars[name];
        emit("(w " + sexpString(name) + (typeof v === "string" ? " s " + sexpString(v) : ' j "' + idOf(v, owner) + '"') + ")\n");
      }
    }
  }

  // A String in double quotes, as the VM writes the elements of standard input.
  function unquote(line) {
    const m = /^\s*"((?:[^"\\]|\\.)*)"\s*$/s.exec(line);
    if (!m) return line;
    return m[1].replace(/\\(.)/gs, (_, c) => ({ n: "\n", t: "\t", r: "\r" })[c] || c);
  }

  const END = {};

  // A call of JavaScript: the pipe fd its lines go to, and its standard input.
  function Job(id, emit) {
    this.id = id;
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

  // The JavaScript of __JS_call__, apart from the VM, so that it can run in another thread than the
  // VM (the page, while the VM runs in a Worker: consh/js/web): calls are known by their job numbers.
  // out.line(job, line) writes a line to the pipe of the call and returns 0 or an errno (32 when the
  // reader has gone), and out.end(job) closes it. write(fd, bytes) is the standard output (1) and
  // error (2) of the host, for console when no call can take it.
  function JsEngine(out, write) {
    const jobs = new Map();
    installConsole(write);
    function call(id, code, args) {
      const keep = code.startsWith(KEEP);
      const sjs = code.startsWith(SJS) || code.startsWith(SJSCALL);
      // Lines go to the pipe of the call at once, and the VM runs again after them.
      const job = new Job(id, (line) => {
        if (job.ended) return;
        const errno = out.line(id, line);
        if (errno === 32 && job.onClosed) job.onClosed(); // EPIPE: the reader has gone
      });
      // End the call, writing line (if not null) as its last.
      const finish = (line) => {
        if (job.ended) return;
        if (line !== null) job.emit(line);
        job.ended = true;
        liveJobs.delete(job);
        jobs.delete(id);
        out.end(id);
        for (const w of job.waiters.splice(0)) w(END);
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
        result = Promise.resolve(
          code.startsWith(SJS)
            ? evaluateSjs(code.slice(SJS.length), args, (line) => job.emit(line), id)
            : code.startsWith(SJSCALL)
              ? callSjs(code.slice(SJSCALL.length), args, (line) => job.emit(line), id)
              : evaluate(keep ? code.slice(KEEP.length) : code, args, makeStdin(job))
        );
      } catch (e) {
        result = Promise.reject(e);
      } finally {
        consoleJob = null;
      }
      result
        .then((v) => {
          if (sjs) {
            const x = v.sjs;
            return typeof x === "string" ? "(r s " + sexpString(x) + ")\n" : '(r j "' + idOf(x, id) + '")\n';
          }
          if (keep && typeof v !== "string" && !isIterator(v)) return '(r j "' + idOf(v, id) + '")\n';
          if (!isIterator(v)) return resultLine(v);
          iterate(v);
          return undefined;
        })
        .catch(errorLine)
        .then((line) => {
          if (line !== undefined) finish(line);
        });
    }

    return {
      // Start a call: evaluate code (a string) with args (from JSON)
      call,
      // The VM wrote to the call's standard input, or closed it (then the values of the call are
      // forgotten, see owned)
      input: (id, data) => {
        const job = jobs.get(id);
        if (job) job.input(data);
      },
      inputEnd: (id) => {
        const job = jobs.get(id);
        if (job) job.closed();
        releaseValues(id);
      },
      // The reader of the call's pipe has gone (for a call in another thread than the VM)
      readerGone: (id) => {
        const job = jobs.get(id);
        if (job && job.onClosed) job.onClosed();
      },
      // The value of id (__JS_value__) as the text of an S-expression; throws when there is none
      value: (id) => "(" + shellValue(valueOf(id)) + ")",
    };
  }

  function create(wasmModule, write, hooks) {
    const { inputNeeded, fileNeeded, dirNeeded, jsDone, hostFs, hostProc, trapInterrupt, buildModule, jsRemote } = hooks || {};
    // The last result of buildModule, until the VM has taken it all (host_build_module)
    let lastBuild = null;
    let x = null;
    let vm = null;
    let jsPending = 0;
    // Run the VM again (later: this may be inside conma_run).
    const wake = () => {
      if (jsDone) Promise.resolve().then(() => jsDone(vm));
    };
    // The calls of JavaScript (__JS_call__): the fd of the pipe of each job, and their lines
    const jsFds = new Map();
    const jsOut = {
      line: (id, line) => {
        const fd = jsFds.get(id);
        if (fd === undefined) return 9; // EBADF: the call has ended
        const errno = pass(line, (p, n) => x.conma_write(fd, p, n));
        wake();
        return errno;
      },
      end: (id) => {
        const fd = jsFds.get(id);
        if (fd === undefined) return;
        jsFds.delete(id);
        x.conma_close(fd);
        jsPending--;
        wake();
      },
    };
    // The JavaScript of the calls: here (JsEngine), or jsRemote, which gives them to an engine in
    // another thread ({call, input, inputEnd, value}: value returns the text at once, or an errno)
    const js = jsRemote || JsEngine(jsOut, write);
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
        host_js_value: (idPtr, idLen, out, cap) => {
          let text;
          try {
            text = js.value(str(idPtr, idLen));
          } catch (e) {
            return -22; // EINVAL: no such value
          }
          return typeof text === "number" ? text : give(text, out, cap);
        },
        host_js_call: (fd, id, codePtr, codeLen, argsPtr, argsLen) => {
          const code = str(codePtr, codeLen);
          const args = JSON.parse(str(argsPtr, argsLen));
          jsFds.set(id, fd);
          jsPending++;
          js.call(id, code, args);
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
        host_js_input: (id, ptr, len) => js.input(id, str(ptr, len)),
        host_js_input_end: (id) => js.inputEnd(id),
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
      // For jsRemote: a line of a call from the engine (0 or an errno, 32 when the reader has gone),
      // and the end of the call.
      jsLine: (id, line) => jsOut.line(id, line),
      jsEnd: (id) => jsOut.end(id),
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

  return { create, JsEngine, utf8, text, latin1, RUNNING: -1, IDLE: -2 };
})();

// Node.js (require) gets it as the module's value.
if (typeof module !== "undefined" && module.exports) module.exports = ConMaVM;
