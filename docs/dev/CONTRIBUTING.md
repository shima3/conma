# Contributing

This is for the developers of ConMa and of ConSh. A developer of ConMa changes its VMs, its front end or its standard library; one who only runs ConMa programs is a user of ConMa. A developer of ConSh changes ConSh, and is a user of ConMa unless they also change the VM. The documents for the users are in [docs](../index.md) (and consh/docs for ConSh), and those for the developers in docs/dev (and consh/docs/dev); a document for both is in docs.

## Tests

[docs/dev/TESTS-ja.md](TESTS-ja.md) (in Japanese) lists every test suite with what to build first, the known differences, and a record of who ran every suite on which commit. Say in the commit message what you ran to check a change; when you run every suite, also add a line there.

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
