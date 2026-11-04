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

## 2026-11-04: 変数リストへのリダイレクト、`stdin`、前回の指摘への対応

対象は次のコミットである（87cfb5a の後）。

* caaf247 `> (a b . rest)`、`2> (a b . rest)`、`< (a b . rest)`、組み込みコマンド `stdin`、標準出力と標準エラー出力が別々の sink に行く場合
* 977ea50 前回の 1〜3 への対応

ConSh のテスト（expand1、parse1、parse2、step1〜6）は、すべて通る。ConMa 本体は変わっていない。

前回の 1 と 2 は直っている。`x=$(mkdir dd)` だけのスクリプトで `dd` が作られ、`y=$(sh -c 'echo made2 > g'); cat g` で `made2` が出て、`z=$(sh -c 'echo to-stderr 1>&2')` の標準エラー出力も出る。

### 1. 変数リストの組の `unsatisfied` を、2つの VProc が同時に書き換えうる（低）

`cmd > (a) 2> (b)` では、2つのリダイレクトの記録が同じ組に入り、別々の VProc から書かれる。

* 標準出力は、`External_run` の VProc が読んで書く。
* 標準エラー出力は、`Pump_start` の VProc が読んで書く。

`Capture_add` は、組の `unsatisfied` を `__Ref_get__`、減算、`__Ref_set__` の数ステップで減らす。2つの VProc がこの間を同時に通ると、片方の減算が失われ、`unsatisfied` が 0 にならない。止まらないコマンドでは、そのまま終わらない。

* 今は再現しない。両方の出力が同時に読める状態から始めても、`a=o b=e status=0` で正しく止まった。前回の 2 と同じく、正しく動くかどうかがステップの並びの偶然に頼っている。
* 修正案：次のどちらかにする。
    * `unsatisfied` を組で共有せず、記録ごとに「変数の数だけ受け取ったか」の Ref を持たせる。止めるかどうかは、書くたびに組のすべての記録を見て決める。各 Ref を書くのは1つの VProc だけになる。
    * 組の更新を1つの VProc にまとめる。

### 2. `$( )` の VProc が実行時エラーで止まると、文の終わりで待ち続ける（低）

`Subst_settle_one` は、状態が "running" でなくなるまで待つ。"done" にするのは `Sh_finish` である。

`Sh_error` はインタプリタごと終わるので問題ない。しかし ConMa の実行時エラー（VProcError）で子の VProc だけが止まると、`Sh_finish` を通らず、状態が "running" のまま残る。親は文の終わりで待ち続ける。

* ほかに動く VProc がなければ、Deadlock（終了コード 2）になる。外部コマンドを待つ VProc などがあれば、止まったままになる。
* ConSh のコードに誤りがあるときにしか起きないが、調べにくくなる。
* 修正案：`__VProc_wait__` で子の終わりも待つか、待つ前に VProc が終わっていないか確かめる。

### 問題なしと確かめたところ

* 変数リストが一杯になった後の書き込みは、`Capture_run` の guard で止まる（SIGPIPE に当たる）。止まったときの `$?` は 0 である。
* `return`、`break`、`continue` で抜けるときも、`Capture_assign` を通る（`Eval_redirected`、`Loops_closing`）。
* シェルの標準入力と `< ファイル` は、まだ `stdin` で読んでいなければ Stream をそのまま外部コマンドに渡し、読んだ後は要素を書く VProc を通す。PLAN-ja.md 2.3 の制限と合っている。
* 外部コマンドの標準エラー出力は別の VProc で読み、`External_run` はその VProc を待ってから `__OS_wait__` に進む。

### 対応

* 1：1つ目の修正案にした。組の `unsatisfied` をなくし、止めるかどうかは、書くたびに組のすべての記録の `count` を見て決める（`Capture_satisfied`）。各記録の `count` を書くのは、その記録に書く VProc だけになる。止まらないコマンドの標準出力と標準エラー出力を別々の変数リストにするテストを、step6 の case11 に加えた。
    * 1つの記録にいくつもの VProc が同時に書く場合（`{ f & g; } > (a b)`）は、`count` の更新が競合しうる。PLAN-ja.md 2.3 の制限に書いた。
* 2：`$( )` の VProc ごとに、その終わりを `__VProc_wait__` で待つ VProc を起動し、終わったら `Sh_release` で状態を "done" にして、出力の書き手を閉じるようにした（`Subst_watch`）。
    * 正常に終わったときは、すでに `Sh_finish` が同じことをしているので、何も変わらない。
    * `$( )` の本体で実行時エラーを起こすように一時的に書き換えて試した。直す前は Deadlock（終了コード 2）だったが、直した後は次の文に進み、値は空になる。

