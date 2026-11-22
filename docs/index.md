---
title: ConMa
---

# ConMa

ConMa is an intermediate language that provides higher-order functions and continuations for functional programming.

The documents here are for the users of ConMa, who run ConMa programs (and write them, or the tools that make them), and for its developers too; those only for the developers, who change the VMs, the front end or the standard library, are on GitHub ([docs/dev](https://github.com/shima3/conma/tree/alpha/docs/dev); see [For developers](#for-developers)). A developer of ConSh who does not change the VM is a user of ConMa.

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

## For developers

The documents for those who change ConMa (its VMs, its front end and its standard library) are not published here: they are read on GitHub, in the branch `alpha`, where they are newer ([docs/dev](https://github.com/shima3/conma/tree/alpha/docs/dev), which lists them, beginning with [Contributing](https://github.com/shima3/conma/blob/alpha/docs/dev/CONTRIBUTING.md)).

## ConSh

ConSh is a shell written in ConMa (consh/ of the repository), with a Rust version that runs about as fast as Bash. Its documents, for its users and its developers, are listed in [ConSh](consh/); those who know Bash may start with [ConSh for Bash users](consh/FOR-BASH-USERS.html).

## ConJS

ConJS is a subset of JavaScript with lightweight threads and blocking calls, made on SJS (conjs/ and sjs/ of the repository). Its documents, and programs of it that run in your browser, are listed in [ConJS](conjs/).
