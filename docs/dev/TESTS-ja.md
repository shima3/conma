# テストの流し方

この文書は、テストを全部流す手順、前からある既知の違い、次に全部流す人への申し送りをまとめる。テストを流した記録は [docs/dev/TESTS-RECORD-ja.md](TESTS-RECORD-ja.md) にある（2026-11-22 に、ここから移した）。コミットメッセージには、その変更を何で確かめたかを短く書く。「全部流す」のすべてを流したときは、誰が流しても、その結果を記録（docs/dev/TESTS-RECORD-ja.md）に 1 行足して、ふつうのコミットにする（結果を書くためだけの空のコミットは作らない）。一部だけを流したとき（変更に関わるテストだけ、ある VM を除いたもの）は、コミットメッセージに書くだけにし、ここには記録しない。

## 流し方

どのテストも、違いがなければ、各テストの `== 名前` の行だけを出す。`-j N` は N 個ずつ並べて流す。

### 先に作るもの

| 作るもの | コマンド | 要るテスト |
|---|---|---|
| Rust VM | `cd vm && cargo build --release` | `CONMA_VM=rust`、ConSh のモジュール（`conma build`） |
| wasm 版の VM と web のファイル | `consh/js/build.sh`（VM だけなら `vm/wasm/build.sh`） | `CONMA_VM=wasm`、consh/js/test |
| 変換版の VM | `bench/build.sh` | `CONMA_VM=transpiled` |
| ConSh の Rust 版（なくてもよい） | `cd consh/rust && cargo build --release` | `CONSH_IMPL=rust` |

* ブランチを切り替えたり pull したりすると、ソースの時刻が変わる。VM がソースより古いと `consh/bin/consh` が警告を出すので、作り直してから流す。
* wasm 版は、`vm/src` か `vm/wasm` を変えたら `consh/js/build.sh` で作り直す。古いまま流すと、VM の出すメッセージが古く、ログと違う（2026-11-15 に、名前を変える前の wasm 版で test/fast1 が違った）。
* 変換版の VM は、前に作ったときから、モジュールと VM に関わるファイルが変わっていなければ、作り直さずに使ってよい（2026-11-18。作り直しに、Apple M1 で約 18 分かかる。m1/transpiled-build で、変換したコードを Rust のモジュールに分けて LLVM が並べて最適化できるようにするまでは、M4 で約 28 分、M1 で約 40 分かかっていた）。前に作ったときのコミットを覚えておき（記録の行に「変換版の VM は 1a2b3c4 で作ったもの」と書く）、次が何も出さないことで見る：`git diff --stat 1a2b3c4 HEAD -- vm include bin consh/include bench test/transpiled`（VM と変換器、標準ライブラリ、フロントエンド、ConSh の最適化版、bench/ と test/transpiled のモジュール）。そのうえで、ConSh のモジュールに変換したコードが使われることを、`CONMA_VM=transpiled CONMA_TRANSPILED_REPORT=1 consh/bin/consh` で（`transpiled code: used`）確かめる。rustc を新しくしたときと、`CONMA_BENCH_REFERENCE=1` で参照版も入れて流すときは、作り直す。テストのためだけに作り直すときは、`CONMA2RUST_COMPACT=1 bench/build.sh` で作ってよい（M1 で約 15 分。変換したコードが短く、ConSh が最大 2% ほど遅い。vm/README.md）。そうして作った VM は、記録の行に「`CONMA2RUST_COMPACT=1` で作ったもの」と書き、ベンチマークには使わない。
* 変換版の VM には、`bench/build.sh` を動かしたときにキャッシュのディレクトリ（`bin/cache-dir`）にある ConSh のモジュールが組み込まれる。ConSh のテストを変換版の VM で流すときは、先に Rust VM で consh/test を流してモジュールを作っておき、それから `bench/build.sh` を動かす。ほかの suite と並べて流すときも、`bench/build.sh` は、consh/test などで ConSh のモジュールを作り直し終えてから始める（ConSh のソースが変わった後に、前のモジュールのまま作ると、consh/test が「the transpiled code of this program was made from another version of its module」の警告を出して、変換したコードを使わない。2026-11-19 の 442e41d の記録）。作った後は、`CONSH_IMPL=conma CONMA_TRANSPILED_REPORT=1 CONMA_VM=transpiled consh/bin/consh <スクリプト>` が `transpiled code: used` を出すことを確かめる。

### 全部流す

