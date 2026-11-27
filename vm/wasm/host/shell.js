// The external commands of the command lines of ConJS (stage S1b of docs/dev/CONJS-SHELL-PLAN.md, section 9;
// conjs/include/conjs_shell.se). The host function "conma.sh.run" argv input output, called with host.wait:
// starts argv[0] with the arguments argv[1..] (no shell), writes the elements of input (an array, or a Queue
// of rt.Queue; false for the standard input of the program) to its standard input, a line for each (a string
// as it is, anything else String(x), then "\n"), and pushes each line of its standard output (without "\n"
// or "\r\n"; bytes read as UTF-8) into the Queue output (false for the standard output of the program), which
// it closes at the end. The standard error is that of the program. It resolves to { status, signal } when the
// command has ended and its output has been read.
const childProcess = require("child_process");
const fs = require("fs");
const path = require("path");

// The message of a command that could not start
function startError(cmd, e) {
  if (e.code === "ENOENT") {
    // Node.js runs .exe and .com files without a shell, not .cmd and .bat (D14)
    if (process.platform === "win32" && !/[\\/]/.test(cmd)) {
      for (const dir of (process.env.PATH || "").split(path.delimiter)) {
        for (const ext of [".cmd", ".bat"]) {
          if (dir && fs.existsSync(path.join(dir, cmd + ext)))
            return `${cmd}: ${cmd + ext} is a .cmd or .bat file, which command lines do not run yet (write $ cmd /c ... with care)`;
        }
      }
    }
    return `${cmd}: command not found`;
  }
  if (e.code === "EINVAL" && /\.(cmd|bat)$/i.test(cmd)) return `${cmd}: command lines do not run .cmd and .bat files yet (write $ cmd /c ... with care)`;
  return `${cmd}: ${e.message}`;
}

// Write the elements of input to the stream w, waiting for room in the pipe; end it. When the command
// stops reading (it has ended, or closed its standard input), stop, and take no more elements from a
// Queue: what is left stays for its other readers (M1's review of shima3/conma#96)
async function feed(w, input) {
  let gone = false;
  // one Promise for the end of the pipe, so that waiting for it adds no listener each time
  const closed = new Promise((resolve) => w.once("close", resolve));
  closed.then(() => {
    gone = true;
  });
  // wait for room in the pipe, or for the pipe to close; the listener that did not fire is taken off
  const room = () => new Promise((resolve) => {
    const done = () => {
      w.off("drain", done);
      w.off("close", done);
      resolve();
    };
    w.on("drain", done);
    w.on("close", done);
  });
  const line = (x) => `${typeof x === "string" ? x : String(x)}\n`;
  try {
    if (Array.isArray(input)) {
      for (const x of input) {
        if (gone || w.destroyed) break;
        if (!w.write(line(x))) await room();
      }
    } else {
      for (;;) {
        if (gone || w.destroyed) break;
        // wait for an element or the end without taking it, so that none is taken after the command
        // has stopped reading
        const ready = input.ready();
        if (ready !== null) {
          await Promise.race([ready, closed]);
          continue;
        }
        const r = input.next();
        if (r.done) break;
        if (!w.write(line(r.value))) await room();
      }
    }
  } finally {
    w.end();
  }
}

function run(argv, input, output) {
  return new Promise((resolve, reject) => {
    const close = () => {
      if (output !== false && !output.closed) output.close();
    };
    if (!Array.isArray(argv) || argv.length === 0) {
      close();
      reject(new Error("conma.sh.run: no command"));
      return;
    }
    let p;
    try {
      p = childProcess.spawn(argv[0], argv.slice(1), {
        stdio: [input === false ? "inherit" : "pipe", output === false ? "inherit" : "pipe", "inherit"],
      });
    } catch (e) {
      close();
      reject(new Error(startError(argv[0], e)));
      return;
    }
    let failed = null;
    p.on("error", (e) => {
      failed = e;
    });
    if (input !== false) {
      // a command that ends without reading all of its input: EPIPE, not an error of the line
      p.stdin.on("error", () => {});
      feed(p.stdin, input).catch(() => {});
    }
    if (output !== false) {
      const decoder = new TextDecoder();
      let rest = "";
      const put = (text) => {
        rest += text;
        let at;
        while ((at = rest.indexOf("\n")) >= 0) {
          const s = rest.slice(0, at);
          rest = rest.slice(at + 1);
          if (!output.closed) output.push(s.endsWith("\r") ? s.slice(0, -1) : s);
        }
      };
      p.stdout.on("data", (chunk) => put(decoder.decode(chunk, { stream: true })));
      p.stdout.on("end", () => {
        put(decoder.decode());
        if (rest !== "" && !output.closed) output.push(rest.endsWith("\r") ? rest.slice(0, -1) : rest);
        rest = "";
      });
    }
    p.on("close", (code, signal) => {
      close();
      if (failed !== null) reject(new Error(startError(argv[0], failed)));
      else resolve({ status: code, signal: signal || undefined });
    });
  });
}

function shellHost() {
  return { hostFunctions: { "conma.sh.run": run } };
}

module.exports = { shellHost };
