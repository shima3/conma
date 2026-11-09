// The Worker of ConSh in a browser: it runs the VM, the file system and the builtin commands
// (consh-core.js), so that the page stays responsive while the VM reads files with synchronous
// requests and runs. The page (consh.js) shows the terminal and evaluates JavaScript, which needs the
// page (document, window): the calls of JavaScript of the VM (__JS_call__) go to the page as messages
// (jsRemote), and their lines come back. __JS_value__ is synchronous: the Worker asks the page and
// waits with Atomics.wait on a SharedArrayBuffer that the page gave it, which needs a page that is
// cross-origin isolated (serve.py sends the headers for it).
//
// Messages from the page: start {home, sab}, typed {text}, ctrlC, ctrlD, send {text},
// file {name, bytes}, jsLine {id, line}, jsEnd {id}. To the page: show {text, cls}, status {text},
// ready {commands}, finish {code}, jsCall {id, code, args}, jsInput {id, data}, jsInputEnd {id},
// jsValue {id, offset}, jsReaderGone {id}.
"use strict";
{
  // Load the scripts again each time, as the page does (the version is in the URL of the Worker)
  const v = new URL(location.href).search;
  importScripts("conma.js" + v, "serverfs.js" + v, "consh-core.js" + v);
}

let core = null;
// The SharedArrayBuffer for __JS_value__: ctl[0] is 1 when the page has answered, ctl[1] the bytes of
// this piece (or minus an errno) and ctl[2] the bytes in all; the bytes are in data
let ctl = null;
let data = null;

const jsRemote = {
  call: (id, code, args) => postMessage({ type: "jsCall", id, code, args }),
  input: (id, text) => postMessage({ type: "jsInput", id, data: text }),
  inputEnd: (id) => postMessage({ type: "jsInputEnd", id }),
  // The text of the value of id, asked of the page piece by piece; or minus an errno
  value: (id) => {
    let bytes = null;
    for (let offset = 0; ; ) {
      Atomics.store(ctl, 0, 0);
      postMessage({ type: "jsValue", id, offset });
      Atomics.wait(ctl, 0, 0);
      const n = Atomics.load(ctl, 1);
      if (n < 0) return n;
      if (bytes === null) bytes = new Uint8Array(Atomics.load(ctl, 2));
      bytes.set(data.subarray(0, n), offset);
      offset += n;
      if (offset >= bytes.length) return ConMaVM.text(bytes);
    }
  },
};

const ui = {
  show: (text, cls) => {
    if (text !== "") postMessage({ type: "show", text, cls });
  },
  status: (text) => postMessage({ type: "status", text }),
  ready: (commands) => postMessage({ type: "ready", commands }),
  finish: (code) => postMessage({ type: "finish", code }),
};

onmessage = async (ev) => {
  const m = ev.data;
  if (m.type === "start") {
    ctl = new Int32Array(m.sab, 0, 4);
    data = new Uint8Array(m.sab, 16);
    core = await ConshCore.start(ui, { home: m.home, jsRemote });
    return;
  }
  if (!core) return;
  if (m.type === "typed") core.typed(m.text);
  else if (m.type === "ctrlC") core.ctrlC();
  else if (m.type === "ctrlD") core.ctrlD();
  else if (m.type === "send") core.send(m.text);
  else if (m.type === "file") ui.show(core.putFile(m.name, m.bytes), "n");
  else if (m.type === "jsLine") {
    if (core.vm.jsLine(m.id, m.line) === 32) postMessage({ type: "jsReaderGone", id: m.id });
  } else if (m.type === "jsEnd") core.vm.jsEnd(m.id);
};
