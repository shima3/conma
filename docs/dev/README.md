# Documents for the developers of ConMa

These are for those who change ConMa: its VMs, its front end and its standard library. They are read here on GitHub, in the branch `alpha`, where they are newer than in `main`; GitHub Pages publishes only the documents for the users ([docs/index.md](../index.md)). Those for the developers of ConSh are in [docs/consh/dev](../consh/dev/README.md).

This list is the only list of these documents; a new one is added here.

- [Contributing](CONTRIBUTING.md): the branches (of the work, `alpha` and `main`), the Git settings, how a change goes into `main`, and the tests
- [Design](DESIGN.md)
- [Primitive Implementation Roadmap](ROADMAP.md)
- [Self-hosted Front End](SELFHOST.md)
- [Vproc Interpreter](VPROC_STEP.md)

### Plans and records (in Japanese)
- [Core ConMa and extended ConMa](FAST-PLAN-ja.md): the plan of the two levels of ConMa
- [Bitwise primitives](BITWISE-PLAN-ja.md): the plan of the bitwise operations of extended ConMa
- [Channel primitives](CHAN-PLAN-ja.md): the plan of the channels of ConSh as primitives of extended ConMa
- [Positions in Strings](STRING-INDEX-PLAN-ja.md): the plan of finding positions in ASCII Strings without scanning them
- [Types in transpiled code](TYPES-PLAN-ja.md): the plan of inferring the types of values to make the transpiled VM faster
- [Arrays](ARRAY-PLAN-ja.md): the plan of arrays that are read and written by index in O(1), as primitives of extended ConMa
- [LazyList in the VM](LAZYLIST-PLAN-ja.md): the plan of reading the evaluated cells of a LazyList with primitives of extended ConMa
- [Speed-ups](SPEED-ja.md): every speed-up tried, with its effect and whether it was kept
- [Running the tests and their record](TESTS-ja.md)
- [Reviews of ConMa](REVIEW-ja.md) (those of ConSh are in [ConSh](../consh/dev/README.md))
