# Checklists

The steps of each task of a developer, in the order they are done (2026-11-22). They are reminders, not rules: each step names the part of [docs/dev/CONTRIBUTING.md](CONTRIBUTING.md) (Branches 1 to 6, unless it says otherwise) or [docs/dev/TESTS-ja.md](TESTS-ja.md) that has the rule and its reasons, and when this page and that part disagree, that part is right and this page is fixed. They are written by task, not by role: one machine often does several tasks at once (it develops on one branch and reviews the pull request of another), and who does which task can change.

## A piece of work

1. Make a branch from the latest `alpha`, named after your machine and the work: `m2/foo` (1). For a change of documents only, see below.
2. Commit with a summary of 50 to 70 characters, a blank line, then what changed, why, and what you ran (Commit messages). Make a move of files a commit of its own (Files that moved).
3. When it is done, run the tests the change touches and the benchmarks it affects (2; TESTS-ja.md, 流し方).
4. Unless every file is a document (5) and no benchmark is asked for, write the notes for the tester under a heading of the branch in TESTS-ja.md, 申し送り (2).
5. Merge it into `alpha` and push when the user says so (2), or, when the user wants a pull request or you want a review, open one (below).

## Opening a pull request

1. Push the branch and open it with the template, into `alpha` (`main` for documents only), with a title that begins with your machine (`M2: ...`) and the label of the machine that is to review it, or `wait-user` (6, Opening, Who wrote it, Whose turn).
2. The description begins with your machine (「M2 の PR です」) and says what changed, what you ran and did not run, the known differences seen, and what you want looked at (6, Opening).
3. Add the number of the pull request to your notes in TESTS-ja.md, in a commit of its own, and push it (6, Opening).
4. Look at its checks (`gh pr checks N`) before you hand it to a reviewer (Tests).

## Reviewing a pull request

1. Read it (`gh pr view N --comments`, `gh pr diff N`), fetch the branch, and run the tests the change touches on its head; if it does not merge cleanly into the latest `alpha`, ask the author to take in `alpha` first (6, Review and tests).
2. Answer in one comment that begins 「M2 のレビューです（commit）」: what you ran and its results, then what you found, most important first, with a concrete case; name any check that failed (6, Review and tests; Tests).
3. Change the label to the author's in the same step (6, Whose turn).
4. When nothing is left: 「M2 の確認です（commit）。指摘なし。」, and the label goes back to the author (6, Answers).

## Answering a review

1. Fix it with new commits on the same branch; take in `alpha` by merging it, never by a rebase; push (6, Answers).
2. Comment what changed and in which commit, and change the label back to the reviewer (6, Answers, Whose turn).
3. After 「指摘なし。」, do what is left before a merge (or say that nothing is), and set `wait-user` (6, Answers).

## "Check your PRs"

When the whole message of the user is 「PR確認」, "check your PRs" or "PRs": list the open pull requests with your label and the merged ones that still have it, do what the last comment of each asks, and say which open ones have no `wait-` label. Post the comments without showing them first, changing the label in the same step, and report afterwards what you posted. Never merge on it (6, "Check your PRs").

## Merging

1. Only on the words of the user (6, Merging).
2. Merge into `alpha` (`main`, then `alpha`, for documents only) and push; delete the branch on GitHub and in your clone (2, 5, 6 Merging).
3. When you merged the pull request of another author, say so in a comment and leave their label; when another machine merged yours, check it, delete your branch and remove the label (6, Merging).

## A change of documents only

1. Only files under `docs/` and `.github/` and files named `*.md`; a comment in a source file is not a document (5).
2. Make the branch from the latest `main`; check the links and what a workflow runs; add a line with 「文書だけ」 to the record, docs/dev/TESTS-RECORD-ja.md (5).
3. Write a new document for the developers in English. A change to a document for the users that is in English and in Japanese (`NAME.md` and `NAME-ja.md`) is made to both, in the same commit; `python3 .github/doc-pairs.py` checks their headings (Languages of the documents).
4. When the user says so, merge it into `main` and into `alpha` and push them, or open a pull request into `main` (5, 6).

## A Full Test (the tester)

1. Merge `alpha` into your branch `m2/test` (machine and `test`) and run every suite of TESTS-ja.md, 全部流す (3).
2. If only the known differences show (TESTS-ja.md, 既知の違い): add the line of the record (docs/dev/TESTS-RECORD-ja.md) with the commit of `alpha` you tested, remove the notes of the branches that commit contains (but not a line that asks for a benchmark), and commit (3).
3. When the user says so, merge `m2/test` into `main`, then into `alpha`, and push them, so that the record and the removed notes reach everybody (3).
4. If something fails, tell the author, who fixes it on a new branch (3).

## A Windows Test (I7)

1. Merge `alpha` into `i7/test` and run every suite of TESTS-ja.md, Windows（I7）で流す, with each VM it lists (Tests).
2. Add the line of the record (docs/dev/TESTS-RECORD-ja.md): 「Windows Test」, the commit of `alpha` you ran, and any difference besides the known ones of I7 and of the VM that ran (the rows 「Windows（I7）」 of 既知の違い, and 「wasm 版」 for the wasm VM); remove no notes, and commit (Tests).
3. When the user says so, merge `i7/test` into `alpha` only, never into `main`, and push it (Tests).
4. If something fails that is not a known difference, tell the author of the change, or the user when it is not clear whose it is (Tests; 3).
