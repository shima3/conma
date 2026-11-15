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
- [Contributing](dev/CONTRIBUTING.html): the Git settings, how `alpha` and `main` are used, and the tests
- [Design](dev/DESIGN.html)
- [Primitive Implementation Roadmap](dev/ROADMAP.html)
- [Self-hosted Front End](dev/SELFHOST.html)
- [Vproc Interpreter](dev/VPROC_STEP.html)

### Plans and records (in Japanese)
- [Core ConMa and extended ConMa](dev/FAST-PLAN-ja.html): the plan of the two levels of ConMa
- [Bitwise primitives](dev/BITWISE-PLAN-ja.html): the plan of the bitwise operations of extended ConMa
- [Speed-ups](dev/SPEED-ja.html): every speed-up tried, with its effect and whether it was kept
- [Running the tests and their record](dev/TESTS-ja.html)
- [Reviews of ConMa](dev/REVIEW-ja.html) (ConSh's are in consh/docs/dev)
