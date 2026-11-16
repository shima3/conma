# Contributing

This is for the developers of ConMa and of ConSh. A developer of ConMa changes its VMs, its front end or its standard library; one who only runs ConMa programs is a user of ConMa. A developer of ConSh changes ConSh, and is a user of ConMa unless they also change the VM. The documents for the users are in [docs](../index.md) (and consh/docs for ConSh), and those for the developers in docs/dev (and consh/docs/dev); a document for both is in docs. The rules only for ConSh (where its files go, and changing both its optimized and reference versions) are in [consh/docs/dev/DEVELOPING-ja.md](../../consh/docs/dev/DEVELOPING-ja.md) (in Japanese).

## Tests

[docs/dev/TESTS-ja.md](TESTS-ja.md) (in Japanese) lists every test suite with what to build first, the known differences, and a record of who ran every suite on which commit. Say in the commit message what you ran to check a change; when you run every suite, also add a line there.

## Languages of the documents

A document in English is named `NAME.md`, and one in Japanese `NAME-ja.md`. When a document is in both, the English one is the original: the Japanese one begins with 「原文は英語。古くなっていることがある」 and a link to `NAME.md`, and a change is made to the English one first.

## Commit messages

Write the first line as a summary of about 50 to 70 characters, then a blank line, then the details: what changed and why, and what you ran to check it and what you did not (see Tests). Commits before 2026-11-16 are written as one long sentence; they are left as they are.

Make a move of files a commit of its own (`git mv` only), and change the references in the next commit, so that the history of each file follows the move.

## Files that moved

Files and directories have been moved and renamed. A text that describes the present (a README, the specification, the design, the comments of the code) is changed to the new place. A text that records its own time (the reviews, the plans, the steps of the roadmap) keeps the old path and adds a note after it: "(moved to `NEW` on 2026-11-15)", or "(moved on DATE; now `NEW`)" for a file that moved more than once, and "(removed on DATE)" for one that was removed, with where its content went if it went somewhere; in Japanese, 「（2026-11-15 に NEW へ移動）」, 「（DATE に移動。今は NEW）」 and 「（DATE に削除）」. The date is that of the move. A reference to another document, which the reader follows to read it, is changed to the new place in every text, the reviews too, as the document is the same. Line numbers in such a text are those of its time, and are not changed. The records of docs/dev/TESTS-ja.md name the commit, and get no notes.

## Branches and Git settings

GitHub has two branches: `main`, which gets only states that passed every suite, and `alpha`, where the work of everybody comes together. Nobody commits to either directly: changes come in by merges, so the first-parent history of `main` (`git log --first-parent main`) lists the tested states. No hook or check stops a direct commit; take care yourself.

1. **A piece of work** (a feature, a fix, a plan and its stages) has a short branch of its own, made from the latest `alpha` in the clone where it is done and named after the machine and the work: `m4/chan`, `m1/string-index` (the Apple M1, M2 and M4 machines are `m1`, `m2` and `m4`, so that the tables of times say where they were measured). Only one person or session writes to it. It stays in the clone; push it to GitHub only to keep a long piece of work safe or to go on with it on another machine, and delete it there when it is merged.
2. **When the work is done**, its author runs the tests the change touches and the benchmarks it affects, and says in the commit messages what they ran (see Tests). They write their notes for the tester in docs/dev/TESTS-ja.md, section 申し送り, under a heading of the branch (what to rebuild and in which order, the new tests, what they ran and did not run, anything to watch for), and commit them on the branch. Then they merge it into `alpha` and push: `git checkout alpha`, `git pull`, `git merge m4/chan`, `git push`; if the push is refused because somebody pushed in the meantime, `git pull` and `git push` again. The branch is then deleted (`git branch -d m4/chan`). A piece of work that goes on after that gets a new branch from `alpha`.
3. **The tester** keeps a branch of their own in their clone, `test/m2` for example (`test/` and the machine). They merge `alpha` into it (`git pull` on `alpha`, then `git merge alpha` on `test/m2`) and run every suite of docs/dev/TESTS-ja.md there. If it passes, they add the line of the record (the commit of `alpha` they tested), remove the notes of the branches that commit contains, and commit on `test/m2`. Then they merge `test/m2` into `main` (`git checkout main`, `git pull`, `git merge test/m2`, `git push`), so that `main` gets exactly what was tested and not what reached `alpha` during the tests, and into `alpha` (`git checkout alpha`, `git pull`, `git merge test/m2`, `git push`), so that the record and the removed notes reach everybody. If something fails, they tell the author, who fixes it on a new branch.

A commit that has been pushed is never rebased, so that the hashes written in documents and messages stay right: `alpha` is pulled with a merge (`branch.alpha.rebase false` below), and a branch takes in `alpha` with `git merge alpha`, not `git rebase`. From 2026-11-15 to 2026-11-16 everybody committed to `alpha` directly, kept linear by rebasing on pull; records that say "alpha の xxxxxxx" name those commits, which stay in `alpha` and `main`.

Run these once in each clone; they go to `.git/config`, which is not committed, so pulling does not set them:

```
git config merge.ff true
git config pull.rebase true
git config pull.ff true
git config pull.autostash true
git config branch.main.mergeoptions --no-ff
git config branch.main.rebase merges
git config branch.alpha.mergeoptions --no-ff
git config branch.alpha.rebase false
```

* `git pull` fast-forwards when it can. Otherwise, on `alpha` it makes a merge commit (`branch.alpha.rebase false`), and on other branches it puts your local commits that are not pushed on top of what was pulled (rebase); changes in the working tree are stashed and put back.
* `git merge` fast-forwards when it can and otherwise makes a merge commit; on `main` and `alpha` it always makes a merge commit (`branch.main.mergeoptions`, `branch.alpha.mergeoptions`), so that the first-parent history of `alpha` (`git log --first-parent alpha`) lists the pieces of work merged into it. `git pull` still fast-forwards on them when it can (`pull.ff true` comes before the merge options), so a pull with nothing of your own makes no empty merge commit. Until 2026-11-16 the clones set `merge.ff only`, which stops such a merge; `merge.ff true` replaces it, and also overrides a global `merge.ff false`, which would make a merge commit even where a fast-forward is possible.
* On `main`, `git merge test/m2` always makes a merge commit (`branch.main.mergeoptions`), and `git pull` keeps merge commits when it rebases (`branch.main.rebase merges`; a plain rebase would flatten them). Pull `main` before merging into it.
* If you commit to `main` or `alpha` by mistake and have not pushed it, move the commit to a branch of the work (`git branch m4/fix` there, then `git reset --keep HEAD~1` on `main` or `alpha`); if you have pushed it, tell the others before you change that branch again.
* `pull.ff true` is there to override a global `pull.ff only` (`git config --global`), which would stop `git pull` before it could rebase or merge; a setting of the clone comes before a global one (`git config --show-origin --get-regexp '^(merge|pull|branch)\.'` shows which applies).
