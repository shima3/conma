---
title: ConMa
---

# ConMa

ConMa is an intermediate language that provides higher-order functions and continuations for functional programming.

## Files that moved

Files and directories have been moved and renamed. A text that describes the present (a README, the specification, the design, the comments of the code) is changed to the new place. A text that records its own time (the reviews, the plans, the steps of the roadmap) keeps the old path and adds a note after it: "(moved to `NEW` on 2026-11-15)", or "(moved on DATE; now `NEW`)" for a file that moved more than once, and "(removed on DATE)" for one that was removed, with where its content went if it went somewhere; in Japanese, 「（2026-11-15 に NEW へ移動）」, 「（DATE に移動。今は NEW）」 and 「（DATE に削除）」. The date is that of the move. A reference to another document, which the reader follows to read it, is changed to the new place in every text, the reviews too, as the document is the same. Line numbers in such a text are those of its time, and are not changed. The records of docs/dev/TESTS-ja.md name the commit, and get no notes.

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