## 2026-11-05: ジョブ、`kill`、対話モード、外部コマンドの直接の接続、glob、ConMa 版の構文解析

対象は、01aa74f の後の次の 23 コミット（35fb216〜432a942）である。

* ConMa：Step 26〜31（`__VProc_kill__`、`__VProc_id__`、`__OS_getpid__`、`__OS_kill__`、`__OS_sysconf__`、`__String_find__`、`__OS_listdir__`、`__Sexp_read__`、`__OS_trap_interrupt__`）、S 式の読み込みの高速化、スケジューラが OS を調べる間隔、util.se の行の読み込み
* ConSh：
    * ジョブ（2338a5a）
    * 前回の指摘への対応（5d8b044）
    * 外部コマンドの直接の接続（16cccb4、825a175）
    * ARG_MAX（6c5940d）
    * チャネルの読み手（85cea09）
    * glob と `${-語}`（305be8f）
    * ConMa 版の構文解析（50e4650、03c5daa）
    * 対話モードと rlwrap（0051783、e6764e8）
    * リダイレクトで開いたファイルを閉じる（2c98587）
    * 止めたコマンドが起動した VProc も止める（432a942）

ConMa のテスト（module1、module2、selfhost1〜7、step1〜31）と ConSh のテスト（expand1、parse1〜3、step1〜13）は、すべて通る。

前回の 1 と 2 は、`Capture_satisfied` と `Subst_watch` で直っている。

### 1. 評価中の LazyList を持つ VProc を止めると、その LazyList が読めなくなる（高）

```
x=$(seq 1 2000)
function eat (l) { for i in $l; do echo $i > /dev/null; done; }
eat $x &
j=$!
sleep 1
kill $j
wait $j
echo $x | wc -l        # __LazyList_claim__: VProc 4 finished without evaluating the LazyList
```

* `__VProc_kill__` は、LazyList を claim して評価している VProc も、そのまま終わらせる。その後、同じ LazyList を読む VProc はエラーになる。
* 対話モードでは、`stdin > (a)` を Ctrl-C で止めると、シェルの標準入力の LazyList が壊れ、次の行を読むところでシェルごと終わる。前のコマンドの `$( )` の値を読んでいる最中に Ctrl-C を押した場合も、同じことが起こる。
* 修正案：次のどちらかにする。
    * kill を、VProc が LazyList の評価を終えるまで延ばす。
    * claim した LazyList を未評価に戻す。ただし `Chan_next` は、値を返す前に状態の Dict を書き換えるので、評価し直すと要素を飛ばしうる。

### 2. `&` のジョブや読まれていない `$( )` が、文の終わりに閉じたファイルに書く（高）

```
function g () { sleep 1; echo late; /bin/echo ext-late; }
function f () { echo early; g & }
f > out4.txt           # consh: sleep: closed output Stream で、シェルごと終わる
wait
```

* `Open_release` は、文の終わりに、その文のリダイレクトで開いたファイルを閉じる。`Child_target` は、ファイルの Stream を子にそのまま渡すので、文より長く生きるジョブや `$( )` は、閉じた Stream に書く。Bash では、`late` と `ext-late` もファイルに書かれる。
* `f > (y) 2> err.txt` で、`f` の中の `$( )` が遅れて標準エラー出力に書く場合も、後で `$y` を読むとエラーになる。
* 組み込みコマンドの `echo` では以前から起きていたが、825a175 で外部コマンドもファイルの Stream を直接受け取るようになったので、起きやすくなった。
* 修正案：開いた Stream を参照数で管理し、ジョブや `$( )` が受け継いだら数を増やし、`Sh_release` で減らす。または、子に渡すときに fd を dup する。

### 3. OS パイプの要素が spawn の前にエラーになると、次の要素が止まったまま戻らない（高、対話モード）

```
ARG_MAX=3 /bin/echo hi | cat       # エラーを出した後、プロンプトに戻らない
/bin/cat < d* | cat                # 曖昧なリダイレクト。同じ
```

* `Pipe_element` は、`Eval_stmt` が普通に戻ったときだけ `Eval_close owned` でパイプの書く端を閉じる。対話モードでは `Sh_error` が `exitTo` に飛ぶので閉じられず、`Sh_release` も `owned` を閉じない。そのため、読む側の `cat` に EOF が届かない。spawn の前に kill した場合も同じである。
* スクリプトでは、エラーでシェルが終わるので起きない。
* 修正案：`Sh_release` で `owned` を閉じる（すでに閉じていてもよいようにする）。または、パイプの端も `openRef` に記録する。

### 4. 先頭の語が glob で関数名に展開されるときも、OS パイプにする（高）

