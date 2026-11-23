**English** | [日本語](QUICKSTART-ja.md)

# ConJS Quick Start

The steps to run ConJS on your own computer: install the tools you need, get the source from GitHub and build it, run a small web server written in ConJS on Node.js, and read its code. It takes 30 minutes to an hour, with the downloads of the tools.

ConJS is a subset of JavaScript. What JavaScript writes with `await`, a call that waits, is written in ConJS as an ordinary call. The [tutorial](TUTORIAL.md) explains the language.

## 0. What you need

* A computer with Windows 10 or 11, or macOS
* A connection to the Internet (to download the tools and the source)

This document uses a terminal, the window where you type commands.

* **macOS**: the Terminal (Applications → Utilities).
* **Windows**: **Git Bash**, which comes with Git for Windows (step 1). Type the commands in Git Bash, not in PowerShell or the Command Prompt: they are for bash.

Type what follows `$` and press Enter (do not type the `$`).

## 1. Install the tools

| Tool | What for |
|---|---|
| Git | Get the source from GitHub |
| Node.js (22 or later) | Run ConJS programs (on the wasm VM) and the web server |
| Rust | Build what runs ConJS (the VMs of ConMa) |

### macOS

1. **Git**: type the following in the Terminal. If you are asked whether to install the developer tools, choose Install. If they are there already, nothing happens.
   ```
   $ xcode-select --install
   ```
