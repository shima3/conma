// ConSh in a browser: the ConMa VM for wasm32 (conma.wasm, built by consh/js/build.sh) runs the ConMa
// version of ConSh (consh.module.se) in interactive mode, as `consh` with no arguments does. Each line
// typed is given to its standard input; standard output and standard error are shown in the page.
//
// The files are those of the server (serverfs.js): /home/user, the current directory, is the
// directory of the page (consh/js/web), whose files are read from the server each time, and the files
// written (and those that "ファイルを置く" puts) are kept in memory over them until the page is closed.
// "スクリプトを実行" gives the lines of a script to standard input, as if they were typed (the ConMa
// version of ConSh has no source).
//
// The commands of COMMANDS, written in JavaScript (cat, pwd, ls), are builtin commands of ConSh: the page
// names them in CONSH_HOST_BUILTINS, and ConSh starts each as the process builtin:<name> (hostProc),
// which reads and writes the standard input, output and error that ConSh gives it, as a real process
// does. There are no external commands. While one reads the terminal, the lines typed go to it instead of
// ConSh, Ctrl+D ends only its input, and Ctrl+C stops it (and ConSh gets the interrupt).
(async function () {
  "use strict";
  const out = document.getElementById("out");
  const term = document.getElementById("term");
  const line = document.getElementById("line");
  const status = document.getElementById("status");
  // The directory of the page is HOME, the home and current directory of ConSh: /home/user, or the
  // absolute path given as ?home= (?home=/ makes it the root)
  const HOME = ServerFS.normalize(new URLSearchParams(location.search).get("home") || "/home/user") || "/home/user";
  const fs = ServerFS.create(HOME);
  const pathOf = (cwd, name) => (name.startsWith("/") ? name : cwd.replace(/\/$/, "") + "/" + name);

  function show(text, cls) {
    if (text === "") return;
    const last = out.lastChild;
    if (last && last.className === (cls || "")) {
      last.textContent += text;
    } else {
      const span = document.createElement("span");
      if (cls) span.className = cls;
      span.textContent = text;
      out.appendChild(span);
    }
    term.scrollTop = term.scrollHeight;
  }

  // ── Processes written in JavaScript (hostProc of conma.js) ──
  // A command is an async function (io, args) that returns the exit status; io.read() gives the next
  // bytes of standard input (null at the end), io.write(i, bytes) writes to standard output (1) or
  // error (2) and returns false when the reader has gone (the command then returns -13, SIGPIPE).
  const COMMANDS = {
    // cat [-u] [file ...]: the files, or standard input for - and with no file, to standard output
    cat: async (io, args) => {
      let files = args;
      while (files.length > 0 && files[0] === "-u") files = files.slice(1);
      if (files.length > 0 && files[0].startsWith("-") && files[0] !== "-") {
        io.write(2, ConMaVM.utf8("cat: illegal option -- " + files[0].slice(1, 2) + "\nusage: cat [-u] [file ...]\n"));
        return 1;
      }
      if (files.length === 0) files = ["-"];
      let status = 0;
      for (const name of files) {
        if (name === "-") {
          for (;;) {
            const data = await io.read();
            if (data === null) break;
            if (!io.write(1, data)) return -13;
          }
          continue;
        }
        const data = fs.readFile(pathOf(io.cwd, name));
        if (typeof data === "number") {
          io.write(2, ConMaVM.utf8("cat: " + name + ": " + errorText(data) + "\n"));
          status = 1;
          continue;
        }
        if (!io.write(1, data)) return -13;
      }
      return status;
    },
    // pwd [-L | -P]: the current directory (that of ConSh) to standard output; the virtual file
    // system has no symbolic links, so -L and -P give the same
    pwd: async (io, args) => {
      for (const a of args) {
        if (a !== "-L" && a !== "-P") {
          const msg = a.startsWith("-") ? "pwd: illegal option -- " + a.slice(1, 2) + "\n" : "pwd: too many arguments\n";
          io.write(2, ConMaVM.utf8(msg + "usage: pwd [-L | -P]\n"));
          return 1;
        }
      }
      return io.write(1, ConMaVM.utf8(io.cwd + "\n")) ? 0 : -13;
    },
    // ls [-1AadF] [file ...]: the names of the files, and the entries of the directories (of the current
    // directory with no file), one in a line (as ls does when its output is not a terminal), sorted
    // by name; -a also . and .. and the names beginning with ., -A those names alone, -d the
    // directories themselves, -F a / after directories
    ls: async (io, args) => {
      const opt = { a: false, A: false, d: false, F: false };
      let i = 0;
      for (; i < args.length && args[i].startsWith("-") && args[i] !== "-"; i++) {
        if (args[i] === "--") {
          i++;
          break;
        }
        for (const c of args[i].slice(1)) {
          if (c in opt) opt[c] = true;
          else if (c !== "1") {
            io.write(2, ConMaVM.utf8("ls: invalid option -- " + c + "\nusage: ls [-1AadF] [file ...]\n"));
            return 1;
          }
        }
      }
      const names = args.length > i ? args.slice(i) : ["."];
      const mark = (name, dir) => (opt.F && dir ? name + "/" : name);
      let status = 0;
      const files = [];
      const dirs = [];
      for (const name of names) {
        const path = pathOf(io.cwd, name);
        const k = fs.stat(path);
        const entries = k === 1 && !opt.d ? fs.readdir(path) : null;
        const r = typeof entries === "number" ? entries : k;
        if (r < 0) {
          io.write(2, ConMaVM.utf8("ls: " + name + ": " + errorText(r) + "\n"));
          status = 1;
        } else if (entries) {
          const prefix = path.endsWith("/") ? path : path + "/";
          dirs.push([name, entries.map((e) => ({ name: e, dir: opt.F && fs.stat(prefix + e) === 1 }))]);
        } else files.push(mark(name, k === 1));
      }
      const byName = (x, y) => (x < y ? -1 : x > y ? 1 : 0);
      const lines = files.sort(byName);
      dirs.sort((x, y) => byName(x[0], y[0]));
      for (const [name, entries] of dirs) {
        if (names.length > 1) {
          if (lines.length > 0) lines.push("");
          lines.push(name + ":");
        }
        if (opt.a) lines.push(mark(".", true), mark("..", true));
        for (const e of entries) if (opt.a || opt.A || !e.name.startsWith(".")) lines.push(mark(e.name, e.dir));
      }
      if (lines.length > 0 && !io.write(1, ConMaVM.utf8(lines.join("\n") + "\n"))) return -13;
      return status;
    },
  };

  function errorText(errno) {
    const texts = { "-2": "No such file or directory", "-20": "Not a directory", "-21": "Is a directory" };
    return texts[errno] || "Input/output error";
  }

  const procs = new Map(); // id -> process
  let nextProc = 0;
  let tickWaiters = []; // processes waiting for the VM to run (data in its pipes)
  const termDecoders = { 1: new TextDecoder(), 2: new TextDecoder() };

  // After each slice of the VM: wake the processes that wait for its pipes.
  function procTick() {
    const ws = tickWaiters;
    tickWaiters = [];
    for (const w of ws) w();
  }

  // The running process that reads the terminal (the last started), or null.
  function terminalProcess() {
    let found = null;
    for (const p of procs.values()) if (p.code === null && p.terminal) found = p;
    return found;
  }

  function spawnProcess(argv, env, cwd, stdio) {
    // Only as builtin commands of ConSh (builtin:<name>); there are no external commands
    const command = argv[0].startsWith("builtin:") ? COMMANDS[argv[0].slice(8)] : null;
    if (!command) return -2; // ENOENT: ConSh says "No such file or directory" and sets $? to 127
    const id = ++nextProc;
    const p = {
      code: null,
      terminal: stdio[0].kind === 0 && stdio[0].n === 0,
      termData: [],
      termEof: false,
      termWaiter: null,
      // A line typed, or Ctrl+D (an end of input for this read only, as a terminal gives it)
      fromTerminal(data) {
        if (data === null) this.termEof = true;
        else this.termData.push(data);
        if (this.termWaiter) {
          const w = this.termWaiter;
          this.termWaiter = null;
          w();
        }
      },
    };
    procs.set(id, p);
    const ownFds = stdio.filter((s) => s.kind === 2 || s.kind === 3).map((s) => s.n);
    // A file (kind 1, an fd of serverfs.js) is the process's own until it ends
    stdio = stdio.map((s) => (s.kind === 1 ? { kind: 1, n: fs.dup(s.n) } : s));
    const fileFds = stdio.filter((s) => s.kind === 1 && s.n >= 0).map((s) => s.n);
    const io = {
      cwd,
      env,
      async read() {
        const s = stdio[0];
        for (;;) {
          if (p.code !== null) return null;
          if (s.kind === 0 && s.n === 0) {
            if (p.termData.length > 0) return p.termData.shift();
            if (p.termEof) {
              p.termEof = false;
              return null;
            }
            await new Promise((resolve) => (p.termWaiter = resolve));
            continue;
          }
          if (s.kind === 1) {
            const r = fs.read(s.n, 65536);
            return typeof r === "number" || r.length === 0 ? null : r;
          }
          if (s.kind !== 2) return null;
          const r = vm.read(s.n, 65536);
          if (typeof r === "number") {
            if (r !== -35) return null;
            await new Promise((resolve) => tickWaiters.push(resolve));
            continue;
          }
          return r.length === 0 ? null : r;
        }
      },
      write(i, bytes) {
        if (p.code !== null) return false;
        const s = stdio[i];
        if (s.kind === 0) show(termDecoders[s.n === 2 ? 2 : 1].decode(bytes, { stream: true }), s.n === 2 ? "e" : "");
        else if (s.kind === 1) fs.write(s.n, bytes);
        else if (s.kind === 3) {
          if (vm.write(s.n, bytes) === -32) return false;
          pump();
        }
        return true;
      },
    };
    const end = (code) => {
      if (p.code !== null) return;
      p.code = code;
      for (const fd of ownFds) vm.close(fd);
      for (const fd of fileFds) fs.close(fd);
      vm.childEvent();
      pump();
    };
    p.kill = (sig) => end(-sig);
    command(io, argv.slice(1)).then(end, (e) => {
      io.write(2, ConMaVM.utf8(argv[0] + ": " + e + "\n"));
      end(1);
    });
    return id;
  }

  const hostProc = {
    spawn: (argv, env, cwd, stdio) => spawnProcess(argv, env, cwd, stdio),
    poll: (id) => (procs.has(id) ? procs.get(id).code : -10),
    kill: (id, sig) => {
      const p = procs.get(id);
      if (p && p.code === null) p.kill(sig);
      return 0;
    },
  };

  let vm;
  try {
    const [wasmBytes, moduleBytes] = await Promise.all(
      // Ask the server each time (it answers 304 when nothing changed), so that the page never runs
      // an old module or wasm from the cache after consh/js/build.sh
      ["conma.wasm", "consh.module.se"].map((f) =>
        fetch(f, { cache: "no-cache" }).then((r) => {
          if (!r.ok) throw new Error(f + ": " + r.status + " " + r.statusText);
          return r.arrayBuffer();
        })
      )
    );
    const wasm = await WebAssembly.compile(wasmBytes);
    const decoders = { 1: new TextDecoder(), 2: new TextDecoder() };
    vm = ConMaVM.create(
      wasm,
      (fd, bytes) => show(decoders[fd].decode(bytes, { stream: true }), fd === 2 ? "e" : ""),
      { jsDone: () => pump(), hostProc, hostFs: fs }
    );
    vm.useHostFs();
    vm.setCwd(HOME);
    for (const [k, v] of [["HOME", HOME], ["PWD", HOME], ["USER", "user"], ["PS1", "$ "]]) vm.setenv(k, v);
    // The commands of COMMANDS are builtin commands of ConSh
    vm.setenv("CONSH_HOST_BUILTINS", Object.keys(COMMANDS).join(" "));
    const code = vm.start(new Uint8Array(moduleBytes), ["--main", "consh.module.se", "interactive", "-", "stdin"]);
    if (code !== 0) {
      finish(code);
      return;
    }
  } catch (e) {
    status.textContent = "読み込めませんでした";
    show(String(e) + "\n(consh/js/build.sh を実行し、consh/js/web を HTTP で開いてください)\n", "e");
    return;
  }
  status.textContent = "動作中";
  show(
    "ConSh (ConMa VM on WebAssembly). 外部コマンドはありません。cat、pwd、ls は JavaScript で作った組み込みコマンドです。\n" +
      "ファイルはサーバーのもので (" + HOME + " がこのページのディレクトリ)、書いたファイルはページを閉じるまでメモリに置きます。例:\n" +
      '  function greet (. names) { for n in $names; do echo "hello, $n"; done }\n' +
      "  greet $(cat < names.txt)\n",
    "n"
  );
  line.disabled = false;
  line.focus();

  // Run the VM in slices, so that the page stays responsive.
  let pumping = false;
  let done = false;
  function pump() {
    if (pumping || done) return;
    pumping = true;
    slice();
  }
  function slice() {
    const t0 = performance.now();
    let r;
    do {
      r = vm.run(20000);
    } while (r === ConMaVM.RUNNING && performance.now() - t0 < 25);
    procTick();
    if (r === ConMaVM.RUNNING) {
      setTimeout(slice, 0);
      return;
    }
    pumping = false;
    if (r === ConMaVM.IDLE) {
      // pump runs again when input comes, or when JavaScript gives a result (jsDone)
      const tp = terminalProcess();
      status.textContent = vm.jsPending() > 0 ? "JavaScript 待ち" : tp ? "入力待ち (cat など)" : "入力待ち";
      return;
    }
    finish(r);
  }
  function finish(code) {
    done = true;
    line.disabled = true;
    status.textContent = "終了しました (exit " + code + ")";
    show("\n[exit " + code + "]\n", "n");
  }

  const history = [];
  let hpos = 0;
  function send(text) {
    status.textContent = "動作中";
    vm.input(text);
    pump();
  }
  // Ctrl+C: the process reading the terminal, if any, is stopped (SIGINT, 2), and ConSh gets the interrupt
  function interrupt() {
    if (done) return;
    show("^C\n", "n");
    const tp = terminalProcess();
    if (tp) tp.kill(2);
    vm.interrupt();
    pump();
  }
  line.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter") {
      const text = line.value;
      line.value = "";
      show(text + "\n");
      if (text !== "") history.push(text);
      hpos = history.length;
      const tp = terminalProcess();
      if (tp) tp.fromTerminal(ConMaVM.utf8(text + "\n"));
      else send(text + "\n");
      ev.preventDefault();
    } else if (ev.key === "c" && ev.ctrlKey) {
      line.value = "";
      interrupt();
      ev.preventDefault();
    } else if (ev.key === "d" && ev.ctrlKey && line.value === "") {
      show("^D\n", "n");
      // The end of input of the process that reads the terminal, or else of ConSh (which then ends)
      const tp = terminalProcess();
      if (tp) tp.fromTerminal(null);
      else {
        vm.inputEnd();
        pump();
      }
      ev.preventDefault();
    } else if (ev.key === "ArrowUp" && hpos > 0) {
      line.value = history[--hpos];
      ev.preventDefault();
    } else if (ev.key === "ArrowDown" && hpos < history.length) {
      hpos++;
      line.value = hpos < history.length ? history[hpos] : "";
      ev.preventDefault();
    }
  });
  document.getElementById("intr").addEventListener("click", () => {
    interrupt();
    line.focus();
  });
  term.addEventListener("click", () => {
    if (!getSelection().toString()) line.focus();
  });
  document.getElementById("script").addEventListener("change", async (ev) => {
    const f = ev.target.files[0];
    ev.target.value = "";
    if (!f || done) return;
    let text = await f.text();
    if (text !== "" && !text.endsWith("\n")) text += "\n";
    show("[" + f.name + " を実行]\n" + text, "s");
    send(text);
    line.focus();
  });
  document.getElementById("files").addEventListener("change", async (ev) => {
    for (const f of ev.target.files) {
      const errno = fs.writeFile(pathOf(HOME, f.name), new Uint8Array(await f.arrayBuffer()));
      show(errno === 0 ? "[" + pathOf(HOME, f.name) + " を置きました]\n" : "[" + f.name + ": " + errorText(errno) + "]\n", "n");
    }
    ev.target.value = "";
    line.focus();
  });
  pump();
})();
