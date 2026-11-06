# conma
ConMa is an intermediate language that provides higher-order functions and continuations for functional programming.

## What ConMa offers

- **Continuations everywhere.** Every function is called in continuation-passing style, and the rest of the computation (the CChain) is an ordinary value. `return`, `break`, exceptions, generators and delimited continuations can be written as library code instead of being built into the language. ConSh implements `return`, `break`, `continue`, and `reset`/`shift` this way.
- **Currying by default.** Calling a function with fewer arguments gives a partial application, and extra arguments are applied to the result.
- **Lightweight processes.** VProcs run concurrently under a scheduler that counts steps, so a run can be traced step by step and repeated.
- **A small core.** A program is a few kinds of nodes plus a set of primitives ([PRIMITIVES](docs/PRIMITIVES.md)), which keeps new implementations small: the Python VM (`test/vproc_step.py`, the reference) and the Rust VM (`vm/`) give the same traces step by step.
- **A self-hosted front end.** The lexer, parser, includer and resolver are also written in ConMa ([SELFHOST](docs/SELFHOST.md)).

ConMa is an experimental language. Its VMs are much slower than a native language or CPython (see [REVIEW-ja.md](REVIEW-ja.md)).

## Where to start

- [docs/index.md](docs/index.md): the specification, the design, the primitives and the tools.
- [vm/README.md](vm/README.md): the Rust VM — how to build it (Rust 1.75 or later) and run modules, and its speed.
- [consh/README-ja.md](consh/README-ja.md): ConSh, a shell written in ConMa, and its Rust version (in Japanese; [consh/FOR-BASH-USERS.md](consh/FOR-BASH-USERS.md) is in English).
