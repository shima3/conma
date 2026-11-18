---
title: ConSh
---

# ConSh

ConSh is a shell written in ConMa (consh/ of the repository). It has a Rust version too, which runs about as fast as Bash; the ConMa version is slower (see [Versions of ConSh and how to run them](RUNNING-ja.html)). The documents of ConMa are in [the index of the documents](../).

The documents of ConSh are listed only here; a new one is added here.

## Docs
- [ConSh for Bash users](FOR-BASH-USERS.html)
- [Specification and design](PLAN-ja.html) (in Japanese)
- [Abstract Syntax Tree of ConSh](AST.html)
- [Versions of ConSh and how to run them](RUNNING-ja.html) (in Japanese)
- [ConSh in your browser](web/): the ConMa version of ConSh on the wasm VM (see consh/js/README-ja.md for what it cannot do here)

## For developers of ConSh (in Japanese)
- [Developing ConSh](dev/DEVELOPING-ja.html)
- [Converting scripts beforehand](dev/COMPILE-PLAN-ja.html) (and its [stage 2](dev/COMPILE2-PLAN-ja.html), the values and contexts of the optimized version), [JavaScript and wasm](dev/JS-PLAN-ja.html), [SJS](dev/SAFEJS-PLAN-ja.html), [SJS on wasm](dev/SJS-WASM-PLAN-ja.html), [arrays of SJS on wasm](dev/SJS-WASM-ARRAY-PLAN-ja.html): the plans
- [Reviews of ConSh](dev/REVIEW-ja.html), [of JS](dev/JS-REVIEW-ja.html), [of SJS](dev/SAFEJS-REVIEW-ja.html)
