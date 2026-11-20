# consh/include：ConMa 版 ConSh（最適化版）

ConMa 版の ConSh には、2 つの版がある。

* **最適化版**（このフォルダ）：ConMa 版のすべてのファイル。ただし、`(( ))` が使う SJS の変換器とランタイム（`sjs_*.se`）は、SJS のものとして sjs/include にある（2026-11-19 にこのフォルダから移した。sjs/README-ja.md）。ふだん使い（`CONSH_IMPL=conma`）、機能もここに足す。
* **参照版**：仕様の参照実装。速くするために書き直したファイルについては、書き直す前のもの（読みやすさを優先したもの）を `consh/reference` に残す（`consh/reference/README-ja.md`）。

2 つの版は、2026-11-15 までは「高速版」（「速い版」とも）と参照版と呼んでいた。参照実装（reference implementation）と、それを最適化した実装（optimized implementation）という、よく使われる言い方に合わせて「最適化版」にした。

最適化版は、このフォルダ、`sjs/include`、標準ライブラリの `include/ext` と `include` の順に include ディレクトリを指定して作る。参照版は、`consh/reference`、`sjs/include/core`（コア ConMa 用。今は `sjs_bits.se` だけ）、このフォルダ、`sjs/include` の順に指定して作る（`--include consh/reference --include sjs/include/core --include consh/include --include sjs/include`。標準ライブラリの `include/core`、`reference`、`include` が続く）。
include は、include ディレクトリを指定の順に探すので（docs/INCLUDER.md）、`consh/reference` にあるファイルは、どのファイルから include されても、このフォルダの同じ名前のファイルの代わりになる。
標準ライブラリも同じ形で、参照版は、リポジトリのルートの `reference`（書き直す前の `util.se` など）を `include` より先に探す。

`bin/consh` は、`CONSH_IMPL=conma` で最適化版、`CONSH_IMPL=reference` で参照版を動かす（既定は Rust 版）。

* 機能を足したり変えたりしたときは、このフォルダのファイルを直し、`consh/reference` に同じ名前のファイルがあれば、それにも同じ変更をして、両方でテストする（`CONSH_IMPL=conma` と `CONSH_IMPL=reference` で `consh/test/run_all.sh`）。
* 速くしただけ（動きは変えない）なら、最適化版のテストだけでよい。そのファイルが `consh/reference` になければ、書き直す前のものをそこに写しておく。

速くするために書き直したファイルには、どの部分をどう速くしたかを、その先頭に書く。

## 今の最適化版

`consh_eval.se`、`consh_expand.se`、`consh_chan.se`、`consh_repl.se` を、次のように書き換えた（書き直す前のものは `consh/reference` にある）（各ファイルの先頭にも書いてある）。また、SJS のビット演算の `sjs_bits.se`（sjs/include）は、拡張 ConMa のプリミティブ（`"bitwise32"`。docs/dev/BITWISE-PLAN-ja.md）を使う（2026-11-12。最適化版のモジュールには、フロントエンドが `(Uses "bitwise32")` を付け、Python VM は bin/to-core で直して動かす）。コア ConMa で作る参照版では、`sjs/include/core/sjs_bits.se` が、標準ライブラリの `include/core/bitwise32.se`（7 つをコア ConMa の関数として定義する）を読んでから `sjs_bits.se` を読む（2026-11-15。それまでは `consh/reference/sjs_bits.se` が、同じものを SJS の側で算術で求めていた）。`sjs/include/core` は、言語の分け方（コア ConMa 用の版）で、`consh/reference`（最適化する前の版）とは別の軸である。チャネル（`consh_chan.se`）も、2026-11-16 から、最適化版は拡張 ConMa の機能 `"chan"` のプリミティブ（`__Chan_new__` など。docs/dev/CHAN-PLAN-ja.md）を呼び、参照版（`consh/reference/consh_chan.se`）は、標準ライブラリの `include/core/chan.se`（同じ名前のコア ConMa の関数）を読んでから最適化版の `consh_chan.se` を読む（最適化版のモジュールには `(Uses "chan")` が付き、Python VM は bin/to-core で直して動かす）。動きは変えていない（consh/test が最適化版と参照版の両方で通り、下のスクリプトの出力も同じ）。

