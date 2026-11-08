// ConSh in a browser: the ConMa VM for wasm32 (conma.wasm, built by consh/js/build.sh) runs the ConMa
// version of ConSh (consh.module.se) in interactive mode, as `consh` with no arguments does. Each line
// typed is given to its standard input; standard output and standard error are shown in the page.
//
// The files of the page are in the virtual file system of the VM, from /home/user (the current
// directory), where the page puts the files of HOME_FILES (from consh/js/web) when it opens;
// "ファイルを置く" puts files there, and "スクリプトを実行" gives the lines of a script to
// standard input, as if they were typed (the ConMa version of ConSh has no source).
//
// The commands of COMMANDS, written in JavaScript (cat), are builtin commands of ConSh: the page
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
  const HOME = "/home/user";
  // Files of consh/js/web that are put in HOME when the page opens
  const HOME_FILES = ["names.txt"];

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
        const data = vm.getFile(name.startsWith("/") ? name : io.cwd + "/" + name);
        if (data === null) {
          io.write(2, ConMaVM.utf8("cat: " + name + ": No such file or directory\n"));
          status = 1;
          continue;
        }
        if (!io.write(1, data)) return -13;
      }
      return status;
    },
  };

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
      { jsDone: () => pump(), hostProc }
    );
    vm.makeDirs(HOME);
    vm.setCwd(HOME);
    for (const [k, v] of [["HOME", HOME], ["PWD", HOME], ["USER", "user"], ["PS1", "$ "]]) vm.setenv(k, v);
    // The commands of COMMANDS are builtin commands of ConSh
    vm.setenv("CONSH_HOST_BUILTINS", Object.keys(COMMANDS).join(" "));
    // The files of the page that start in HOME (fetched from beside the page)
    for (const name of HOME_FILES) {
      const r = await fetch(name, { cache: "no-cache" });
      if (r.ok) vm.putFile(HOME + "/" + name, new Uint8Array(await r.arrayBuffer()));
    }
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
    "ConSh (ConMa VM on WebAssembly). 外部コマンドはありません。cat は JavaScript で作った組み込みコマンドです。例:\n" +
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
      const errno = vm.putFile(HOME + "/" + f.name, new Uint8Array(await f.arrayBuffer()));
      show(errno === 0 ? "[" + HOME + "/" + f.name + " を置きました]\n" : "[" + f.name + ": errno " + errno + "]\n", "n");
    }
    ev.target.value = "";
    line.focus();
  });
  pump();
})();
