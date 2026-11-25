// bin/lang.cjs: what bin/lang does, in Node.js, when the program runs on the wasm VM (an experiment of I7, 2026-11-25).
//   node bin/lang.cjs root language-directory name translator include file [argument ...]
// bin/lang reads lang.conf and execs this. The program runs in this same process (vm/wasm/vm-node.js), so that Node.js
// starts once, and the checks of the caches, the split of the include lines and the hash need no other process (each
// costs 50 to 170 ms in Git Bash on Windows). The translation, the build and the link are done as bin/lang does them: by
// the Rust VM (vm/target/release/conma) if it is there and CONMA_TRANSLATE_VM is not wasm, else by the wasm VM (vm-node.js
// build and link, in other processes). The modules in the cache are the same files as bin/lang makes and uses.
// bin/lang and bin/cache-dir do the same work in bash (for the other VMs, and with CONMA_LANG_SH=1): when you change the
// rules of the cache (the names of the modules, when they are made again, the hash), change both.
"use strict";
const fs = require("fs");
const path = require("path");
const os = require("os");
const { spawnSync } = require("child_process");

const [root, dir, name, translator, include, file, ...args] = process.argv.slice(2);
const exe = process.platform === "win32" ? ".exe" : "";
const conma = path.join(root, "vm/target/release/conma");
const wasmDir = path.join(root, "vm/wasm");
const wasm = path.join(wasmDir, "target/wasm32-unknown-unknown/release/conma_wasm.wasm");
const vmNode = path.join(wasmDir, "vm-node.js");

function isExecutable(f) {
  try {
    fs.accessSync(f + exe, fs.constants.X_OK);
    return true;
  } catch (e) {
    return false;
  }
}

// The VM that translates and builds (tr), and the files that, when newer, make its modules old (stamps).
let tr, stamps;
if (isExecutable(conma) && process.env.CONMA_TRANSLATE_VM !== "wasm") {
  tr = [conma + exe];
  stamps = [conma + exe];
} else {
  tr = [process.execPath, `--stack-size=${process.execArgv.join(" ").match(/--stack-size=(\d+)/)?.[1] || 984}`, vmNode];
  stamps = [wasm, path.join(root, "vm/frontend.module.se")];
}

const mtime = (f) => {
  try {
    return fs.statSync(f).mtimeMs;
  } catch (e) {
    return -1;
  }
};
const stale = (f) => stamps.some((s) => mtime(s) > mtime(f));
// As find dirs \( -name '*.se' -o -type d \) -newer f: a file *.se or a directory, under dirs, newer than t.
function newerUnder(dirs, t) {
  const walk = (p) => {
    let st;
    try {
      st = fs.statSync(p);
    } catch (e) {
      return false;
    }
    if (st.isDirectory()) {
      if (st.mtimeMs > t) return true;
      for (const e of fs.readdirSync(p)) if (walk(path.join(p, e))) return true;
      return false;
    }
    return p.endsWith(".se") && st.mtimeMs > t;
  };
  return dirs.some(walk);
}

// The CRC of cksum (POSIX), so that the names in the cache are those of bin/lang and bin/cache-dir.
const crcTable = new Uint32Array(256);
for (let i = 0; i < 256; i++) {
  let c = i << 24;
  for (let k = 0; k < 8; k++) c = c & 0x80000000 ? (c << 1) ^ 0x04c11db7 : c << 1;
  crcTable[i] = c >>> 0;
}
function cksum(buf) {
  let c = 0;
  const add = (b) => {
    c = ((c << 8) ^ crcTable[((c >>> 24) ^ b) & 0xff]) >>> 0;
  };
  for (const b of buf) add(b);
  for (let n = buf.length; n > 0; n = Math.floor(n / 256)) add(n & 0xff);
  return (~c >>> 0).toString();
}

// As bin/cache-dir, which hashes the path of the repository as `pwd -P` writes it. On Windows that is Git Bash, which
// writes C:\Users\a as /c/Users/a: the same text is hashed here, so that both use one directory.
function shellPath(p) {
  if (process.platform !== "win32") return p;
  const m = /^([A-Za-z]):[\\/]?(.*)$/.exec(p);
  return m ? `/${m[1].toLowerCase()}/${m[2].replace(/\\/g, "/")}`.replace(/\/$/, "") || "/" : p.replace(/\\/g, "/");
}
function cacheDir() {
  let d = process.env.CONMA_CACHE_DIR;
  if (!d) {
    const home = process.env.HOME || os.homedir();
    const base = path.join(process.env.XDG_CACHE_HOME || path.join(home, ".cache"), "conma");
    d = path.join(base, cksum(Buffer.from(shellPath(fs.realpathSync(root)))));
  }
  fs.mkdirSync(d, { recursive: true, mode: 0o700 });
  return d;
}

