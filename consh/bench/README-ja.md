# ConSh の処理時間

ConSh の版（Rust 版、ConMa 版の高速版と参照版）と VM（Rust 版の VM、変換した VM）の組み合わせごとに、同じスクリプトの処理時間を測る。

## スクリプト

`NN_名前.sh` の 1 行目のコメントが、何を測るかである。`consh/include/README-ja.md` の速さの表の行を、その説明から書いたもの（表を作ったときのスクリプトそのものではない）に、起動の時間を測る `00_start.sh` を足した。出力を比べられるように、多くは最後に変数の値を出す。

| スクリプト | 測るもの |
|---|---|
| `00_start.sh` | 何もしない（起動の時間） |
| `01_for_seq.sh` | `for i in $(seq 1 10000); do n=$i; done` |
| `02_for_seq_100000.sh` | `n=0; for i in $(seq 1 100000); do n=$i; done; echo $n` |
| `03_nested_for.sh` | `xs=$(seq 1 100)` の入れ子の for 文で `n=1` を 1 万回 |
| `04_if_defined.sh` | `if defined i; then n=1; else n=2; fi` を 5000 回 |
| `05_func_assign.sh` | 関数 `g $i`（`n=$x`）を 5000 回 |
| `06_func_echo.sh` | 関数 `f $i`（`echo "x$a"`）を 3000 回（`/dev/null` へ） |
| `07_echo.sh` | `echo $i` を 1 万回（`/dev/null` へ） |
| `08_func_varlist.sh` | `add $i x > (r)` を 3000 回 |
| `09_subst_func.sh` | `y=$(f $i)` を 1000 回 |
| `10_pipe_for_stdin.sh` | `seq 1 100000 \| cnt`（`cnt` は `for x in $(stdin)` で数える関数） |
| `11_seq_varlist.sh` | `seq 1 100000 > (first . rest)` |

## 測る

```
consh/bench/run.py                         # Rust 版と、高速版＋変換した VM
consh/bench/run.py -v rust,conma:rust,reference:rust
consh/bench/run.py -n 5 02_for_seq_100000.sh 11_seq_varlist.sh
```

* `-v` で、版と VM の組み合わせをコンマで区切って選ぶ：`rust`（Rust 版、`consh/rust/target/release/consh-rs`）、`conma:rust` と `reference:rust`（高速版と参照版を Rust 版の VM で）、`conma:transpiled` と `reference:transpiled`（変換した VM で）。既定は `rust,conma:transpiled`。
* 各スクリプトを、組み合わせごとに `-n` 回（既定は 3）実行し、プロセスの user + sys の最小を、起動を含めて、Markdown の表に出す。2 つ目からの列には、最初の列に対する倍率を付ける。組み合わせの出力（終了コード、標準出力、標準エラー出力）が違えば、その行に `outputs differ` と付け、終了コードを 1 にする。
* 実行ファイルは、`bin/consh` と同じ引数で、`bin/consh` を通さずに起動する（`consh-rs run スクリプト`、`conma --main モジュール run-src スクリプト スクリプト`）。その前に、`bin/consh` でモジュール（`/tmp/consh.module.se` と `/tmp/consh.reference.module.se`、`CONSH_MODULE_DIR`）を作る。`consh-rs` がソースより古ければ、そのとき `bin/consh` が警告する。
* 変換した VM は、モジュールとまったく同じ中身のモジュールの変換したコードしか使わない。モジュールを作り直したら、`bench/build.sh`（参照版も測るなら `CONMA_BENCH_REFERENCE=1 bench/build.sh`）で変換した VM を作り直す。`run.py` は、変換したコードが使われるかを先に確かめ（`CONMA_TRANSPILED_REPORT`）、使われなければ、そう言って止まる。

## 結果

Apple M2、2026-11-09（main の 98b885d）。user + sys、3 回の最小、起動を含む。括弧の中は Rust 版に対する倍率。変換した VM は `CONMA_BENCH_REFERENCE=1 bench/build.sh` で作った。

| スクリプト | Rust 版 | 高速版＋変換した VM | 参照版＋変換した VM | 高速版＋Rust 版の VM | 参照版＋Rust 版の VM |
|---|---|---|---|---|---|
| `00_start.sh` | 0.002 秒 | 0.025 秒（12.9×） | 0.025 秒（12.9×） | 0.025 秒（12.6×） | 0.024 秒（12.3×） |
| `01_for_seq.sh` | 0.011 秒 | 0.080 秒（7.3×） | 0.102 秒（9.3×） | 0.157 秒（14.3×） | 0.194 秒（17.7×） |
| `02_for_seq_100000.sh` | 0.091 秒 | 0.604 秒（6.7×） | 2.235 秒（24.6×） | 1.818 秒（20.0×） | 5.434 秒（59.8×） |
| `03_nested_for.sh` | 0.006 秒 | 0.047 秒（8.6×） | 0.048 秒（8.7×） | 0.087 秒（15.7×） | 0.090 秒（16.3×） |
| `04_if_defined.sh` | 0.012 秒 | 0.096 秒（8.1×） | 0.150 秒（12.7×） | 0.246 秒（20.9×） | 0.329 秒（28.0×） |
| `05_func_assign.sh` | 0.012 秒 | 0.100 秒（8.5×） | 0.112 秒（9.6×） | 0.260 秒（22.2×） | 0.290 秒（24.7×） |
| `06_func_echo.sh` | 0.014 秒 | 0.119 秒（8.6×） | 0.124 秒（9.0×） | 0.281 秒（20.3×） | 0.304 秒（21.9×） |
| `07_echo.sh` | 0.026 秒 | 0.175 秒（6.7×） | 0.200 秒（7.6×） | 0.375 秒（14.3×） | 0.432 秒（16.5×） |
| `08_func_varlist.sh` | 0.013 秒 | 0.153 秒（11.4×） | 0.164 秒（12.2×） | 0.377 秒（28.0×） | 0.406 秒（30.2×） |
| `09_subst_func.sh` | 0.041 秒 | 0.115 秒（2.8×） | 0.121 秒（2.9×） | 0.254 秒（6.2×） | 0.271 秒（6.6×） |
| `10_pipe_for_stdin.sh` | 0.553 秒 | 1.621 秒（2.9×） | 1.913 秒（3.5×） | 4.084 秒（7.4×） | 4.647 秒（8.4×） |
| `11_seq_varlist.sh` | 0.036 秒 | 0.519 秒（14.5×） | 1.104 秒（30.9×） | 1.275 秒（35.7×） | 1.383 秒（38.7×） |

* ConMa 版は、起動だけで約 0.025 秒かかる（約 2.3 MB のモジュールを読む）。Rust 版は 0.002 秒。
* シェルの中だけで動くもの（`01`〜`08`）は、高速版＋変換した VM が Rust 版の約 7〜11 倍、高速版＋Rust 版の VM が約 14〜28 倍。外部コマンドや `$( )` を多く使うもの（`09`、`10`）は、Rust 版もプロセスとパイプに時間がかかるので、差が約 3 倍に縮まる。
* `11_seq_varlist.sh` は、同じ日の 98b885d の前（3927ba4）には、高速版＋変換した VM で 1.467 秒（Rust 版の 40 倍）だった。