* `Sh_get sh "key"` を `__Dict_lookup_or__ sh "key" __print_exit_error__` に、`num s` を `__Number_fromString__ __print_exit_error__ s` にした（関数の呼び出しを 1 つ減らす）。
* 条件が組み込み 1 つの関数（`__if__ (,() __is_null__ x) ...`）を、先に計算して渡す形（`__is_null__ x ,(test__) __if__ test__ ...`）にした（条件の関数を作って呼ぶ分を減らす）。
* `Func_lookup` を `__Cont_get__` を使わない形にし、`Sh_defined` で普通の変数を先に調べるようにした。
* 外部コマンド 1 つだけの `$( )`（`$(seq 1 10000)` など）の値を、1 段の LazyList で作るようにした（`Subst_lines_from`）。参照版は、util.se の `__LazyList_lines__`（改行を含む行の LazyList）を `Subst_lines` でもう一度包み、`__LazyList_lines_scan__` は行を取り出すたびに読んだ塊の残り全体を写す。最適化版は、読んだ塊を 1024 文字ずつの断片に切り、断片の中を位置で進めて行を取り出す（`__String_find__` と `__String_substring__` は、呼ぶたびに文字列全体の文字数を数えるので、長い塊の中を位置で進めると、塊の長さの 2 乗の時間がかかる）。
* LazyList の値を持つ変数 1 つだけの語（`for b in $xs`、`echo $xs` の `$xs`）を、葉ごとに、glob の記号を含むかだけを調べて語にするようにした（`Single_list_fields`）。参照版は、葉ごとに「語の文字列、glob のパターン、記号を含むか」の組（候補）を作り、前の部分の候補とつなぎ、分けてから語にする（`Expand_fields_from`）。変数 1 つだけなら前後につなぐ文字列はないので、記号を含む葉にだけ glob を適用すれば同じ結果になる。
* Stream から作る標準入力（シェルの標準入力、`< ファイル`、外部コマンドからのパイプ。`Input_stream`）と、外部コマンドの出力を行ごとにターゲット（変数リスト、チャネルなど）に書く `Target_pump` も、同じ 1 段の LazyList で読むようにした（`seq 1 20000 > (first . rest)` は 5.20M → 4.61M ステップ）。なお、変数リストへの大きな出力は、Rust 版の VM の `__MSeq_uncons__` が残りの要素を毎回写していたので、両方の版で行の数の 2 乗の時間がかかっていた（4 万行で 4.8 秒）。VM の側で、残りが元の MSeq の要素を共有するようにした（vm/README.md の「MSeqs」。4 万行は 0.43 秒、10 万行は 1.07 秒）。参照版は、`__LazyList_lines__` を `Lines_chomped` でもう一度包む。Stream は、それを開いた側が閉じるので、ここでは閉じない。
* 変数リストに書いた要素の MSeq を LList に直す `List_fromMSeq` と、それを LazyList の前に付ける `LazyList_prepend` を、要素の数だけ深く再帰しない形（前から裏返しに集めて裏返す）にした。参照版は、要素の数だけ深く再帰し、その間 CChain が要素の数だけ伸びるので、`seq 1 400000 > (first . rest)` は最大 865 MB 使う（最適化版は 291 MB）。
* `Subst_settle_os` で、まだ終わっていない `$( )` の値の先頭に `Pipe_cap` 個そろったこと（書き手がパイプに書けずに待っていること）を、記録（`"full"`）に覚えるようにした。参照版は、コマンドが終わるたびに値の先頭から `Pipe_cap` 個たどり直すので、出力がパイプに収まらない `$( )`（`$(seq 1 100000)`）を `for` 文で読むと、1 回ごとに約 760 ステップ余分にかかっていた。値の先頭は変わらず、評価した要素は残るので、一度そろえば、そのあとも同じ答えになる。
* `Subst_settle` で、待った後に残った `$( )` がどれも `"full"` なら、そのことを global の `"substsParked"` に覚え、次の文の終わりからは、状態が `"done"` になったものがないかだけを見るようにした（2026-11-16。`"full"` の記録は、終わらないかぎり残るので、結果は同じ）。`for i in $(seq 1 100000)` の中の文の終わりが、1 回 約 82 ステップから約 48 ステップになる（`02_for_seq.sh` が 33.17M → 29.72M ステップ）。
* 関数の変数リストの名前と rest の名前、本体の文を、関数を定義したとき（`Func_make`）に 1 回だけ節から取り出して関数の Dict に置き、`Func_call` は引数の数を、渡しながら調べるようにした（`Func_bind_args`。合わないと分かった所で、参照版と同じ `Arity_check` のエラーにする）。関数の呼び出し 1 回が約 75 ステップ減る（`13_func_assign.sh` が、上と合わせて 199.54M → 181.78M ステップ。docs/consh/dev/COMPILE-PLAN-ja.md の 5）。
* LazyList の値を語の葉にする `Value_leaves_lazy` に、値ができている部分の近道を付けた（2026-11-17、`Leaves_ready`）。先頭から、評価済みで String の要素が続く間は、LazyList の Ref を `__Ref_get__` で読むだけでたどり（評価も、待つことも、継続で戻ることもしない。vm/tools/cont_needs.py で E）、その文字数を引数の上限からまとめて引く。評価していない要素か String でない要素に来たら、その 1 つを前と同じく `__LazyList_uncons__` と `Value_leaves_in` で扱い、その先でまた近道を試す。関数の引数の `$rest` のように、要素がもうできているリストを展開するときに効く。再帰する `consh/bench/15_recursion.sh`（深さ 500 を 5 回）で、ステップが 96.0M → 74.8M（−22%）、時間は Apple M1 で Rust 版の VM が 3.247 → 3.030 秒（−7%）、wasm 版の VM が 4.101 → 3.853 秒（−6%）。`11`、`16` は変わらない（1% 以内）。はじめ、Ref を読むところを include/util.se の関数（`__LazyList_ready__`）にしたら、ステップは 8% 減ったのに時間は 3% 増えた：`CONMA_PROFILE` では VM の速い道（docs/dev/FAST-PLAN-ja.md の F2〜F5。評価済みの LazyList を `__LazyList_force__` に渡すと 1 回で済む F3 など）が働かないので、ステップの減り方は、ふだんの時間の減り方より大きく出る。変換した VM では測っていない。

