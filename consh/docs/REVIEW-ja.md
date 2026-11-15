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

### 対応

テストは、ConMa の step33（`__VProc_kill__`、`__OS_dup__`、5、6）と、ConSh の step14（1〜4、7〜10）に加えた。直す前のコードで同じテストを実行して、指摘のとおりに失敗することを確かめたものもある（8 は fd が 3 → 6、ゾンビが 3。10 の Ctrl-C は 30 秒待つ）。

* 1：ご指示どおり、評価を終えるまで延ばすことにした。`__VProc_kill__` は、VProc が `__LazyList_claim__` で claim した LazyList を評価している途中なら、すべての評価を終えた（publish した、または継続で抜けた）ときに止める（`kill_pending`、`evaluating_lazy`）。`__LazyList_hold__` の LazyList では延ばさない。評価の中で待っていれば（標準入力の行、チャネルの書き手）、その待ちが終わるまで止まらないので、PLAN-ja.md 2.3 に書いた。
* 2：ご指示どおり、fd を dup することにした（ConMa の `__OS_dup__`）。`Child_target` が、stream と file のターゲットの Stream を複製して子に渡し、子は終わるときに閉じる（`Sh_release` の `Target_close`）。file のターゲットは、書く関数ではなく Stream と文脈を持ち、書くときに Stream を取り出すようにした（複製に書けるように）。そのため、読まれずに止まった `$( )` も複製を持ち続け、VProc の終わりまでファイルが開いたままになる（2.3 に書いた。step13 の case1 の変数リストの例は、`$( )` を使わないように変えた）。
* 3 と 8：`Sh_release` で `"owned"` を閉じる。`External_run` が出力を読むパイプを `Open_track` する。`VProc_stop` で SIGTERM を送った外部コマンドを、別の VProc で `__OS_wait__` する（`Proc_reap`）。
* 4：`Pipe_is_external` で、裸文字が `*?[` を含む語を除いた。`External_run` は、`owned` を閉じた後、文脈から外す。
* 5：`__OS_kill__` は、`math.isfinite` を先に調べる。`__Sexp_read__` は、明示的なスタックで読む（5000 段を確かめた）。
* 6：`__OS_trap_interrupt__` を呼んでいて、Ref を待つ VProc しかなければ、Deadlock とせず `select()` で眠る。
* 7：引用符の中の文字は、`*?[\` に加えて `]-!` もエスケープする（ConMa の `Glob_escape`）。consh_parse は、Python の `glob.glob` では表せないので、ConMa 版と同じ手順の照合（`glob_expand`、`glob_match`）に書き換えた。表の3つのパターンが Bash と同じになることを確かめた。
* 9：`Job_wait` は、`"members"` のすべての VProc を待つ。
* 10：
    * 終わったジョブ（すべての要素の文脈の `"finished"` が真）を `kill` しても、"killed" にしない。
    * 行の読み込みは、新しく読んだ塊の中だけで改行を探し、行の途中は断片の LList に持って、改行が来たら半分ずつつなぐ。512 KB の1行は約 0.4 秒になった。
    * 対話モード：Ctrl-C で止めたコマンドの `"procRef"` の外部コマンドに SIGTERM を送る。コマンドが終わるたびに `"recover"` を消す。`break`、`continue`、`return` の継続は、表をその文の mark まで閉じる（`Open_release`）。rlwrap の印は `CONSH_RLWRAPPED` にし、ConSh が起動時に環境変数から除く。
    * `__Sexp_read__` と `Sexp_read` は、最後が `\` の文字列を、入力の終わりの列で `unterminated string` にする。
    * `*/.` と `*/..`：`Glob_add_literal` は、`.` と `..` を、ディレクトリがあればあるとする。
    * チャネルの読み手は、開いた鎖がなくなったら、もう一度 `Chan_register` してから終える。
    * 別々の鎖に書かれた行の順序は、85cea09 の前からのものなので、2.3 の制限に書いた。
    * ARG_MAX は環境変数を数えないまま（2.1 に書いてある）。起動が E2BIG（と権限がない）で失敗したときの `$?` を 126 にした。

## 2026-11-06: 実行文脈の設計、性能、起動経路、Linux での再現

対象は f9625cd（段階 20 の後）である。ConMa 本体の指摘は、リポジトリ直下の REVIEW-ja.md にある。

確かめた環境は Linux（Ubuntu 24.04）、Python 3.12.3、rustc 1.75（apt）で、作者の環境（macOS、Apple M4）とは違う。VM は `CONMA_VM=rust`（vm/）を使った。`consh/test/run_all.sh` は 116 秒で終わった。最初の実行で差分が出たのは step10、step13、step14 で、すべて「5. テストが macOS の道具に頼っている」の原因だった。`lsof` を入れた後は、step10 の `od` の行末の空白の差だけが残る。

### 1. 実行文脈 `sh` が、文字列のキーを約 26 個持つ Dict である（中）

consh_eval.se の先頭の説明のとおり、`sh` は Dict で、`"global"`、`"vars"`、`"loops"`、`"outerLoops"`、`"bgCaptures"`、`"stmtCode"`、`"fieldsCode"` などのキーを持つ。26 個を数えた。

* キーの綴りの誤りは、実行時に読むまで分からない。キーが書かれた場所が 1 か所にまとまっていないので、読んだときに初めて空だと分かる。
* キーによって、持つ文脈が決まっている（`"outerLoops"` は `For_run` の文脈だけ、`"noReturn"` は関数を呼ぶと null に戻る、など）。この決まりは、先頭のコメントだけが保っている。
* ConSh の Rust 版（consh/rust）では、同じ内容を構造体にできるので、この問題は起きない。ConMa 版は 7,670 行（consh/include）、Rust 版は 5,639 行（consh/rust/src）で、同じ仕様を実現している。差のかなりの部分は、ConMa に構造体がないことによると思われる（行数の内訳は調べていない）。
* 修正案：
    * キーを定数の関数にして、綴りを 1 か所にする。読み書きは、キーごとの関数（`Sh_get_loops` など）を通す。アクセサがすでにあるかどうかは、確かめていない。
    * 根本は ConMa 側の課題で、リゾルバがフィールド名をスロット番号にするレコードを、REVIEW-ja.md（ConMa）の 6 に書いた。

### 2. 起動に 0.17 秒かかり、ループは Bash の約 70 倍遅い（低〜中）

| 項目 | ConSh | Bash |
|---|---|---|
| `examples/logsummary.sh`（起動と 1 回の実行） | 0.17 秒 | 0.013 秒 |
| `for i in $(seq 1 20000); do n=$i; done` | 1.87 秒 | 0.026 秒 |

vm/README.md には、同じループが 0.61 秒、`echo hi` の起動が 0.06 秒とある（Apple M4）。この環境が 3 倍ほど遅いので、数字は矛盾していない。比は同じ環境で測った。プロファイルは取っていない。

対話的な使い方やスクリプトの実行には足りる。ループの多い処理や、短いスクリプトを大量に起動する使い方には向かない。性能の根本は ConMa 側にある（ConMa の 7）。

### 3. 通常の起動経路に Python が残っている（中）

* `bin/consh` は、`python3 consh_parse --pos` で構文木を作る（73 行目）。`--conma-parser` を付けると Python の構文解析は外れる。
* ConSh のモジュールを作り直すときは、`bin/includer` と `bin/resolver`（どちらも Python）を使う。ConMa 版のフロントエンドでも同じモジュールができるが、3.7 秒かかる（Python 版は 1.2 秒。vm/README.md）。
* `CONSH_IMPL=rust` は、構文解析も Rust 版で行うので、Python を使わない。
* このため、ConSh を 1 つの実行形式として配るには、Rust 版を使うか、ビルド済みのモジュールと `--conma-parser` を既定にする必要がある。どちらを正式な配布形態とするかが、決まっていないように見える。

### 4. Rust 版のビルドに必要な Rust のバージョンが書かれていない（中）

* `consh/rust` は `std::io::pipe` と `PipeReader`、`PipeWriter` を使う。これらは Rust 1.87 以降の API である。rustc 1.75 では、`eval.rs` と `os.rs` で 7 件のエラーになり、ビルドできなかった。
* `Cargo.lock` が version 4 で、Cargo 1.78 以降が要る。rustc 1.75 では、`vm/` も `consh/rust/` も、ロックファイルの読み込みで失敗する。`vm/` は、`Cargo.lock` を消すとビルドできた（依存クレートがないので、消してよい）。
* どちらの `Cargo.toml` にも `rust-version` がなく、エラーを見て初めて分かる。
* 修正案：`consh/rust/Cargo.toml` に `rust-version = "1.87"`、`vm/Cargo.toml` に必要な最小のバージョンを書く。README にも書く。

この環境でビルドできなかったので、Rust 版 ConSh の出力を ConMa 版と比べることはしていない。

### 5. テストが macOS の道具に頼っている（低）

* step13 の case1、step14 の case6 が、`lsof` を使う。入っていないと `consh: case1.sh:3:30: lsof: No such file or directory: lsof` が出て、差分になる。`lsof` を入れると、step13 と step14 は通る。
* step10 の case2、case3 が、`od -c | sed 's/  */ /g'` の出力を比べる。BSD の `od` は行末に空白を付け、GNU の `od` は付けないので、期待ログ（`0000020 i n e \n e n d \n ` など）が GNU の出力と 1 文字違う。
* 修正案：
    * `od` の出力に `sed 's/ *$//'` を足し、期待ログの行末の空白を消す。どちらの `od` でも通る。
    * `lsof` がないときは、`common.sh` で、足りない道具の名前を出して止める。

### 問題なしと確かめたところ

* `consh/test/run_all.sh` を、Rust 版の VM で実行した。上のとおり、環境の違いによる差分を除いて、すべて通る。
* `examples/` の 4 つ（lazy、logsummary、spaces、typo）は、テストが通り、実行しても期待どおりの出力になる。
* ConSh の継続（`return`、`break`、`continue`、`reset` と `shift`）は、ConMa の継続の機能で実現されている。設計の意図どおりに使われている。

### 未確認

* Rust 版 ConSh（consh-rs）の出力と、ConMa 版の出力の比較（4 のとおり、ビルドできなかった）。
* 対話モード、メモリ使用量、長時間の実行。
* Python 版の VM での `run_all.sh` の実行。

### 対応

指摘のあと、1〜5 に対応した（1 は、変えないことにした）。

* 4：`consh/rust/Cargo.toml` に `rust-version = "1.87"`、`vm/Cargo.toml` に `rust-version = "1.75"` を書いた。rustup で 1.75、1.86、1.87 を入れて確かめた。`vm/` は 1.75 でビルドでき、`consh/rust/` は 1.86 では `std::io::pipe` が安定していないのでビルドできず、1.87 でビルドできる。両方の `Cargo.lock` を version 3 にした（Cargo 1.78 より前でも読める。依存クレートがないので中身は同じで、新しい Cargo でビルドしても version 3 のまま）。vm/README.md と `bin/consh` の説明にも、必要なバージョンを書いた。
* 5：step10 の case2 と case3 は、`od -c` の出力の行末の空白を `sed 's/ *$//'` で消し、期待ログの行末の空白も消した。BSD と GNU のどちらの `od` でも同じになるはずである（GNU の `od` では確かめていない）。`common.sh` に `require 道具 …` を足し、step13 と step14 の `test.sh` は、`lsof` がなければ `step13: lsof is needed for this test (install it)` を出して止める。ConMa 版（Python と Rust の両方の VM）と Rust 版で、step10、step13、step14 が通る。
* 2：`bin/consh` の既定を Rust 版（consh/rust、`CONSH_IMPL=rust`）にした（d004283）。ConMa 版は仕様の参照実装として残し、`CONSH_IMPL=conma` で使う。指摘の 2 つのスクリプトの時間（Apple M4、user + sys、5 回の最小）は次のとおり。Rust 版は、起動もループも Bash と同じ程度か速い（ほかのスクリプトとの比較は PLAN-ja.md の段階 19 の段階 5 の表）。ConMa 版も、その後の改良（文と単純文のコンパイル、データの組み込み、変換した VM など。PLAN-ja.md、vm/README.md）で速くなったが、ループは Bash の約 16〜29 倍遅いままである（指摘のときは約 70 倍）。

    | 項目 | Rust 版（既定） | ConMa 版（Rust 版の VM） | ConMa 版（変換した VM） | Bash |
    |---|---|---|---|---|
    | `examples/logsummary.sh`（起動と 1 回の実行） | 0.01 秒 | 0.06 秒 | 0.07 秒 | 0.01 秒 |
    | `for i in $(seq 1 20000); do n=$i; done` | 0.01 秒 | 0.58 秒 | 0.33 秒 | 0.02 秒 |

* 3：既定の Rust 版は、構文解析も Rust で行い（段階 19 の段階 4）、Python を使わない。consh-rs を作っておけば、ConSh は 1 つの実行形式として配れる（Rust 1.87 以降。4）。ConMa 版（`CONSH_IMPL=conma`）は、今も構文木を `consh_parse`（Python）で作り、モジュールを `bin/includer` と `bin/resolver`（Python）で作る。`--conma-parser` を付ければ、構文解析は ConMa 版（`consh_parser.se`）になる。作者の方針（memo.txt の 2026-11-06）では、VM は Python 版を基準、Rust 版を製品とし、フロントエンドは製品では ConMa 版を使い、Python 版を基準として残す。当面は Python なしで動かなくてもよい。ConMa 版の ConSh から Python をなくすことは、その方針の作業（ConMa のレビューの 8）として扱う。
* 1：作者の判断で、ConMa 版の `sh` のキーは当面このままにする。ConMa は、ほかの言語から生成することを前提とする中間言語なので、キーやフィールドの名前は、生成ツールの中の 1 か所（構造体の宣言など）から出てくる。名前の誤りは、手で書く人ではなく生成ツールの側で防ぐもので、生成ツールのテストで確かめる（レコードは、言語に足さずに ConMa から Rust への変換器の側で作った。REVIEW-ja.md の 6）。ConMa 版 ConSh は、仕様の参照実装として手で書いた例外で、製品の Rust 版は `Ctx` を構造体にしているので、名前は Rust のコンパイラが調べる。参照実装の誤りは、consh/test と、両方の版の出力の比較で見つける（段階 19 の段階 5 のエラーの照合でも、そうして違いを見つけた）。Bash も、実行の状態を C の大域変数と構造体に持ち、名前はコンパイラが調べる。文字列のキーの表を使うのは、利用者の変数名をキーにする変数の表だけで、ConSh の `"vars"` の Dict と同じである。
