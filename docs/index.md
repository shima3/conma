---
title: ConMa
---

# ConMa

ConMa is an intermediate language that provides higher-order functions and continuations for functional programming.

The documents here are for the users of ConMa, who run ConMa programs (and write them, or the tools that make them), and for its developers too; those only for the developers, who change the VMs, the front end or the standard library, are in [dev](dev/CONTRIBUTING.html). A developer of ConSh who does not change the VM is a user of ConMa.

## Docs
- [Specification](SPEC.html)
- [Abstract Syntax Tree](AST.html)
- [Extended Backus–Naur Form](EBNF.html)
- [Terminology](TERMS.html)
- [Primitives](PRIMITIVES.html)

## Tools
- [Lexical Analyzer](LEXER.html)
- [Comment Remover](COMMENT_REMOVER.html)
- [Parser](PARSER.html)
- [Includer](INCLUDER.html)
- [Resolver](RESOLVER.html)

## For developers (dev)
- [Contributing](dev/CONTRIBUTING.html): the branches (of the work, `alpha` and `main`), the Git settings, how a change goes into `main`, and the tests
- [Design](dev/DESIGN.html)
- [Primitive Implementation Roadmap](dev/ROADMAP.html)
- [Self-hosted Front End](dev/SELFHOST.html)
- [Vproc Interpreter](dev/VPROC_STEP.html)

### Plans and records (in Japanese)
- [Core ConMa and extended ConMa](dev/FAST-PLAN-ja.html): the plan of the two levels of ConMa
- [Bitwise primitives](dev/BITWISE-PLAN-ja.html): the plan of the bitwise operations of extended ConMa
- [Channel primitives](dev/CHAN-PLAN-ja.html): the plan of the channels of ConSh as primitives of extended ConMa
- [Positions in Strings](dev/STRING-INDEX-PLAN-ja.html): the plan of finding positions in ASCII Strings without scanning them
- [Types in transpiled code](dev/TYPES-PLAN-ja.html): the plan of inferring the types of values to make the transpiled VM faster
- [Arrays](dev/ARRAY-PLAN-ja.html): the plan of arrays that are read and written by index in O(1), as primitives of extended ConMa
- [LazyList in the VM](dev/LAZYLIST-PLAN-ja.html): the plan of reading the evaluated cells of a LazyList with primitives of extended ConMa
- [Speed-ups](dev/SPEED-ja.html): every speed-up tried, with its effect and whether it was kept
- [Running the tests and their record](dev/TESTS-ja.html)
- [Reviews of ConMa](dev/REVIEW-ja.html) (those of ConSh are in [ConSh](consh/))

## ConSh

ConSh is a shell written in ConMa (consh/ of the repository), with a Rust version that runs about as fast as Bash. Its documents, for its users and its developers, are listed in [ConSh](consh/); those who know Bash may start with [ConSh for Bash users](consh/FOR-BASH-USERS.html).
