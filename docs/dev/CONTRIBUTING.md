# Contributing

This is for the developers of ConMa and of ConSh. A developer of ConMa changes its VMs, its front end or its standard library; one who only runs ConMa programs is a user of ConMa. A developer of ConSh changes ConSh, and is a user of ConMa unless they also change the VM. The documents for the users are in [docs](../index.md) (and consh/docs for ConSh), and those for the developers in docs/dev (and consh/docs/dev); a document for both is in docs. The rules only for ConSh (where its files go, and changing both its optimized and reference versions) are in [consh/docs/dev/DEVELOPING-ja.md](../../consh/docs/dev/DEVELOPING-ja.md) (in Japanese).

## Tests

[docs/dev/TESTS-ja.md](TESTS-ja.md) (in Japanese) lists every test suite with what to build first, the known differences, and a record of who ran every suite on which commit. Say in the commit message what you ran to check a change; when you run every suite, also add a line there.

## Commit messages

Write the first line as a summary of about 50 to 70 characters, then a blank line, then the details: what changed and why, and what you ran to check it and what you did not (see Tests). Commits before 2026-11-16 are written as one long sentence; they are left as they are.

Make a move of files a commit of its own (`git mv` only), and change the references in the next commit, so that the history of each file follows the move.

## Files that moved

Files and directories have been moved and renamed. A text that describes the present (a README, the specification, the design, the comments of the code) is changed to the new place. A text that records its own time (the reviews, the plans, the steps of the roadmap) keeps the old path and adds a note after it: "(moved to `NEW` on 2026-11-15)", or "(moved on DATE; now `NEW`)" for a file that moved more than once, and "(removed on DATE)" for one that was removed, with where its content went if it went somewhere; in Japanese, 「（2026-11-15 に NEW へ移動）」, 「（DATE に移動。今は NEW）」 and 「（DATE に削除）」. The date is that of the move. A reference to another document, which the reader follows to read it, is changed to the new place in every text, the reviews too, as the document is the same. Line numbers in such a text are those of its time, and are not changed. The records of docs/dev/TESTS-ja.md name the commit, and get no notes.

## Git settings

Several people and sessions commit to `alpha`, whose history is kept linear. Nobody commits to `main` directly: `alpha` is merged into it with a merge commit, so the first-parent history of `main` (`git log --first-parent main`) lists the merges. No hook or check stops a direct commit to `main`; take care yourself. Run these once in each clone; they go to `.git/config`, which is not committed, so pulling does not set them:

```
git config pull.ff true
git config pull.rebase true
git config pull.autostash true
git config merge.ff only
git config branch.main.mergeoptions --no-ff
git config branch.main.rebase merges
```

* `git pull` fast-forwards when it can, and otherwise puts your local commits on top of what was pulled (rebase) instead of making a merge commit; changes in the working tree are stashed and put back.
* `git merge` only fast-forwards on other branches, and stops when the branches have diverged.
* On `main`, `git merge alpha` always makes a merge commit (`branch.main.mergeoptions`), and `git pull` keeps merge commits when it rebases (`branch.main.rebase merges`; a plain rebase would flatten them). Pull `main` before merging into it.
* A fix for `main` is committed to `alpha` (or to a branch made from `main` and merged with `--no-ff`) and then merged. If you commit to `main` by mistake and have not pushed it, move the commit to `alpha` (`git cherry-pick`) and take it off `main` with `git reset --keep HEAD~1`; if you have pushed it, tell the others before you change `main` again.
* `pull.ff true` is there to override a global `pull.ff only` (`git config --global`), which would stop `git pull` before it could rebase; a setting of the clone comes before a global one (`git config --show-origin --get-regexp '^(merge|pull)\.'` shows which applies).
