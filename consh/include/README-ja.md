# consh/include：ConMa 版 ConSh の高速版

ConMa 版の ConSh には、2 つの版がある。

* **参照版**（`consh/reference`）：仕様の参照実装。読みやすさを優先して書く。
* **高速版**（このフォルダ）：参照版のうち、速くするために書き直したファイルだけを置く。

高速版は、このフォルダ、`consh/reference` の順に include ディレクトリを指定して作る（`--include consh/include --include consh/reference`）。
include は、include ディレクトリを指定の順に探すので（docs/INCLUDER.md）、このフォルダにあるファイルは、どのファイルから include されても、`consh/reference` の同じ名前のファイルの代わりになる。
ここにないファイルは `consh/reference` のものを使う。最初のファイル（`consh_main.se`）も、ここになければ `consh/reference` のものを使う。

`bin/consh` は、`CONSH_IMPL=conma` で高速版、`CONSH_IMPL=reference` で参照版を動かす（既定は Rust 版）。

* 機能を足したり変えたりしたときは、参照版を直し、ここにあるファイルにも同じ変更をして、両方でテストする
  （`CONSH_IMPL=conma` と `CONSH_IMPL=reference` で `consh/test/run_all.sh`）。
* 速くしただけ（動きは変えない）なら、高速版のテストだけでよい。

ここにファイルを書き写すときは、参照版のどの部分をどう速くしたかを、そのファイルの先頭に書く。

## 今の高速版

`consh_eval.se`、`consh_expand.se`、`consh_chan.se`、`consh_repl.se` を、次のように機械的に書き換えた（各ファイルの先頭にも書いてある）。動きは変えていない（consh/test が高速版と参照版の両方で通り、下のスクリプトの出力も同じ）。

* `Sh_get sh "key"` を `__Dict_lookup_or__ sh "key" __print_exit_error__` に、`num s` を `__Number_fromString__ __print_exit_error__ s` にした（関数の呼び出しを 1 つ減らす）。
* 条件が組み込み 1 つの関数（`__if__ (,() __is_null__ x) ...`）を、先に計算して渡す形（`__is_null__ x ,(test__) __if__ test__ ...`）にした（条件の関数を作って呼ぶ分を減らす）。
* `Func_lookup` を `__Cont_get__` を使わない形にし、`Sh_defined` で普通の変数を先に調べるようにした。

速さ（Apple M4、起動を含む、user + sys、3 回の最小。ステップ数は Rust 版の VM の `CONMA_PROFILE`）：

| スクリプト | ステップ数（参照版 → 高速版） | Rust 版の VM | 変換した VM |
|---|---|---|---|
| `for i in $(seq 1 10000); do n=$i; done` | 3.64M → 3.52M（-3%） | 0.161 → 0.158 秒 | 0.10 → 0.10 秒 |
| `xs=$(seq 1 100)` の入れ子の for 文で `n=1` を 1 万回 | 4.54M → 4.35M（-4%） | 0.168 → 0.163 秒 | 0.078 → 0.073 秒 |
| `if defined i; then n=1; else n=2; fi` を 5000 回 | 6.47M → 4.80M（-26%） | 0.257 → 0.203 秒 | 0.127 → 0.097 秒 |
| 関数 `g $i`（`n=$x`）を 5000 回 | 5.17M → 4.83M（-7%） | 0.227 → 0.217 秒 | 0.112 → 0.107 秒 |
| 関数 `f $i`（`echo "x$a"`）を 3000 回 | 5.34M → 4.98M（-7%） | 0.237 → 0.227 秒 | 0.111 → 0.108 秒 |
| `echo $i` を 1 万回（`/dev/null` へ） | 7.88M → 7.34M（-7%） | 0.334 → 0.317 秒 | 0.171 → 0.168 秒 |
| `add $i x > (r)` を 3000 回 | 6.49M → 6.07M（-6%） | 0.287 → 0.279 秒 | 0.124 → 0.118 秒 |
| `y=$(f $i)` を 1000 回 | 3.75M → 3.47M（-7%） | 0.216 → 0.205 秒 | 0.117 → 0.112 秒 |

残りの時間は、どの関数にも広く分かれている。`for` 文で `$( )` の行を 1 つ読むごとに 100 ステップ以上かかるのは、OS から読んだ行の LazyList（util.se）を、改行を除いた LazyList（`Subst_lines`）でさらに包んでいるためで、遅延して読む作りを変えないと減らせない。