速さ：`consh/bench` のスクリプト（何を測るかは `consh/bench/README-ja.md`）を、`consh/bench/run.py -v conma:rust,reference:rust,conma:transpiled,reference:transpiled` で測ったもの（Apple M4、2026-11-09、main の cba4430。user + sys、3 回の最小、起動を含む。どの組み合わせも出力は同じ）。変換した VM は `CONMA_BENCH_REFERENCE=1 bench/build.sh` で、両方の版の変換したコードを組み込んで作った。ステップ数は Rust 版の VM の `CONMA_PROFILE`：

| スクリプト | ステップ数（参照版 → 最適化版） | Rust 版の VM（参照版 → 最適化版） | 変換した VM（参照版 → 最適化版） |
|---|---|---|---|
| `00_start.sh` | 5,472 → 5,495（+0%） | 0.018 → 0.017 秒 | 0.018 → 0.019 秒 |
| `01_for_seq.sh` | 3.67M → 2.70M（-26%） | 0.142 → 0.113 秒 | 0.074 → 0.058 秒 |
| `02_for_seq_100000.sh` | 112.59M → 33.17M（-71%） | 4.000 → 1.341 秒 | 1.638 → 0.447 秒 |
| `03_nested_for.sh` | 1.37M → 1.27M（-7%） | 0.065 → 0.063 秒 | 0.035 → 0.035 秒 |
| `04_if_defined.sh` | 6.49M → 4.40M（-32%） | 0.245 → 0.183 秒 | 0.110 → 0.071 秒 |
| `05_func_assign.sh` | 5.18M → 4.41M（-15%） | 0.216 → 0.192 秒 | 0.082 → 0.073 秒 |
| `06_func_echo.sh` | 5.35M → 4.73M（-11%） | 0.229 → 0.211 秒 | 0.093 → 0.089 秒 |
| `07_echo.sh` | 7.89M → 6.50M（-18%） | 0.313 → 0.273 秒 | 0.142 → 0.125 秒 |
| `08_func_varlist.sh` | 7.16M → 6.35M（-11%） | 0.313 → 0.289 秒 | 0.123 → 0.117 秒 |
| `09_subst_func.sh` | 3.76M → 3.40M（-10%） | 0.205 → 0.192 秒 | 0.094 → 0.089 秒 |
| `10_pipe_for_stdin.sh` | 77.60M → 67.77M（-13%） | 3.461 → 3.032 秒 | 1.447 → 1.206 秒 |
| `11_seq_varlist.sh` | 25.86M → 25.43M（-2%） | 1.016 → 0.941 秒 | 0.868 → 0.390 秒 |

表の名前は測ったときのもの：`01_for_seq`（2026-11-17 に削除。大きさだけが違う 02_for_seq にまとめた）、`02_for_seq_100000`（2026-11-17 に 02_for_seq へ名前を変更）、`05_func_assign`（2026-11-17 に削除。大きさだけが違う 13_func_assign にまとめた）、`08_func_varlist`（2026-11-17 に削除。大きさだけが違う 16_func_varlist にまとめた）、`09_subst_func`（2026-11-17 に削除。大きさだけが違う 17_subst_func にまとめた）。

`03_nested_for.sh` は `for i in ${xs}` と書くので、`${xs}` の LazyList をそのままたどり、`Single_list_fields`（`$xs` を単語に分ける近道）を通らない。`for i in $xs` と書くと、参照版 4.54M → 最適化版 2.30M ステップ、Rust 版の VM で 0.167 → 0.098 秒になる（2026-11-08 に測ったもの）。

`10_pipe_for_stdin.sh` の 1 行あたりの時間の多くは、関数どうしをつなぐチャネル（書き手が空きを待つ `Chan_wait_room` が 1 行に 2 回、コマンドごとの `$( )` の確かめが約 110 ステップなど）にかかる。書き手は、読み手が 1 つ取り出すたびに起こされて 1 つ書く。まとめて起こすようにすると減らせるが、書き手と読み手の動く順（標準エラー出力の並びなど）が変わり、Rust 版と同じ結果にならなくなるので、変えていない。`11_seq_varlist.sh` は、両方の版でステップ数がほぼ同じで、時間の差は主に、最適化版が変数リストの要素を再帰せずに LList にする（`List_fromMSeq`）ことによる、使うメモリと循環の回収の違いである。

1 回の呼び出しで 1 ステップしか減らない書き換え（よく呼ばれる関数から `__Cont_get__` を取り除くなど）は、試したが効果がほとんどなかったので入れていない。