```
function foo () { /bin/echo one; /bin/echo two; echo three }
cd g1                  # g1/foo がある
f?o | cat              # one の後、/bin/echo: closed output Stream で、シェルごと終わる
```

* `Pipe_is_external` は展開前の字面で外部コマンドかを決めるので、`f?o` は外部コマンドとして OS パイプになる。実行時には glob で関数 `foo` に展開される。関数の中の最初の外部コマンドが、コピーされた `sh` の `owned` を閉じるので、2つ目の外部コマンドは閉じた Stream に書く。読む側でも同じことが起こる。
* 修正案：`Pipe_is_external` で `*?[` を含む語を除く。閉じた `owned` を `sh` から外すか、その要素の VProc だけが閉じるようにする。

### 5. ConMa の実行時エラーにならず、インタプリタ全体が Python の例外で終わる（中）

* `__OS_kill__` に無限大や NaN を渡すと、範囲を調べる前に `int(sig.value)` を呼ぶので、OverflowError や ValueError でインタプリタ全体が終わる（test/vproc_step.py:999）。修正案：先に `math.isfinite` で調べる。
* `__Sexp_read__` は入れ子1段ごとに再帰するので、約 1000 段で RecursionError になり、インタプリタ全体が終わる。ConSh は構文木をこれで読むので、深く入れ子にしたスクリプトでシェルが終わりうる。修正案：明示的なスタックで読むか、RecursionError を onError に渡す。

### 6. `__OS_trap_interrupt__` の Ref だけを待つ VProc があると、Deadlock になる（中）

```
__Ref_new__ "a" ,(ref)
__OS_trap_interrupt__ ref ,()
__Ref_get__ ref ,(v)
__Ref_wait_change__ ref v ,()      # すぐに Deadlock（終了コード 2）
```

* スケジューラは、OS を待つ VProc（`self.waiting`）があるときだけ `select()` に入る。Ref の変化を待つ VProc しかなければ、Ctrl-C で起きうるのに Deadlock とする。
* 対話モードで、ConMa の中だけで待ち続けるコマンドを Ctrl-C で止めようとしても、その前にシェルが終わりうる。
* 修正案：trap があり、`changes` に待つ VProc があれば、Deadlock とせず、wakeup の fd で `select()` する。

### 7. ブラケットの中のクォートやエスケープされた `-`、`!` が、ブラケットの意味のままになる（中）

| パターン（ファイルは `a b c - ]`） | Bash | ConSh |
|---|---|---|
| `[a\-c]` | `- a c` | `a b c` |
| `[a"-"c]` | `- a c` | `a b c` |
| `["!"a]` | `a` | `- ] b c` |

* `Glob_escape`（consh_glob.se:14）と consh_parse の `glob.escape` は、`*?[\` しかエスケープしない。
* 修正案：クォートの中の文字は、すべて（少なくとも `-!]^` も）エスケープする。`Glob_class_items` は `\c` を扱える。

### 8. `kill` で止めたジョブのパイプの fd が漏れる（中）

* `/bin/sleep 31 | f &` を `kill` するたびに、パイプの fd が1つ漏れる。`/bin/echo $(/bin/sleep 20) | /bin/cat &` では2つ漏れる。
* `External_run` は、普通に終わったときだけ reader を閉じる。`Sh_release` は `owned` を閉じない。
* SIGTERM を送った外部コマンドは `__OS_wait__` されないので、終了するまでゾンビとして残る。
* 修正案：パイプの端を `Open_track` するか、`Sh_release` で `owned` を閉じる。`VProc_stop` で SIGTERM の後、別の VProc で `__OS_wait__` する。

### 9. パイプラインのジョブへの `wait` が、最後の要素しか待たない（中）

```
function slow () { sleep 2; echo first-done >> f10; }
slow | /usr/bin/true &
wait
cat f10                # ない。スクリプトが終わると、slow も打ち切られる
```

* 修正案：`Job_wait` で、`Pipe_wait` と同じく `"members"` のすべての VProc を待つ。

### 10. 細かい点

* 終わったジョブを `kill` すると、`wait` は 143 を返す（Bash は実際の終了ステータス）。`Job_kill` は、VProc がまだ終わっていないときだけ "killed" にするとよい。
* util.se の行の読み込みは、長い1行ではまだ長さの2乗の時間がかかる。`__Stream_lines__` で1行 128 KB は 1.0 秒、256 KB は 3.5 秒、512 KB は 12.9 秒だった。ConMa の String はエスケープしたまま持ち、`__String_find__` なども呼ぶたびに全体のエスケープを外すので、塊ごとに残り全体をなめる。PLAN-ja.md 2.3 の「40000 文字の1行は速い」は正しいが、もっと長いと遅くなる。修正案：新しい塊だけを探し、残りは断片のリストで持ち、改行が見つかったときに1回だけつなぐ。
* 対話モード（読んで見つけたもの。再現はしていない）：
    * 標準入力が端末でないとき（SIGINT がインタプリタにだけ届く）、Ctrl-C で止めたコマンド自身の外部コマンドに SIGTERM が送られない（`Kill_tree` は、パイプの要素と `$( )` しか見ない）。
    * Ctrl-C で止めたコマンドの `"recover"` が残る。コマンドの間に REPL の VProc で `Sh_error` が起きると、そこに飛ぶ。
    * リダイレクトした文から `break`、`continue`、`return` で抜けると、閉じた Stream が開いたファイルの表に残る。ループの中で繰り返すと表が伸び、`Open_track` の `List_length` が遅くなる。
    * `CONSH_NO_RLWRAP=1` が、対話モードで起動するコマンドすべてに渡る。中で起動した対話モードの consh は rlwrap を使わない。
* `__Sexp_read__` で、文字列の最後が `\` のとき（`"abc\`）、エラーの列が入力の外（column 7）になる。
* `*/.` と `*/..` は何にも一致しない（Bash は `d1/.` などに一致する）。`Glob_add_literal` が `__OS_listdir__` で存在を調べ、それが `.` と `..` を返さないからである。
* チャネルの別々の鎖に書かれた行は、届いた順にならないことがある（`echo a$i; echo b$i | cat; echo c$i` を遅い読み手に渡すと `a1 b1 b2 b3 b4 c1 …`）。85cea09 の前からある。
* チャネルの読み手は、`Chan_register` で鎖を調べた後、知っている鎖が閉じたのを見て終わる。その間に書き手が新しい鎖を加え、知っている鎖を閉じると、新しい鎖を読まずに終わりうる（再現しない）。修正案：知っている鎖がなくなったら、もう一度 `Chan_register` し、加わらなければ終える。
* ARG_MAX は環境変数を数えない。環境変数が大きいと spawn が E2BIG で失敗し、終了ステータスは 127 になる（Bash は 126）。

