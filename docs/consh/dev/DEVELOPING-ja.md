# ConSh の開発

ConSh を変える人（ConSh の開発者）向けの決まりである。ConSh を変えても ConMa の VM を変えなければ、ConMa については利用者である。Git の設定と運用、コミットメッセージの書き方、移したファイルの参照の直し方は、ConMa と共通で [docs/dev/CONTRIBUTING.md](../../dev/CONTRIBUTING.md) にある。テストを全部流す手順は [docs/dev/TESTS-ja.md](../../dev/TESTS-ja.md) に、その記録は [docs/dev/TESTS-RECORD-ja.md](../../dev/TESTS-RECORD-ja.md) にある。

## ファイルの置き場所と名前

* ConSh のために ConMa に足すプリミティブやライブラリは、`consh/` ではなく、リポジトリのルートの `docs/`、`include/`、`test/` に置く。
* `consh/reference/` と `consh/include/` のファイルには、ルートの `include/` のファイルと名前がぶつからないよう、`consh_` を付ける。
* 大きくなったら、`consh_lexer.se`、`consh_expand.se`、`consh_builtin.se` などに分ける。

## 最適化版と参照版

機能を足したり変えたりしたときは、`consh/include` のファイルと、`consh/reference` にある同じ名前のファイルの両方を直し、両方でテストする（`CONSH_IMPL=conma` と `CONSH_IMPL=reference`）。速くしただけなら、最適化版のテストだけでよい。