2. **Node.js**: download the installer of the LTS version (22 or later) from https://nodejs.org/ and install it.
3. **Rust**: type the following, and press Enter (the default) when asked.
   ```
   $ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
   Then close the Terminal and open it again, so that the command `cargo` can be found.

### Windows

1. **Git**: download Git for Windows from https://git-scm.com/ and install it. The default settings are fine. This installs Git Bash too.
2. **Node.js**: download the installer of the LTS version (22 or later) from https://nodejs.org/ and install it.
3. **Rust**: download `rustup-init.exe` from https://rustup.rs/ and run it. If it asks whether to install the C++ build tools of Visual Studio, install them (Rust uses them to make programs on Windows). Leave the rest as it is.

Then open Git Bash again.

### Check them (both)

Type these three in the terminal (Git Bash on Windows). Each should print a version number.

```
$ git --version
$ node --version
$ cargo --version
```

`node --version` must be `v22` or later. If you see `command not found`, open the terminal again and retry.

Last, add the part of Rust for WebAssembly (used to run ConJS on Node.js):

```
$ rustup target add wasm32-unknown-unknown
```

## 2. Get the source from GitHub (clone)

Go to the folder you work in and get the source. Here it goes in your home folder.

```
$ cd ~
$ git clone -c core.autocrlf=false https://github.com/shima3/conma.git
$ cd conma
```

`-c core.autocrlf=false` keeps the line ends as they are on Windows (if they were changed, the scripts would not run). On macOS it makes no difference.

You now have a folder `conma` and are in it. Type every command below in this folder. ConJS is in its folder `conjs`.

## 3. Build

A ConJS program is translated into the language ConMa and run on a VM (virtual machine) of ConMa. Build two VMs:

```
$ (cd vm && cargo build --release)
$ vm/wasm/build.sh
$ conjs/conjs --prepare
```

* The first is the VM that runs directly on your computer (the Rust VM). It also translates ConJS.
* The second is the VM in WebAssembly (the wasm VM). It runs on Node.js, and the web server below runs on it.
* The third makes the translator of ConJS and its runtime (the parts that ConJS programs use) in advance.

The first build takes from tens of seconds to a few minutes, by the computer. `Finished` (the first two) and `conjs: ready` (the third) mean it worked. The third may print nothing for about 30 seconds (longer on some computers). Wait, and do not press Ctrl-C, even if it seems to have stopped.

## 4. Run your first program

Make a file `hello.con.js`, in an editor (such as VS Code) or with this command:

```
$ echo 'console.log("Hello, ConJS");' > hello.con.js
```

The name of a ConJS file ends with `.con.js`. (The files you make in this document and in the tutorial show as "Untracked" in `git status`; that does no harm, and you may delete them when you no longer need them.) Run it:

```
$ conjs/conjs hello.con.js
Hello, ConJS
```

It starts at once, as `conjs/conjs --prepare` in step 3 has made the translator and the runtime. (Without it, the first run alone takes about 30 seconds here.)

## 5. Run the web server

Run the small web server written in ConJS (`conjs/examples/counter_server.con.js`) on Node.js. A web server runs only on Node.js, so add `CONMA_VM=wasm` to use the wasm VM.

```
$ CONMA_VM=wasm conjs/conjs conjs/examples/counter_server.con.js
listening on http://127.0.0.1:8000/
```

When it says `listening on ...`, the server is running. Open these addresses in your browser:

| Address | Answer |
|---|---|
| http://127.0.0.1:8000/count | 1, 2, 3, … one more each time you open it |
| http://127.0.0.1:8000/slow | `slow`, after 0.2 seconds (it answers the other requests meanwhile) |
| http://127.0.0.1:8000/file?name=README.md | the number of lines of README.md in the folder `conma` |
| http://127.0.0.1:8000/stop | `bye`, and the server stops |

To stop the server, open `/stop` or press Ctrl-C in the terminal.

If port 8000 is taken and you see `EADDRINUSE`, add another number at the end (`... counter_server.con.js 8001`, and use `8001` in the addresses).

## 6. Read the sample code

Open the code of the server in your editor (`conjs/examples/counter_server.con.js`), or in the terminal:

```
$ less conjs/examples/counter_server.con.js
```

(`less` ends with q.)

The most important part is the loop that takes the requests:

```js
function serve(server, dir) {
  for (;;) {
    const req = server.accept();          // wait for the next request
    if (req === undefined) return;        // the server was closed
    spawn(() => handle(server, dir, req)); // answer each request in a task of its own
  }
}
```

`server.accept()` is a call that waits until the next request comes. JavaScript would need `await` there; ConJS writes it as an ordinary call. While it waits, the other tasks that `spawn` started (the answers to the other requests) keep running. The `sleep(200)` in the answer to `/slow` is the same: only that task stops for 0.2 seconds.

The same server written in plain JavaScript for Node.js is `conjs/examples/counter_server.node.js`. It runs on Node.js as it is:

```
$ node conjs/examples/counter_server.node.js
```

Read the two side by side: ConJS needs no `async` and no `await`.

## When something goes wrong

| Message | What to do |
|---|---|
| `cargo: command not found` | Open the terminal again. If it still says so, install Rust again (step 1) |
| `vm-wasm.sh: ... is not built` | Run `vm/wasm/build.sh` (step 3) |
| `vm/wasm/build.sh` stops with an error that `wasm32-unknown-unknown` is not installed | Type `rustup target add wasm32-unknown-unknown` (the end of step 1) |
| Nothing is printed, or you are asked which app should open the file (PowerShell on Windows) | You typed it in PowerShell. Open Git Bash and type it there (step 0) |
| `'conjs' is not recognized as an internal or external command, operable program or batch file.` (the Command Prompt on Windows) | You typed it in the Command Prompt. Open Git Bash and type it there (step 0) |
| `$'\r': command not found` (Windows) | The line ends were changed in the clone. Delete the folder `conma` and clone again with `-c core.autocrlf=false` (step 2) |
| `EADDRINUSE` | Port 8000 is taken. Add another number, as in step 5 |
| `http.listen: this platform has no server` | `CONMA_VM=wasm` is missing |

## What to read next

* [The tutorial](TUTORIAL.md): the JavaScript that ConJS takes, and where it differs from JavaScript
* [The README of ConJS](README.md): all of the language and its library
* Samples of ConJS in the browser (the game Stars and others): https://shima3.github.io/conma/conjs/