// The temporary directory of the modules of the program, removed when this process ends, also by a signal (as the trap of
// bin/lang). Until the program runs, a signal is caught here (caught) and handled before vm-node.js starts; a listener must
// not stay, since vm-node.js traps SIGINT only when there is none (trapInterrupt).
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "lang."));
const removeTmp = () => fs.rmSync(tmp, { recursive: true, force: true });
process.on("exit", removeTmp);
const signals = process.platform === "win32" ? ["SIGINT"] : ["SIGINT", "SIGTERM", "SIGHUP"];
let caught = null;
const catcher = (s) => {
  caught = caught || s;
};
for (const s of signals) process.on(s, catcher);
// Ends this process by the signal s, as bash ends when a command it waits for ends by a signal it does not trap.
function dieBy(s) {
  removeTmp();
  for (const t of signals) process.removeListener(t, catcher);
  process.kill(process.pid, s);
  process.exit(128 + (os.constants.signals[s] || 0)); // if the signal did not end it
}

// Runs tr with arguments; its standard error goes to ours. Returns its standard output, or exits as it did.
function runTr(trArgs) {
  const r = spawnSync(tr[0], tr.slice(1).concat(trArgs), { stdio: ["ignore", "pipe", "inherit"], maxBuffer: 1 << 30 });
  if (r.error) throw r.error;
  if (r.signal) dieBy(r.signal);
  if (r.status !== 0) process.exit(r.status);
  return r.stdout;
}
// Writes the output of tr to a temporary file beside f, then moves it to f (no bin/lang reads a module half made).
function makeInto(f, trArgs) {
  const out = runTr(trArgs);
  const t = `${f}.${process.pid}.tmp`;
  fs.writeFileSync(t, out);
  fs.renameSync(t, f);
}

const incDirs = include.split(/\s+/).filter((d) => d !== "").map((d) => path.join(root, d));
const inc = incDirs.flatMap((d) => ["--include", d]);
const libDirs = incDirs.concat([path.join(root, "include/ext"), path.join(root, "include")]);

const cache = cacheDir();
const transMod = path.join(cache, `${name}c.module.se`);
const translatorFile = path.join(dir, translator);
if (mtime(transMod) <= 0 || fs.statSync(transMod).size === 0 || newerUnder(libDirs.concat([translatorFile]), mtime(transMod)) || stale(transMod))
  makeInto(transMod, ["build", ...inc, translatorFile]);
const prog = runTr(["--main", transMod, file]).toString("latin1");

// The include lines (the runtime) and the rest, as the awk of bin/lang splits them.
const lines = prog.split("\n");
if (lines[lines.length - 1] === "") lines.pop();
const rt = lines.filter((l) => l.startsWith("(include ")).map((l) => `${l}\n`).join("");
const alone = lines.filter((l) => !l.startsWith("(include ")).map((l) => `${l}\n`).join("");
const rthash = cksum(Buffer.from(`${include}\n${rt}`, "latin1"));
const rtsrc = path.join(cache, `${name}-rt-${rthash}.se`);
const rtmod = path.join(cache, `${name}-rt-${rthash}.module.se`);
if (mtime(rtmod) <= 0 || fs.statSync(rtmod).size === 0 || newerUnder(libDirs, mtime(rtmod)) || stale(rtmod)) {
  let same = false;
  try {
    same = fs.readFileSync(rtsrc, "latin1") === rt;
  } catch (e) {}
  if (!same) {
    const t = `${rtsrc}.${process.pid}.tmp`;
    fs.writeFileSync(t, rt, "latin1");
    fs.renameSync(t, rtsrc);
  }
  makeInto(rtmod, ["build", ...inc, rtsrc]);
}
const aloneSe = path.join(tmp, "alone.se");
fs.writeFileSync(aloneSe, alone, "latin1");
const aloneMod = path.join(tmp, "alone.module.se");
fs.writeFileSync(aloneMod, runTr(["build", ...inc, aloneSe]));
const progMod = path.join(tmp, "prog.module.se");
fs.writeFileSync(progMod, runTr(["link", rtmod, aloneMod]));

// Run it here, as vm-wasm.sh runs node vm-node.js --main prog.module.se arguments. Everything above is synchronous: let a
// signal that came meanwhile reach catcher first. vm-node.js reads the module before it first waits, that is, before
// require returns, so the temporary directory goes then.
setImmediate(() => {
  if (caught) dieBy(caught);
  for (const s of signals) process.removeListener(s, catcher);
  process.argv = [process.argv[0], vmNode, "--main", progMod, ...args];
  require(vmNode);
  removeTmp();
});
