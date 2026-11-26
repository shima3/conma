# Documents for the developers of ConMa

These are for those who change ConMa: its VMs, its front end and its standard library. They are read here on GitHub, in the branch `alpha`, where they are newer than in `main`; GitHub Pages publishes only the documents for the users ([docs/index.md](../index.md)). Those for the developers of ConSh are in [docs/consh/dev](../consh/dev/README.md).

This list is the only list of these documents; a new one is added here. They are written in English; the plans and records below stay in Japanese ([Contributing](CONTRIBUTING.md), Languages of the documents).

- [Contributing](CONTRIBUTING.md): the branches (of the work, `alpha` and `main`), the Git settings, how a change goes into `main`, and the tests
- [Design](DESIGN.md)
- [Primitive Implementation Roadmap](ROADMAP.md)
- [Self-hosted Front End](SELFHOST.md)
- [Vproc Interpreter](VPROC_STEP.md)
- [How ConJS is made, and its tests](CONJS.md): the parts of ConJS and its scheduler, and what each step of conjs/test checks (the language is in [conjs/README.md](../../conjs/README.md))

### Plans and records (in Japanese)
- [Core ConMa and extended ConMa](FAST-PLAN-ja.md): the plan of the two levels of ConMa
- [Bitwise primitives](BITWISE-PLAN-ja.md): the plan of the bitwise operations of extended ConMa
- [Channel primitives](CHAN-PLAN-ja.md): the plan of the channels of ConSh as primitives of extended ConMa
- [Positions in Strings](STRING-INDEX-PLAN-ja.md): the plan of finding positions in ASCII Strings without scanning them
- [Types in transpiled code](TYPES-PLAN-ja.md): the plan of inferring the types of values to make the transpiled VM faster
- [Arrays](ARRAY-PLAN-ja.md): the plan of arrays that are read and written by index in O(1), as primitives of extended ConMa
- [LazyList in the VM](LAZYLIST-PLAN-ja.md): the plan of reading the evaluated cells of a LazyList with primitives of extended ConMa
- [Platforms and languages translated to ConMa](PLATFORM-PLAN-ja.md): the plan of running ConMa on more platforms with their own features, and of a kit for making languages that are translated to ConMa, as SJS is
- [ConJS on the host's JavaScript](CONJS-HOST-PLAN.md): the plan of running the functions of ConJS on the JavaScript engine of the host on the wasm VM, calling the VM back for what the host cannot do
- [Command lines in ConJS](CONJS-SHELL-PLAN.md): the plan of lines that begin with `$ ` in a ConJS file, running external commands and command functions with redirections from and to variables
- [Speed-ups](SPEED-ja.md): every speed-up tried, with its effect and whether it was kept
- [Checklists](CHECKLISTS.md): the steps of each task (a piece of work, a pull request, a review, a Full Test), with the rules they come from in [Contributing](CONTRIBUTING.md)
- [Running the tests](TESTS-ja.md): what to build, how to run every suite, the known differences and the notes for the tester
- [The record of the tests](TESTS-RECORD-ja.md): who ran every suite on which commit
- [Reviews of ConMa](REVIEW-ja.md) (those of ConSh are in [ConSh](../consh/dev/README.md))
