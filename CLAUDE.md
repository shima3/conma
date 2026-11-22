# CLAUDE.md

Notes for AI agents (Claude Code) working in this repository, in every clone (M1, M2, M4, I7). Keep it short: it is read at the start of every session. The rules themselves are in docs/dev/CONTRIBUTING.md and docs/dev/TESTS-ja.md; this file lists where to look and the mistakes agents have made. When an agent makes a new mistake worth a line here, it proposes the line in a pull request of documents only (Branches 5), and the user decides whether it goes in.

## Who you are

- Your machine is in `git config user.name` (`m1 Shima`, `m2 Shima`, `m4 Shima`, `i7 Shima`). Every clone uses the same GitHub account (shima3), so begin each PR description and comment with your machine (「M2 のレビューです」) and each PR title with it (`M2: ...`). Docs: CONTRIBUTING, Branches 6.
- Write "the tester", not a machine, for the role: who tests can change.
- Talk to the user in Japanese. In Japanese text, keep proper nouns as they are written in English, untranslated, so that they read as names: transpiled VM (not 変換した VM), Full Test, ConMa, ConJS, VProc.

## Before you act

- The steps of the task at hand (a piece of work, a pull request, a review, an answer, a merge, documents only, a Full Test): docs/dev/CHECKLISTS.md, each step with the part of CONTRIBUTING that has its rule.
- Branches, merging, pull requests, the `wait-` labels and the signal 「PR確認」/"check your PRs"/"PRs": docs/dev/CONTRIBUTING.md, Branches 1–6.
- Push a branch of work when you open its pull request or answer a review (Branches 6). Push `alpha` and `main` only when the user says so (Merging, Branches 5). Merge only on the user's words. Never rebase a pushed commit.
- A change of documents only (`docs/`, `.github/`, `*.md`) goes to `main` by Branches 5. A comment in a `.se`, `.rs`, `.py` or script file is not a document: it moves positions in logs and transpiled code.
- Write documents for developers in English. A document for users in two languages (`NAME.md`, `NAME-ja.md`) is changed in both in the same commit; `python3 .github/doc-pairs.py` checks the pair (CONTRIBUTING, Languages of the documents).
- Commit messages: a summary of 50–70 characters, a blank line, then what changed, why, and what you ran.

## Tests

- What to build and how to run each suite: docs/dev/TESTS-ja.md (流し方, 既知の違い). The record of the Full Tests is docs/dev/TESTS-RECORD-ja.md. Build first: `cd vm && cargo build --release`; `vm/wasm/build.sh` for `CONMA_VM=wasm`.
- Run the wasm VM with Node.js only.
- The Python VM is slow (conjs/test takes 3–4 minutes): run it in the background.
- Do not switch the checkout while tests run in the background: they read the working tree. Use a worktree to test another commit.
- On M1, M2 and M4 the shell is macOS bash 3.2 and BSD sed (`sed -i ''`). I7 is Windows (Git Bash with GNU sed, and PowerShell): the wasm VM builds there, the Rust VM does not (vm/src uses Unix-only APIs: sys.rs, prim/process.rs, value.rs). Before a bulk replace with sed, check how many lines match: a pattern that ends a line in one record often ends many.
- Benchmarks: measure the versions alternately, on the same machine.
- Do not kill processes by a broad pattern (`pkill -f conma`): other sessions on the machine run the same commands. Stop only the processes you started.

## ConJS and SJS (not JavaScript)

Read conjs/README.md (Language, how it differs from JavaScript) before writing ConJS. Agents have tripped on:

- No `async`, `await`, `for await` or `Promise`: a call that waits is an ordinary call (`f.readLine()`, `sleep(ms)`); `spawn` and `join` run tasks.
- `+` joins numbers only: join strings with template literals (`${a}${b}`).
- No `Set`, `Map` or regular expressions.
- Built-ins missing from the README's list may be missing too (`splice`, `Array.from`, `Math.cos`): a missing method fails only when it runs, so try a small program first.
- A call that waits is a point where other tasks run, though nothing marks it. State kept across such a call in shared objects can be changed by another task (the readLine race in shima3/conma#21).
- Node.js versions of ConJS samples sit beside them (`*.node.js`); conjs/test/compare.sh compares them.

## ConMa (`.se`)

Read docs/SPEC.md and docs/PRIMITIVES.md for the language. Things that are easy to get wrong:

- Every function is in continuation passing style: `f a b ,(x) rest` binds the result of `f a b` to `x`. `__Cont_get__ ,(return)` gives an early exit.
- Primitives that can fail take `onError` first (`__print_exit_error__`, or a closure).
- A clause of `__if__` or `__unless__` goes on to what follows it unless it ends in `return ...` of `__Cont_get__`.
- A core Boolean only returns the value it chooses: `b (,() ...) (,() ...)` calls neither clause (`__TRUE__` is `,(t f) __NULL__ t`). To run the chosen clause, use `__if_then_else__ b (...) (...)`.
- The scheduler can switch VProcs between any two steps, and there is no atomic update of a Ref: keep state in one owner VProc and talk to it through channels (include/core/chan.se), and close channel writers you no longer use.
- The Rust VM runs until every VProc has finished: kill VProcs that wait for the OS when the program ends.
