// The external commands of the command lines of ConJS (stages S1b and S3 of docs/dev/CONJS-SHELL-PLAN.md;
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

// Push the lines of the readable stream r into the Queue q (UTF-8; "\n" and "\r\n" end a line; a last
// line without one too). A queue with a limit makes r pause until it has room; a queue whose reader has
// gone (abandon) makes r stop, so that the command gets SIGPIPE on its next write, as in a shell. done is
// called at the end of r.
function linesInto(r, q, done) {
  const decoder = new TextDecoder();
  let rest = "";
  let stopped = false;
  const push = (s) => {
    if (stopped) return;
    if (q.gone || q.closed) {
      stopped = true;
      r.destroy();
      return;
    }
    const wait = q.push(s.endsWith("\r") ? s.slice(0, -1) : s);
    if (wait !== undefined) {
      r.pause();
      wait.then(() => {
        if (!r.destroyed) r.resume();
      });
    }
  };
  const put = (text) => {
    rest += text;
    let at;
    while ((at = rest.indexOf("\n")) >= 0) {
      const s = rest.slice(0, at);
      rest = rest.slice(at + 1);
      push(s);
    }
  };
  r.on("data", (chunk) => put(decoder.decode(chunk, { stream: true })));
  r.on("error", () => {});
  const end = () => {
    put(decoder.decode());
    if (rest !== "") push(rest);
    rest = "";
    done();
  };
  r.on("end", end);
  r.on("close", () => {
    if (!r.readableEnded) done();
  });
}

// A file of < > >> 2> 2>> (an object that the command line makes: { conjs_sh_file: path, append })
function isFile(x) {
  return x !== null && typeof x === "object" && !Array.isArray(x) && typeof x.conjs_sh_file === "string";
}

// The message of a file that cannot be opened, as Bash writes it
function fileError(file, e) {
  const why = { ENOENT: "No such file or directory", EACCES: "Permission denied", EISDIR: "Is a directory", ENOTDIR: "Not a directory" }[e.code];
  return `${file}: ${why || e.message}`;
}

// Write the readable stream r to the file descriptor fd, which it then closes; done is called at the end
function bytesInto(r, fd, done) {
  const w = fs.createWriteStream("", { fd, autoClose: true });
  r.on("error", () => {});
  w.on("error", () => {});
  w.on("close", done);
  r.pipe(w);
}