```
test/run_all.sh -j 6                                   # Python VM（基準）
CONMA_VM=rust test/run_all.sh -j 6
CONMA_VM=wasm test/run_all.sh -j 3
CONMA_VM=transpiled test/run_all.sh -j 6
CONMA_VM=rust consh/test/run_all.sh -j 6               # ConSh の最適化版
CONMA_VM=rust CONSH_IMPL=reference consh/test/run_all.sh -j 6   # ConSh の参照版
CONMA_VM=wasm consh/test/run_all.sh -j 3
CONMA_VM=transpiled consh/test/run_all.sh -j 6
consh/js/test/run_all.sh
sjs/test/run_all.sh
conjs/test/run_all.sh                                  # ConJS（Rust 版の VM）
CONMA_VM=wasm conjs/test/run_all.sh
CONMA_VM=python conjs/test/run_all.sh
```

* 標準ライブラリの参照版で test/ を流すときは、`CONMA_REFERENCE=1` を付ける（reference/README-ja.md）。下の既知の違いがある。
* consh/test は、`CONSH_IMPL` がなければ ConMa 版の最適化版を試す（consh/test/common.sh）。`CONMA_VM` がなければ、ConSh は Rust 版の VM を使う（作っていなければエラー。Python 版の VM で流すときは `CONMA_VM=python`）。

### Windows（I7）で流す

2026-11-22 から（段階 P6c。docs/dev/PLATFORM-PLAN-ja.md の 4.11 節と 8.13 節）。Git Bash で流す。

* clone は `git clone -c core.autocrlf=false -c core.symlinks=true` で作る（CRLF の台本は bash で動かず、CRLF のログは VM の出力と合わない。シンボリックリンクには Windows の開発者モードが要る）。今ある clone なら、`git config core.autocrlf false`、`git config core.symlinks true` の後に `git rm -q --cached -r . && git reset -q --hard` で取り出し直す。
* 本当の Python 3 が要る（Python 版の VM を使わなくても、bin/includer などの道具が Python で動く）。python.org の版を入れ、その python.exe を同じディレクトリに python3.exe として写し、そのディレクトリを PATH の先頭に置く（PATH の `python3` は、ふつうは Microsoft Store を開くアプリ実行エイリアス）。
* PATH に cargo（`$HOME/.cargo/bin`）と Node.js も置く。`MSYS=winsymlinks:nativestrict` を付ける（test/step16 と step23 の `ln -s` が、本当のシンボリックリンクを作るように）。
* `CONMA_VM` を必ず付ける（bin/vm の既定は Python 版の VM で、Windows では動かない。パイプを `select` で待つため）。Python 版の VM の列は流さない。
* 台本は、`cygpath` があれば（Git Bash）、`pwd` と一時ディレクトリを VM が書く形（`C:/Users/...`）にする（test/common.sh、consh/test/common.sh）。

```
export PATH="$LOCALAPPDATA/Programs/Python/Python315:$HOME/.cargo/bin:$PATH" MSYS=winsymlinks:nativestrict
(cd vm && cargo build --release && cargo test --release --lib)
CONMA_VM=rust test/run_all.sh -j 8
CONMA_VM=rust consh/test/run_all.sh -j 6
conjs/test/run_all.sh
sjs/test/run_all.sh
```

## 既知の違い

これらは前からある違いで、変更の誤りではない。これ以外の違いが出たら、誤りを疑う。

