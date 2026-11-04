# ConSh のレビュー

## 2026-11-04: パイプライン、遅延のコマンド置換、`__VProc_start__`、`__OS_wait_writable__`

対象は次のコミットである（72322e3 の後）。

* e4b8529 関数の中の `${1-def}`、`${@-x}` と、`command` に渡る `${s}` の空文字列の Step 3 テスト
* d28abc6 `__VProc_start__` と `__OS_wait_writable__`（ConMa の Step 24、Step 25）
* a849235 パイプライン、チャネル（`consh_chan.se`）、別の VProc で遅延して動くコマンド置換

ConMa のテスト（module1、module2、selfhost1〜7、step1〜25）と ConSh のテスト（expand1、parse1、parse2、step1〜5）は、すべて通る。

ConMa 側に問題は見つからなかった。`check_waiting` は、読む Stream と書く Stream を `select()` の別のリストに渡す。`__VProc_start__` は、クロージャの GVEnv と、親のカレントディレクトリを使う。

### 1. 読まれない `$( )` が最後まで実行されないことがある（中）

```
x=$(mkdir dd)          # スクリプトの最後の文
```

このスクリプトを実行しても、`dd` は作られない。`z=$(sh -c 'echo to-stderr 1>&2')` の標準エラー出力も出ない。

* 原因：`$( )` は子の VProc を起動するだけで、終わるのを待たない。`Eval_run` は最後の文の後すぐに `__OS_exit__` を呼ぶので、まだ動いていない子は打ち切られる。
* PLAN-ja.md 2.1 には「中の副作用は、値を読む前に終わっているとは限らない」とあるが、「スクリプトが終わると、最後まで行われないことがある」とは書いていない。Bash では、`x=$(mkdir dd)` は必ず実行される。
* 修正案：起動した `$( )` の VProc とチャネルを global に記録し、`Eval_run` の最後で、それらのチャネルを cancel してから、VProc が終わるのを待つ。
    * `x=$(mkdir dd)` は、出力がないので最後まで動く。
    * `x=$(yes)` は、次に書いたときに中断されて終わる（SIGPIPE に当たる）。

### 2. チャネルの書き手が状態を読む順序（低）

`consh_chan.se` の `Chan_wait_room` は、`cancelled` を読んでから `consumed` を読み、`consumed` が変わるのを待つ。次の順に動くと、書き手は二度と起きない。

1. 書き手が `cancelled`（まだ偽）を読む。
1. 読み手の `Chan_cancel` が、`cancelled` と `consumed` の両方を書き換える。
1. 書き手が新しい `consumed` を読み、それが変わるのを待つ。この後 `consumed` を変える者はいない。

* 今は再現しない。読み手の時機を 41 通りにずらして試したが、どれも正しく終わった。ラウンドロビンでは、`Chan_cancel` の2つの書き換えの間のステップ数が、書き手が2つを読む間のステップ数より多いからである。正しく動くかどうかが、ステップ数の偶然に頼っている。
* 修正案：`consumed` を先に読み、その後で `cancelled` を読む（1行の入れ替え）。`__LazyList_pipe__` の `__LazyList_pipe_wait__` で stateRef を先に読むのと同じ考え方である。

### 3. 細かい点

* パイプラインの要素が `Sh_abort` で中断すると、リダイレクトで開いたファイル（`for … done 2> file | head -1` など）を `Eval_close` で閉じずに終わる。インタプリタが終われば閉じられる。
* `Chan_cancel_chains` は、`consumed + 0` で同じ数の別のオブジェクトを作り、書き手を起こしている。`__Ref_wait_change__` が値の同一性で比べるので動くが、その意図をコメントに書いておくとよい。

### 対応

* 1：文の終わりに、その VProc で起動した `$( )` を、終わるか、チャネルが一杯で書き手が待つまで待つようにした（PLAN-ja.md 2.1 のコマンド置換）。スクリプトと子の VProc の終わりには、まだ終わっていない `$( )` のチャネルを cancel してから、終わるのを待つ。`x=$(mkdir dd); y=$(mkdir dd/d2)` も、この順に動く（step4 の case6〜8）。
* 2：`Chan_wait_room` で `consumed` を先に読むようにし、理由をコメントに書いた。
* 3：開いたファイルを閉じない件は PLAN-ja.md 2.3 のまま。`Chan_cancel_chains` の `consumed + 0` の意図をコメントに書いた。
