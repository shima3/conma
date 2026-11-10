// ConSh in a browser: the page shows the terminal, and runs the ConMa version of ConSh (consh-core.js)
// in a Worker (consh-worker.js), so that the page stays responsive while the VM reads files with
// synchronous requests and runs. JavaScript (js and the SJS of (( ))) is evaluated here in the page, where
// it can use document and window: the Worker gives the calls of JavaScript to the JsEngine of conma.js
// here, and answers __JS_value__ synchronously through a SharedArrayBuffer (Atomics). A
// SharedArrayBuffer needs a page that is cross-origin isolated (serve.py sends the headers
// Cross-Origin-Opener-Policy and Cross-Origin-Embedder-Policy); with another server, the page runs
// ConSh in itself, as before the Worker (and then may stop while a file is read).
//
// Each line typed is given to ConSh (or to a builtin command that reads the terminal); standard
// output and standard error are shown in the page. The files are those of the server (serverfs.js):
// /home/user, the current directory, is the directory of the page (consh/js/web), whose files are read
// from the server each time, and the files written (and those that "ファイルを置く" puts) are kept in
// memory over them until the page is closed. "スクリプトを実行" gives the lines of a script to standard
// input, as if they were typed (the ConMa version of ConSh has no source).
(async function () {
  "use strict";
  const out = document.getElementById("out");
  const term = document.getElementById("term");
  const line = document.getElementById("line");
  const statusText = document.getElementById("status");
  // The directory of the page is HOME, the home and current directory of ConSh: /home/user, or the
  // absolute path given as ?home= (?home=/ makes it the root)
  const HOME = ServerFS.normalize(new URLSearchParams(location.search).get("home") || "/home/user") || "/home/user";
  const useWorker = typeof Worker !== "undefined" && typeof SharedArrayBuffer !== "undefined" && self.crossOriginIsolated === true;

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

  let done = false;
  const ui = {
    show,
    status: (text) => (statusText.textContent = text),
    ready: (commands) => {
      show(
        "ConSh (ConMa VM on WebAssembly" + (useWorker ? ", Worker" : "") + "). 外部コマンドはありません。" +
          commands.join("、") + " は JavaScript で作った組み込みコマンドです。\n" +
          "ファイルはサーバーのもので (" + HOME + " がこのページのディレクトリ)、書いたファイルはページを閉じるまでメモリに置きます。例:\n" +
          '  function greet (. names) { for n in $names; do echo "hello, $n"; done }\n' +
          "  greet $(cat < names.txt)\n" +
          (useWorker ? "" : "(このサーバーはページを cross-origin isolated にしないので、ConSh をページの中で動かします。consh/js/serve.py を使うと Worker で動かします)\n"),
        "n"
      );
      line.disabled = false;
      line.focus();
    },
    finish: (code) => {
      done = true;
      line.disabled = true;
      statusText.textContent = "終了しました (exit " + code + ")";
      show("\n[exit " + code + "]\n", "n");
    },
  };

  // The controller of ConSh: in the Worker (messages), or in the page (consh-core.js)
  let core;
  if (useWorker) {
    const worker = new Worker("consh-worker.js?" + Date.now());
    const post = (m) => worker.postMessage(m);
    const decoders = { 1: new TextDecoder(), 2: new TextDecoder() };
    // The JavaScript of the VM in the Worker: its lines go back as messages
    const engine = ConMaVM.JsEngine(
      {
        line: (id, l) => {
          post({ type: "jsLine", id, line: l });
          return 0; // a reader that has gone comes back as jsReaderGone
        },
        end: (id) => post({ type: "jsEnd", id }),
      },
      (fd, bytes) => show(decoders[fd].decode(bytes, { stream: true }), fd === 2 ? "e" : "")
    );
    // __JS_value__: the text of a value, given to the Worker piece by piece through the SharedArrayBuffer
    const sab = new SharedArrayBuffer(16 + (1 << 20));
    const ctl = new Int32Array(sab, 0, 4);
    const data = new Uint8Array(sab, 16);
    let lastValue = null; // {id, bytes} of the value being given
    function giveValue(id, offset) {
      if (offset === 0 || !lastValue || lastValue.id !== id) {
        try {
          lastValue = { id, bytes: ConMaVM.utf8(engine.value(id)) };
        } catch (e) {
          lastValue = null;
          Atomics.store(ctl, 1, -22); // EINVAL: no such value
          Atomics.store(ctl, 0, 1);
          Atomics.notify(ctl, 0);
          return;
        }
      }
      const piece = lastValue.bytes.subarray(offset, offset + data.length);
      data.set(piece);
      Atomics.store(ctl, 1, piece.length);
      Atomics.store(ctl, 2, lastValue.bytes.length);
      Atomics.store(ctl, 0, 1);
      Atomics.notify(ctl, 0);
    }
    worker.onmessage = (ev) => {
      const m = ev.data;
      if (m.type === "show") show(m.text, m.cls);
      else if (m.type === "status") ui.status(m.text);
      else if (m.type === "ready") ui.ready(m.commands);
      else if (m.type === "finish") ui.finish(m.code);
      else if (m.type === "jsCall") engine.call(m.id, m.code, m.args);
      else if (m.type === "jsInput") engine.input(m.id, m.data);
      else if (m.type === "jsInputEnd") engine.inputEnd(m.id);
      else if (m.type === "jsReaderGone") engine.readerGone(m.id);
      else if (m.type === "jsValue") giveValue(m.id, m.offset);
    };
    worker.onerror = (e) => {
      ui.status("読み込めませんでした");
      show(String(e.message || e) + "\n(consh/js/build.sh を実行し、consh/js/web を HTTP で開いてください)\n", "e");
    };
    post({ type: "start", home: HOME, sab });
    core = {
      typed: (text) => post({ type: "typed", text }),
      ctrlC: () => post({ type: "ctrlC" }),
      ctrlD: () => post({ type: "ctrlD" }),
      send: (text) => post({ type: "send", text }),
      putFile: (name, bytes) => post({ type: "file", name, bytes }),
    };
  } else {
    core = await ConshCore.start(ui, { home: HOME });
    if (!core) return;
    const putFile = core.putFile;
    core.putFile = (name, bytes) => show(putFile(name, bytes), "n");
  }

  const history = [];
  let hpos = 0;
  function interrupt() {
    if (done) return;
    show("^C\n", "n");
    core.ctrlC();
  }
  line.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter") {
      const text = line.value;
      line.value = "";
      show(text + "\n");
      if (text !== "") history.push(text);
      hpos = history.length;
      ui.status("動作中");
      core.typed(text + "\n");
      ev.preventDefault();
    } else if (ev.key === "c" && ev.ctrlKey) {
      line.value = "";
      interrupt();
      ev.preventDefault();
    } else if (ev.key === "d" && ev.ctrlKey && line.value === "") {
      show("^D\n", "n");
      core.ctrlD();
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
    core.send(text);
    line.focus();
  });
  document.getElementById("files").addEventListener("change", async (ev) => {
    for (const f of ev.target.files) core.putFile(f.name, new Uint8Array(await f.arrayBuffer()));
    ev.target.value = "";
    line.focus();
  });
})();