`&` で起動した外部コマンドにも対話モードの Ctrl-C が届くことと、Ctrl-C で止めたコマンドの `cd` がシェルに残らないことは、PLAN-ja.md 2.3 に書かれているので、指摘から除いた。

### 問題なしと確かめたところ

* `__VProc_kill__`：
    * 待ち状態（Ref、OS、他の VProc、LazyList）にかかわらず止まり、それを待つ VProc は起きる。
    * 自分自身や、終わった VProc を止めてもよい。
    * 止めた VProc が claim していた LazyList を待つ VProc は、エラーを受け取り、止まったままにはならない。
* SIGINT：C の handler が wakeup の fd に書いてから Python の handler が Ref を変えるので、取りこぼさない。
* スケジューラ：64 ステップごとの OS の確認で、飢餓や busy loop は起きない。
* `__String_find__`、`__OS_sysconf__`、`__OS_listdir__`、S 式の読み込みの書き換え：境界の場合も含めて問題ない。
* `kill`、`wait`、`exit`：
    * `kill` と `wait` の終了ステータスは、止めたジョブで 143、2回目の `wait` で 127、ないジョブで 1 である。
    * `exit` は、ジョブの中では `wait` に、パイプの要素の中ではパイプに終了ステータスを返し、`$( )` の中ではその VProc だけを終える。
    * 止めたパイプラインのジョブの外部コマンドには、SIGTERM が届く。
* OS パイプ：
    * 書く側が終わる、または起動に失敗すると、読む側に EOF が届く。
    * 外部コマンドと関数を混ぜたパイプも動く。
    * 失敗するパイプを繰り返しても、fd は増えない。
* 関数の中で、組み込みコマンドと外部コマンドが同じ端末やファイルに書くとき（`2>&1`、`>>` も）、出力の順序は Bash と同じである。
* glob：
    * `[!a]`、`[]a]`、`[a-]`、範囲、`.*`、`**`、シンボリックリンク、読めないディレクトリ、glob の文字を含む名前、`\*`、`"x*"`、パターンを持つ `$x` は、Bash と同じである。
    * `${-…}` は、空の場合も前後に文字列が付く場合も正しく展開される。
    * リダイレクト先が 0 個や 2 個以上に一致すると、エラーになる。
* ConMa 版の構文解析：
    * 200 以上のテストスクリプトと、約 30 の難しい入力（行の継続、CRLF、入れ子の `$( )`、`$( )` の中のコメント、`$'...'`、`${-}`）で、`--dump` の出力が Python 版と一致する。
    * エラーの位置も同じで、違うのは接頭辞（`consh:` と `consh_parse:`）だけである。