// argv is the words of one command, or an array of them for a pipeline of commands joined by pipes of
// the OS (stage S3; section 11 of docs/dev/CONJS-SHELL-PLAN.md). errs, if given, is for each command a
// Queue for the lines of its standard error (2>$), or false for that of the program. input, output and each
// of errs may also be a file (< > >> 2> 2>>), which this process opens and gives to the command as it is; a
// file that cannot be opened makes the line fail before any command starts. It resolves to the status and
// the signal of the last command.
function run(argv, input, output, errs) {
  return new Promise((resolve, reject) => {
    const cmds = Array.isArray(argv) && argv.length > 0 && Array.isArray(argv[0]) ? argv : [argv];
    const errOf = (i) => (Array.isArray(errs) && errs[i] ? errs[i] : false);
    const isQueue = (q) => q !== false && !isFile(q);
    const closeAll = () => {
      if (isQueue(output) && !output.closed) output.close();
      cmds.forEach((c, i) => {
        const e = errOf(i);
        if (isQueue(e) && !e.closed) e.close();
      });
    };
    if (cmds.some((c) => !Array.isArray(c) || c.length === 0)) {
      closeAll();
      reject(new Error("conma.sh.run: no command"));
      return;
    }
    // the files, opened in the order of the line before any command starts (the commands get copies)
    const win = process.platform === "win32";
    const fds = [];
    const open = (f, flags) => {
      try {
        // /dev/null of Git Bash is NUL on Windows, as hostPath of vm-node.js makes it for fs.open
        const name = win && f.conjs_sh_file === "/dev/null" ? "\\\\.\\NUL" : f.conjs_sh_file;
        const fd = fs.openSync(name, flags);
        fds.push(fd);
        return fd;
      } catch (e) {
        throw new Error(fileError(f.conjs_sh_file, e));
      }
    };
    // On Windows a file opened to add to (O_APPEND) has no right to write but to append, and a program of
    // MSYS (Git Bash) given it fails ("Bad file descriptor"; I7's review of shima3/conma#102): the command
    // writes to a pipe, and this process adds the bytes to the file (kept: these are not closed at the start)
    const kept = new Set();
    const closeFds = (all) => {
      for (const fd of fds) if (all || !kept.has(fd)) fs.closeSync(fd);
      fds.length = 0;
    };
    let inFd, outFd, errFds;
    try {
      inFd = isFile(input) ? open(input, "r") : null;
      errFds = cmds.map((c, i) => (isFile(errOf(i)) ? open(errOf(i), errOf(i).append ? "a" : "w") : null));
      outFd = isFile(output) ? open(output, output.append ? "a" : "w") : null;
    } catch (e) {
      closeFds(true);
      closeAll();
      reject(e);
      return;
    }
    const relayed = (f, fd) => {
      if (fd === null || !win || !f.append) return null;
      kept.add(fd);
      return fd;
    };
    const outSink = relayed(output, outFd);
    if (outSink !== null) outFd = null;
    const errSinks = errFds.map((fd, i) => relayed(errOf(i), fd));
    errSinks.forEach((fd, i) => {
      if (fd !== null) errFds[i] = null;
    });
    const procs = [];
    let failed = null;
    // On Windows, external commands are not joined by a pipe of the OS: the end of a pipe that this process
    // holds is a named pipe opened for overlapped I/O, and a program of MSYS (Git Bash) given it as its
    // standard input gets NUL bytes when it reads it while it is empty ($ sh -c "sleep 0.5; printf out" |
    // cat gave about 2,000,000 of them; step25 case14). This process reads the output of each command and
    // writes it to the next one, through a pipe made for that command, as for the input of <$.
    const relay = process.platform === "win32";
    for (let i = 0; i < cmds.length; i++) {
      const last = i === cmds.length - 1;
      const stdin = i > 0 ? (relay ? "pipe" : procs[i - 1].stdout) : inFd !== null ? inFd : input === false ? "inherit" : "pipe";
      const stdout = last && outFd !== null ? outFd : !last || output !== false ? "pipe" : "inherit";
      const stderr = errFds[i] !== null ? errFds[i] : errOf(i) !== false ? "pipe" : "inherit";
      let p;
      try {
        p = childProcess.spawn(cmds[i][0], cmds[i].slice(1), { stdio: [stdin, stdout, stderr] });
      } catch (e) {
        for (const q of procs) q.kill();
        closeFds(true);
        closeAll();
        reject(new Error(startError(cmds[i][0], e)));
        return;
      }
      if (i > 0 && relay) {
        // this process carries the bytes; pipe() waits for room in the next command's pipe. When the next
        // command stops reading (EPIPE, or its pipe closes), let go of the output of the one before it, so
        // that it fails on its next write, as with SIGPIPE
        const from = procs[i - 1].stdout;
        const stop = () => {
          from.unpipe(p.stdin);
          from.destroy();
        };
        p.stdin.on("error", stop);
        p.stdin.on("close", stop);
        from.on("error", () => p.stdin.end());
        from.pipe(p.stdin);
      } else if (i > 0) {
        // the next command has its own copy of the pipe: this process lets go of the end it does not read,
        // so that a command that stops reading makes the one before it get SIGPIPE
        procs[i - 1].stdout.destroy();
      }
      // a command that cannot start: the last one makes the line fail; one before it is reported on the
      // standard error, and the pipeline goes on, as in Bash (its output is empty; section 18)
      p.on("error", (e) => {
        if (i === cmds.length - 1) {
          if (failed === null) failed = { cmd: cmds[i][0], e };
        } else {
          fs.writeSync(2, `conjs: ${startError(cmds[i][0], e)}\n`);
        }
      });
      procs.push(p);
    }
    closeFds(false);
    const first = procs[0];
    if (isQueue(input)) {
      // a command that ends without reading all of its input: EPIPE, not an error of the line
      first.stdin.on("error", () => {});
      feed(first.stdin, input).catch(() => {});
    }
    // the end comes when every process has closed and every stream given to a Queue has ended
    let waiting = procs.length;
    const lastProc = procs[procs.length - 1];
    let result = null;
    const finish = () => {
      if (--waiting > 0) return;
      closeAll();
      if (failed !== null) reject(new Error(startError(failed.cmd, failed.e)));
      else resolve(result);
    };
    if (isQueue(output)) {
      waiting++;
      linesInto(lastProc.stdout, output, finish);
    }
    if (outSink !== null) {
      waiting++;
      bytesInto(lastProc.stdout, outSink, finish);
    }
    procs.forEach((p, i) => {
      const e = errOf(i);
      if (isQueue(e)) {
        waiting++;
        linesInto(p.stderr, e, finish);
      }
      if (errSinks[i] !== null) {
        waiting++;
        bytesInto(p.stderr, errSinks[i], finish);
      }
      p.on("close", (code, signal) => {
        if (p === lastProc) result = { status: code, signal: signal || undefined };
        finish();
      });
    });
  });
}

function shellHost() {
  return { hostFunctions: { "conma.sh.run": run } };
}

module.exports = { shellHost };
