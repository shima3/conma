# ConMa
ConMa is an intermediate language that provides higher-order functions and continuations for functional programming.

## What ConMa offers

- **Continuations everywhere.** Every function is called in continuation-passing style, and the rest of the computation (the CChain) is an ordinary value. `return`, `break`, exceptions, generators and delimited continuations can be written as library code instead of being built into the language. ConSh implements `return`, `break`, `continue`, and `reset`/`shift` this way.
- **Currying by default.** Calling a function with fewer arguments gives a partial application, and extra arguments are applied to the result.
- **Lightweight processes.** VProcs run concurrently under a scheduler that counts steps, so a run can be traced step by step and repeated.
- **A small core.** A program is a few kinds of nodes plus a set of primitives ([PRIMITIVES](docs/PRIMITIVES.md)), which keeps new implementations small. There are four: the Python VM (`vm/python/vproc_step.py`, the reference) and the Rust VM (`vm/`), which give the same traces step by step; the transpiled VM, the Rust VM with Rust code made from ConMa modules ahead of time (`vm/conma2rust`); and the Rust VM built for WebAssembly, which runs ConSh in a browser and in Node.js (`consh/js`).
- **A self-hosted front end.** The lexer, parser, includer and resolver are also written in ConMa ([SELFHOST](docs/dev/SELFHOST.md)). The Rust VM embeds it, so `conma build` and `conma run` turn source files into modules and run them without Python.

ConMa is an experimental language. Its interpreting VMs (Python and Rust) are much slower than CPython; the transpiled VM, which runs Rust code made from the program ahead of time, is about as fast as CPython 3.9 on the benchmarks of [bench/](bench/README.md), which also says what those results do not show.

## Try it

Build the Rust VM and run ConSh, the shell written in ConMa, on it (interactive mode; `exit` or Ctrl-D ends it):

```
cd vm && cargo build --release && cd ..
CONSH_IMPL=conma consh/bin/consh
```

The Rust VM interprets ConMa, and a simple statement takes thousands of ConMa steps, so loops of statements in this ConSh are slower than in Bash: 3 to 10 times on most benchmarks of [consh/bench](consh/bench/README-ja.md), and over 40 times on one (it is faster than Bash where Bash forks for `$( )`; external commands connected by pipes run at full speed). The transpiled VM runs it about twice as fast (`bench/build.sh`, which takes about 20 minutes to build; [docs/consh/RUNNING-ja.md](docs/consh/RUNNING-ja.md)). To use ConSh as a shell rather than to try ConMa, build its Rust version (Rust 1.87 or later), which runs about as fast as Bash, and run `consh/bin/consh` without `CONSH_IMPL`:

```
cd consh/rust && cargo build --release && cd ../..
consh/bin/consh
```

Run the tests with the Python VM (the default) or another VM (`bin/vm`); they pass when nothing but the `== name` line of each test is printed:

```
test/run_all.sh -j 8
CONMA_VM=rust test/run_all.sh -j 8
```

## For developers

Those who change ConMa (its VMs, its front end and its standard library) or ConSh read [docs/dev/CONTRIBUTING.md](docs/dev/CONTRIBUTING.md): the branches of the work, `alpha` and `main`, the Git settings of each clone, how a change goes through the tester into `main`, how the tests are run and recorded, how commit messages are written, how English and Japanese documents are named and kept in step, and how a text refers to a file that has moved. Those who only run ConMa programs, ConSh among them, are its users, and need not read docs/dev.

## Where to start

- [docs/index.md](docs/index.md): the specification, the primitives and the tools, and, for the developers, the design, the plans and the records (docs/dev).
- [vm/README.md](vm/README.md): the Rust VM — how to build it (Rust 1.75 or later) and run modules, its speed, and the transpiled code.
- [bench/README.md](bench/README.md): benchmarks of the VMs against CPython.
- [consh/README-ja.md](consh/README-ja.md): ConSh, a shell written in ConMa, and its Rust version (in Japanese; [docs/consh/FOR-BASH-USERS.md](docs/consh/FOR-BASH-USERS.md) is in English).
- [consh/js/README-ja.md](consh/js/README-ja.md): ConSh in a browser and in Node.js, with the builtin `js` that calls JavaScript (in Japanese).