| VM や版 | テスト | 理由 |
|---|---|---|
| wasm 版 | test/step40 | ログはネイティブの VM のもので、JavaScript のある wasm 版では case1 の呼び出しが Stream を受け取る（docs/dev/ROADMAP.md の Step 40） |
| wasm 版 | consh/test の step17 | JavaScript のない版の `js` のテストで、wasm 版には JavaScript があるので、メッセージと `$?` が違う（`JavaScript is disabled`、2） |
| wasm 版 | consh/test の step8 | VProc を切り替える時の細かい所による（docs/dev/FAST-PLAN-ja.md の 3.3） |
| どの VM でも、機械が混んでいるとき | test/step31 | SIGINT を 2 回送るテストで、混んでいると 2 回が 1 回にまとまることがある。単独で流せば通る（`-j 8` の wasm 版で見られた） |
| 標準ライブラリの参照版（`CONMA_REFERENCE=1`） | test/step18 の case12、step20 の case15、step21 の case6、fast3、fast9 | util.se そのものに依るログ：エラーの位置が reference/util.se の行になる、数える時点のステップがずれる、印を付けた Body の数や大域の番号が違う（reference/README-ja.md） |
| ConSh の Rust 版（`CONSH_IMPL=rust`） | consh/test の examples | Rust 版は `(( ))` を読めず、consh/examples/nbody.sh が `expected a command, but found '('` で止まる（nbody.sh を足した 2026-11-15 から） |
| Windows（I7）のネイティブの VM | test/ の step11、13、14、15、16、23、consh/test の examples と step16 | Unix の絶対パス（`/bin/sh`、`/bin/cat`、`/bin/echo`、`/usr/bin/env`、`/usr/bin/true`、`/usr/bin/grep` など）でプログラムを起動する。Windows にそのパスはない（2026-11-22 から。段階 P6c） |
| Windows（I7）のネイティブの VM | test/step11 の `/etc/hosts`、test/step16 の `realpath /` | `Permission denied` でなく `No such file or directory`（Windows に /etc/hosts はない）。`/` の realpath は `C:/` |
| Windows（I7） | test/step21 の case5 と case6 | memcheck.py が Python 版の VM を使う（Windows では動かない） |
| Windows（I7） | test/step31、test/step33 の case4、consh/test/step12 の case7 と case8 | Git Bash の `kill -INT` は Windows のプログラムに届かず、Python に pty がない。流さず、その旨を 1 行出す |
| Windows（I7） | test/step32 | Git Bash の `ps` に `-o` がない。`another group` でなく `same group` |
| Windows（I7） | consh/test の step2 | `words` はシバンで始まる台本で、Windows はそれを直接起動できない。MSYS の `sh` がシグナルで終わると、終了コードは 143 でなく 3840 |
| Windows（I7） | consh/test の step3、step4、step11 | 外部のコマンドの `pwd`（Git Bash）が `/c/...` の形を書き、`cd /` の後は `/c`。step11 は、その形のパスの glob が一致しない |
| Windows（I7） | consh/test の step6 | `/dev/nul?` の glob が一致しない（Windows に /dev はない） |
| Windows（I7） | consh/test の step13 と step14 | lsof がない（台本がその旨を出す） |
| Windows（I7） | （テストにはない） | MSYS（Git Bash）のプログラムは、引用符の中の `\\` を 1 つの `\` に読むので、ConMa が渡す引数の 2 つ続く `\` は 1 つで届く（Windows の C ランタイムと MSYS の両方に合う書き方がない。vm/src/sys/windows.rs の `add_args`） |

## 申し送り

次に全部流す人への申し送り。作業した人が、作業のブランチの見出し（`### m4/chan` など）の下に書いて alpha にマージし、テストの担当者は、その見出しを含む alpha を自分のブランチ（`m2/test` など。機械と `test`）で全部流して確かめたら、その見出しの行を消して記録（docs/dev/TESTS-RECORD-ja.md）の行を足す（docs/dev/CONTRIBUTING.md の「Branches and Git settings」）。見出しを書かないのは、変えたファイルがすべて文書（docs/dev/CONTRIBUTING.md の Branches の 5 の、`docs/` と `.github/` の下のファイルと、`*.md`）で、ベンチマークも頼まないブランチだけ（2026-11-21 から。設計の案など。変えたものは、コミットのメッセージと PR にある）。ほかのファイルを 1 つでも変えたブランチには、どの suite も読まないと思うファイル（`.gitignore`、見本のページなど）だけでも、見出しを書く（テストの担当者は、それも含めて全部流すので、何が変わったかを知る必要がある）。Full Bench（Full ConMa Bench、Full ConSh Bench、Full SJS Bench）も、ここで「ベンチマーク：」で始まる行で頼む。行には、3 つのどれか、測るコミット（7 桁。前後を比べるなら前のコミットも）、版（最適化版＋transpiled VM など）を書き、測る人は書かない。Full Bench は、ディレクトリのすべてのベンチマークを、書いた版だけで測る（すべての版で流す Full Test とは違う。版を書かなければ、表が比べるすべての版）。誰がいつ測るかはユーザーが決め、測る人は、その行を見ただけでは始めない。Full Bench でない測り直しは、変更した開発者が、ユーザーの指示を待たずに自分で測る（2026-11-18 から。それまでは、どの測り直しもここで頼んでいた）。その行は、測った人が、測ったブランチ（`m2/bench-1a2b3c4` など。機械と、測ったコミットの 7 桁）で消すまで残し（テストが済んでも消さない）、見出しは、その下の行がすべてなくなったときに消す（同じ文書の「Benchmarks」）。全部流している間に alpha に入ったブランチの見出しは、次に流すまで残す（記録には流したコミットだけを書く）。2026-11-16 までは、作業のブランチを作らずに alpha に書いていたので、見出しは alpha にした。
