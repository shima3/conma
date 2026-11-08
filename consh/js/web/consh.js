// ConSh in a browser: the ConMa VM for wasm32 (conma.wasm, built by consh/js/build.sh) runs the ConMa
// version of ConSh (consh.module.se) in interactive mode, as `consh` with no arguments does. Each line
// typed is given to its standard input; standard output and standard error are shown in the page.
//
// The files of the page are in the virtual file system of the VM, from /home/user (the current
// directory); "ファイルを置く" puts files there, and "スクリプトを実行" gives the lines of a script to
// standard input, as if they were typed (the ConMa version of ConSh has no source). There are no
// external commands.
(async function () {
  "use strict";
  const out = document.getElementById("out");
  const term = document.getElementById("term");
  const line = document.getElementById("line");
  const status = document.getElementById("status");
  const HOME = "/home/user";

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

  let vm;
  try {
    const [wasmBytes, moduleBytes] = await Promise.all(
      ["conma.wasm", "consh.module.se"].map((f) =>
        fetch(f).then((r) => {
          if (!r.ok) throw new Error(f + ": " + r.status + " " + r.statusText);
          return r.arrayBuffer();
        })
      )
    );
    const wasm = await WebAssembly.compile(wasmBytes);
    const decoders = { 1: new TextDecoder(), 2: new TextDecoder() };
    vm = ConMaVM.create(wasm, (fd, bytes) => show(decoders[fd].decode(bytes, { stream: true }), fd === 2 ? "e" : ""));
    vm.makeDirs(HOME);
    vm.setCwd(HOME);
    for (const [k, v] of [["HOME", HOME], ["PWD", HOME], ["USER", "user"], ["PS1", "$ "]]) vm.setenv(k, v);
    vm.putFile(HOME + "/names.txt", "ConSh\nConMa\nwasm\n");
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
    "ConSh (ConMa VM on WebAssembly). 外部コマンドはありません。例:\n" +
      '  function greet (. names) { for n in $names; do echo "hello, $n"; done }\n' +
      "  greet $(stdin < names.txt)\n",
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
    if (r === ConMaVM.RUNNING) {
      setTimeout(slice, 0);
      return;
    }
    pumping = false;
    if (r === ConMaVM.IDLE) {
      status.textContent = "入力待ち";
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
  function interrupt() {
    if (done) return;
    show("^C\n", "n");
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
      send(text + "\n");
      ev.preventDefault();
    } else if (ev.key === "c" && ev.ctrlKey) {
      line.value = "";
      interrupt();
      ev.preventDefault();
    } else if (ev.key === "d" && ev.ctrlKey && line.value === "") {
      show("^D\n", "n");
      vm.inputEnd();
      pump();
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
