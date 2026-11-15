# ConMa
ConMa is an intermediate language that provides higher-order functions and continuations for functional programming.

## What ConMa offers

- **Continuations everywhere.** Every function is called in continuation-passing style, and the rest of the computation (the CChain) is an ordinary value. `return`, `break`, exceptions, generators and delimited continuations can be written as library code instead of being built into the language. ConSh implements `return`, `break`, `continue`, and `reset`/`shift` this way.
- **Currying by default.** Calling a function with fewer arguments gives a partial application, and extra arguments are applied to the result.
- **Lightweight processes.** VProcs run concurrently under a scheduler that counts steps, so a run can be traced step by step and repeated.
- **A small core.** A program is a few kinds of nodes plus a set of primitives ([PRIMITIVES](docs/PRIMITIVES.md)), which keeps new implementations small. There are four: the Python VM (`vm/python/vproc_step.py`, the reference) and the Rust VM (`vm/`), which give the same traces step by step; the transpiled VM, the Rust VM with Rust code made from ConMa modules ahead of time (`vm/conma2rust`); and the Rust VM built for WebAssembly, which runs ConSh in a browser and in Node.js (`consh/js`).
- **A self-hosted front end.** The lexer, parser, includer and resolver are also written in ConMa ([SELFHOST](docs/SELFHOST.md)). The Rust VM embeds it, so `conma build` and `conma run` turn source files into modules and run them without Python.

ConMa is an experimental language. Its interpreting VMs (Python and Rust) are much slower than CPython; the transpiled VM, which runs Rust code made from the program ahead of time, is about as fast as CPython 3.9 on the benchmarks of [bench/](bench/README.md), which also says what those results do not show.

## Try it

Build the Rust VM and run ConSh, the shell written in ConMa, on it (interactive mode; `exit` or Ctrl-D ends it):

```
cd vm && cargo build --release && cd ..
CONSH_IMPL=conma CONMA_VM=rust consh/bin/consh
```

Run the tests with the Python VM (the default) or another VM (`bin/vm`); they pass when nothing but the `== name` line of each test is printed:

```
test/run_all.sh -j 8
CONMA_VM=rust test/run_all.sh -j 8
```

[docs/TESTS-ja.md](docs/TESTS-ja.md) (in Japanese) lists every test suite with what to build first, the known differences, and a record of who ran every suite on which commit. Say in the commit message what you ran to check a change; when you run every suite, also add a line there.

## Git settings

Several people and sessions commit to `alpha`, whose history is kept linear. Nobody commits to `main` directly: `alpha` is merged into it with a merge commit, so the first-parent history of `main` (`git log --first-parent main`) lists the merges. No hook or check stops a direct commit to `main`; take care yourself. Run these once in each clone; they go to `.git/config`, which is not committed, so pulling does not set them:

```
git config pull.ff true
git config pull.rebase true
git config pull.autostash true
git config merge.ff only
git config branch.main.mergeoptions --no-ff
git config branch.main.rebase merges
```

* `git pull` fast-forwards when it can, and otherwise puts your local commits on top of what was pulled (rebase) instead of making a merge commit; changes in the working tree are stashed and put back.
* `git merge` only fast-forwards on other branches, and stops when the branches have diverged.
* On `main`, `git merge alpha` always makes a merge commit (`branch.main.mergeoptions`), and `git pull` keeps merge commits when it rebases (`branch.main.rebase merges`; a plain rebase would flatten them). Pull `main` before merging into it.
* A fix for `main` is committed to `alpha` (or to a branch made from `main` and merged with `--no-ff`) and then merged. If you commit to `main` by mistake and have not pushed it, move the commit to `alpha` (`git cherry-pick`) and take it off `main` with `git reset --keep HEAD~1`; if you have pushed it, tell the others before you change `main` again.
* `pull.ff true` is there to override a global `pull.ff only` (`git config --global`), which would stop `git pull` before it could rebase; a setting of the clone comes before a global one (`git config --show-origin --get-regexp '^(merge|pull)\.'` shows which applies).

## Where to start

- [docs/index.md](docs/index.md): the specification, the design, the primitives and the tools.
- [vm/README.md](vm/README.md): the Rust VM — how to build it (Rust 1.75 or later) and run modules, its speed, and the transpiled code.
- [bench/README.md](bench/README.md): benchmarks of the VMs against CPython.
- [consh/README-ja.md](consh/README-ja.md): ConSh, a shell written in ConMa, and its Rust version (in Japanese; [consh/docs/FOR-BASH-USERS.md](consh/docs/FOR-BASH-USERS.md) is in English).
- [consh/js/README-ja.md](consh/js/README-ja.md): ConSh in a browser and in Node.js, with the builtin `js` that calls JavaScript (in Japanese).
