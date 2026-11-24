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

### CI（GitHub Actions）

PR ごとと alpha への push ごとに、GitHub Actions が、速い suite を Linux と macOS で流す（.github/workflows/ci.yml、2026-11-22 から。docs/dev/CONTRIBUTING.md の Tests）。流すのは、Rust 版と wasm 版の VM の test/、Rust 版と wasm 版の VM の consh/test、consh/js/test、sjs/test、Rust 版と wasm 版の VM の conjs/test、vm の `cargo test --release`、test/host-api.sh。それぞれを .github/ci-suite.sh で流し、下の既知の違いのほかに違いがあれば失敗にする（既知の違いが出なくても失敗にはしない）。既知の違いの表を変えたら、ci.yml の、その suite の行も変える。Python 版と transpiled VM、ConSh の参照版と Rust 版、`CONMA_REFERENCE=1` は流さないので、CI が通っても Full Test の代わりにはならず、記録（docs/dev/TESTS-RECORD-ja.md）にも書かない。Windows では、下の「Windows（I7）で流す」のとおりに用意して、vm の `cargo test --release --lib` と、Rust 版の VM の test/、consh/test、conjs/test、sjs/test を流す（既知の違いは、下の表の I7 の行。段階 P6c から。docs/dev/PLATFORM-PLAN-ja.md の 4.11 節）。

### Windows（I7）で流す

2026-11-22 から（段階 P6c。docs/dev/PLATFORM-PLAN-ja.md の 4.11 節と 8.13 節）。Git Bash で流す。ここの suite を、ここに書いたどの VM でもすべて流すことを、**Windows Test** と呼ぶ（2026-11-23。docs/dev/CONTRIBUTING.md の Tests）。I7 が i7/test で流し、記録（docs/dev/TESTS-RECORD-ja.md）に行を足す。Full Test とは違い、申し送りは消さず、main にもマージしない。

* clone は `git clone -c core.autocrlf=false -c core.symlinks=true` で作る（CRLF の台本は bash で動かず、CRLF のログは VM の出力と合わない。シンボリックリンクには Windows の開発者モードが要る）。今ある clone なら、`git config core.autocrlf false`、`git config core.symlinks true` の後に `git rm -q --cached -r . && git reset -q --hard` で取り出し直す。
* 本当の Python 3 が要る（Python 版の VM を使わなくても、bin/includer などの道具が Python で動く）。python.org の版を入れ、その python.exe を同じディレクトリに python3.exe として写し、そのディレクトリを PATH の先頭に置く（PATH の `python3` は、ふつうは Microsoft Store を開くアプリ実行エイリアス）。
* PATH に cargo（`$HOME/.cargo/bin`）と Node.js も置く。`MSYS=winsymlinks:nativestrict` を付ける（test/step16 と step23 の `ln -s` が、本当のシンボリックリンクを作るように）。
* `CONMA_VM` を必ず付ける（bin/vm の既定は Python 版の VM で、Windows では動かない。パイプを `select` で待つため）。Python 版の VM の列は流さない。
* 台本は、`cygpath` があれば（Git Bash）、`pwd` と一時ディレクトリを VM が書く形（`C:/Users/...`）にする（test/common.sh、consh/test/common.sh）。
* wasm 版の VM のホスト（vm/wasm/vm-node.js）は、Windows では、パスを VM が書く形にして渡し、Node.js の errno を Unix の番号にし、コマンドを PATH（`;` で区切る）から `.exe` を付けて探し、引数をネイティブの VM と同じく引用する（2026-11-23 から）。

```
export PATH="$LOCALAPPDATA/Programs/Python/Python315:$HOME/.cargo/bin:$PATH" MSYS=winsymlinks:nativestrict
(cd vm && cargo build --release && cargo test --release --lib)
CONMA_VM=rust test/run_all.sh -j 8
CONMA_VM=rust consh/test/run_all.sh -j 6
conjs/test/run_all.sh
sjs/test/run_all.sh
vm/wasm/build.sh                                       # wasm 版の VM（2026-11-23 から。CI の windows ジョブは流さない）
CONMA_VM=wasm test/run_all.sh -j 4
CONMA_VM=wasm consh/test/run_all.sh -j 4
CONMA_VM=wasm conjs/test/run_all.sh
```

## 既知の違い

これらは前からある違いで、変更の誤りではない。これ以外の違いが出たら、誤りを疑う。

| VM や版 | テスト | 理由 |
|---|---|---|
| wasm 版 | test/step40 | ログはネイティブの VM のもので、JavaScript のある wasm 版では case1 の呼び出しが Stream を受け取る（docs/dev/ROADMAP.md の Step 40） |
| wasm 版 | consh/test の step17 | JavaScript のない版の `js` のテストで、wasm 版には JavaScript があるので、メッセージと `$?` が違う（`JavaScript is disabled`、2） |
| wasm 版 | consh/test の step8 | VProc を切り替える時の細かい所による（docs/dev/FAST-PLAN-ja.md の 3.3） |
| wasm 版（ときどき） | consh/test の step12 の case8 | 疑似端末で Ctrl-C を押すテストで、Ctrl-C の後にプロンプトが 2 回出て、その後の出力が 1 つずつずれる（GitHub Actions の Ubuntu で 2 回、macOS で 1 回。2026-11-22）。ConSh の対話モードの競り合いで、前景の sleep が終わった後に VM が SIGINT に気づくと、見張りの VProc（consh/include/consh_repl.se の Repl_watch）は、実行中のコマンドがないので、プロンプトで押した Ctrl-C として、改行と新しいプロンプトを出し、読みかけの入力を捨てる。wasm 版（Node.js）では、SIGINT と子の終わりが別のイベントで来るので、順序が入れ替わることがある。遅れた SIGINT と、新しいプロンプトで押した Ctrl-C を見分ける方法を決めるまで、既知の違いにする |
| どの VM でも、機械が混んでいるとき | test/step31 | SIGINT を 2 回送るテストで、混んでいると 2 回が 1 回にまとまることがある。単独で流せば通る（`-j 8` の wasm 版で見られた） |
| 標準ライブラリの参照版（`CONMA_REFERENCE=1`） | test/step18 の case12、step20 の case15、step21 の case6、fast3、fast9 | util.se そのものに依るログ：エラーの位置が reference/util.se の行になる、数える時点のステップがずれる、印を付けた Body の数や大域の番号が違う（reference/README-ja.md） |
| ConSh の Rust 版（`CONSH_IMPL=rust`） | consh/test の examples | Rust 版は `(( ))` を読めず、consh/examples/nbody.sh が `expected a command, but found '('` で止まる（nbody.sh を足した 2026-11-15 から） |
| Windows（I7）のネイティブの VM と wasm 版 | test/ の step11、13、14、15、16、23、consh/test の examples と step16 | Unix の絶対パス（`/bin/sh`、`/bin/cat`、`/bin/echo`、`/usr/bin/env`、`/usr/bin/true`、`/usr/bin/grep` など）でプログラムを起動する。Windows にそのパスはない（2026-11-22 から。段階 P6c。wasm 版は 2026-11-23 から） |
| Windows（I7）のネイティブの VM と wasm 版 | test/step11 の `/etc/hosts`、test/step16 の `realpath /` | `Permission denied` でなく `No such file or directory`（Windows に /etc/hosts はない）。`/` の realpath は `C:/` |
| Windows（I7） | test/step21 の case5 と case6 | memcheck.py が Python 版の VM を使う（Windows では動かない） |
| Windows（I7） | test/step31、test/step33 の case4、consh/test/step12 の case7 と case8 | Git Bash の `kill -INT` は Windows のプログラムに届かず、Python に pty がない。流さず、その旨を 1 行出す |
| Windows（I7）の wasm 版 | test/fast7 の case7 と case14 | Git Bash の `kill -INT` は Windows のプログラムに届かず、wasm 版は終わらない関数を呼び続ける。流さず、その旨を 1 行ずつ出す（ネイティブの VM は WebAssembly がないので、すぐ終わり、ログのとおり） |
| Windows（I7） | test/step32 | Git Bash の `ps` に `-o` がない。`another group` でなく `same group` |
| Windows（I7） | consh/test の step2 | `words` はシバンで始まる台本で、Windows はそれを直接起動できない。MSYS の `sh` がシグナルで終わると、終了コードは 143 でなく 3840 |
| Windows（I7） | consh/test の step3、step4、step11 | 外部のコマンドの `pwd`（Git Bash）が `/c/...` の形を書き、`cd /` の後は `/c`。step11 は、その形のパスの glob が一致しない |
| Windows（I7） | consh/test の step6 | `/dev/nul?` の glob が一致しない（Windows に /dev はない） |
| Windows（I7） | consh/test の step13 と step14 | lsof がない（台本がその旨を出す） |
| Windows（I7） | （テストにはない） | MSYS（Git Bash）のプログラムは、引用符の中の `\\` を 1 つの `\` に読むので、ConMa が渡す引数の 2 つ続く `\` は 1 つで届く（Windows の C ランタイムと MSYS の両方に合う書き方がない。vm/src/sys/windows.rs の `add_args`） |

## 申し送り

次に全部流す人への申し送り。作業した人が、作業のブランチの見出し（`### m4/chan` など）の下に書いて alpha にマージし、テストの担当者は、その見出しを含む alpha を自分のブランチ（`m2/test` など。機械と `test`）で全部流して確かめたら、その見出しの行を消して記録（docs/dev/TESTS-RECORD-ja.md）の行を足す（docs/dev/CONTRIBUTING.md の「Branches and Git settings」）。見出しを書かないのは、変えたファイルがすべて文書（docs/dev/CONTRIBUTING.md の Branches の 5 の、`docs/` と `.github/` の下のファイルと、`*.md`）で、ベンチマークも頼まないブランチだけ（2026-11-21 から。設計の案など。変えたものは、コミットのメッセージと PR にある）。ほかのファイルを 1 つでも変えたブランチには、どの suite も読まないと思うファイル（`.gitignore`、見本のページなど）だけでも、見出しを書く（テストの担当者は、それも含めて全部流すので、何が変わったかを知る必要がある）。Full Bench（Full ConMa Bench、Full ConSh Bench、Full SJS Bench、Full ConJS Bench）も、ここで「ベンチマーク：」で始まる行で頼む。行には、4 つのどれか、測るコミット（7 桁。前後を比べるなら前のコミットも）、版（最適化版＋transpiled VM など）を書き、測る人は書かない。Full Bench は、ディレクトリのすべてのベンチマークを、書いた版だけで測る（すべての版で流す Full Test とは違う。版を書かなければ、表が比べるすべての版）。誰がいつ測るかはユーザーが決め、測る人は、その行を見ただけでは始めない。Full Bench でない測り直しは、変更した開発者が、ユーザーの指示を待たずに自分で測る（2026-11-18 から。それまでは、どの測り直しもここで頼んでいた）。その行は、測った人が、測ったブランチ（`m2/bench-1a2b3c4` など。機械と、測ったコミットの 7 桁）で消すまで残し（テストが済んでも消さない）、見出しは、その下の行がすべてなくなったときに消す（同じ文書の「Benchmarks」）。全部流している間に alpha に入ったブランチの見出しは、次に流すまで残す（記録には流したコミットだけを書く）。2026-11-16 までは、作業のブランチを作らずに alpha に書いていたので、見出しは alpha にした。見出しの下には「種類：」の行を書き、機能（機能を足した）、直し（直しただけ）、両方、なし（テスト、ベンチマーク、ワークフローだけ）のどれかを書く。テストの担当者は、Full Test で消す見出しのこの行を見て VERSION を上げる（2026-11-24 から。docs/dev/CONTRIBUTING.md の Versions）。

### m4/conjs-host-plan（shima3/conma#75）

* 種類：なし（計画と、測るための台本だけ）。
* docs/dev/CONJS-HOST-PLAN.md（ConJS の関数を、wasm 版の VM のとき、ホストの JavaScript で動かす計画）と、その段階 H0 で測った台本 conjs/bench/hostjs（nbody_host.js、hof_host.js、crossing.con.js）を足した。suite は、どれもこの台本を読まない。
* 作り直すもの：なし。
* M4 が流したもの：conjs/bench/hostjs の 3 つ（Node.js と wasm 版の VM。計画の 11 節）。
* 流していないもの：ほかの suite（ランタイムも VM も変えていない）。
* 測ったもの：計画の 11 節。

### m4/conjs-host-h1（shima3/conma#75）

* 種類：機能（wasm 版の VM で、ホストの関数から VM の関数を呼び返す）。
* docs/dev/CONJS-HOST-PLAN.md の段階 H1。vm/src/prim/host.rs に入れ子の実行（`callback`）を、vm/src/sched.rs に `nested` と `set_booleans` を、vm/src/gc.rs に入れ子の実行の間は集めない印を、vm/wasm/src/lib.rs に `conma_callback` を、vm/wasm/conma.js にホストの関数の `this.callVm` を足した。ネイティブの VM の動きは変わらない（`__Host_call__` の間に PDict を預けて戻すだけ）。
* 作り直すもの：Rust 版の VM（`cd vm && cargo build --release`）と wasm 版の VM（vm/wasm/build.sh）。consh/js/build.sh の作るブラウザ版も、wasm を作り直す。
* 新しいテスト：test/fast19（呼び返し。wasm 版の VM のログは case1.wasm.log、ほかの VM は case1.log）。
* M4 が流したもの：test/fast19 を wasm 版、Rust 版、Python 版、transpiled VM で。Rust 版と wasm 版の VM の test/、consh/test、consh/js/test、sjs/test、Rust 版と wasm 版の VM の conjs/test、test/host-api.sh、vm の `cargo test --release`。
* 流していないもの：Python 版の VM と transpiled VM の、fast19 のほかの suite。Windows（I7）。
* 測ったもの：呼び返し 1 回の時間（計画の 12 節）。

### m4/conjs-host-h2（shima3/conma#75）

* 種類：機能（ホストの JavaScript の関数を、VM の値を写して呼ぶプリミティブ）。
* docs/dev/CONJS-HOST-PLAN.md の段階 H2（H2a と H2b）。プリミティブ `__JS_fn_compile__`、`__JS_fn_call__`、`__JS_fn_resume__`（プラットフォームの機能 "js"。vm/src/prim/jsfn.rs、Python 版の VM は onError、docs/PRIMITIVES.md）。bin/resolver、include/resolver.se、reference/resolver.se の "js" に 3 つを足し、vm/frontend.module.se を bin/update-frontend で作り直した。wasm 版の VM のホスト（vm/wasm/conma.js）が、値のグラフ（同じもの、輪、ハンドル）を受け取り、変えたものを書き戻す。呼び出しごとのセッションが写したものの表を持ち、変わったものだけを送り合う。ジェネレータの yield と再開（`__JS_fn_resume__`）。呼び返しも、同じ表で値を渡す（conma_jsfn_callback）。
* 作り直すもの：Rust 版の VM（frontend.module.se を埋め込む）、wasm 版の VM、transpiled VM（新しいプリミティブ。bench/build.sh）。consh/js/build.sh の作るブラウザ版も。
* 新しいテスト：test/fast20 と test/fast21（wasm 版の VM は case*.wasm.log、ほかの VM は case*.log）。
* M4 が流したもの：test/fast20 と fast21 を wasm 版、Rust 版、Python 版、transpiled VM で。Rust 版、wasm 版、Python 版の VM の test/、Rust 版と wasm 版の VM の consh/test と conjs/test、consh/js/test、sjs/test、test/host-api.sh、vm の `cargo test --release`。
* 流していないもの：Python 版の VM の consh/test と conjs/test、transpiled VM の fast19〜21 のほかの suite、Windows（I7。vm/src/sys/windows.rs に足した関数がビルドできるかも）。
* 測ったもの：写す費用、yield と再開、呼び返し（計画の 13 節と 14 節）。

### m4/conjs-host-h3（shima3/conma#75）

* 種類：機能（ConJS の関数を、wasm 版の VM のとき、ホストの JavaScript で動かす）。
* docs/dev/CONJS-HOST-PLAN.md の段階 H3（H3a と H3b）と H4（H4a はクラスのメソッド、H4b は new、instanceof、throw、try、H4c の B1 と B2 は VM に頼む遅い道と呼び返し）。変換器に sjs/include/sjs_host.se（conjs/conjsc.se が include する）を足し、sjs/include/sjs_gen.se が、ConJS のとき、外の変数を読み書きしない関数（値、配列、オブジェクト、アロー関数、console.log）を JavaScript にして、Conjs_hostfn で包む。ランタイムは conjs/include/conjs_host.se（conjs_rt.se が include する）。数の関数でもある関数は、wasm 版、ホストの版、ConMa 版の順に試す。alpha（#70〜#74 と M1 の 7 回目の Full Test）を取り込んだ。
* 作り直すもの：Rust 版の VM、wasm 版の VM（H1、H2 から）。transpiled VM（sjs_gen.se が変わって、ConSh のモジュールが変わる）。
* 新しいテスト：conjs/test/step20（7 つの場合。どの VM でも同じログ）。wasm 版の VM では、conjs/test/run_all.sh が、ホストで値を返した関数の名前を step20/case*.host.txt と比べる。
* M4 が流したもの：conjs/test（Rust 版、wasm 版（`CONJS_HOST_STRICT=1`）、Python 版）、conjs/test/compare.sh、sjs/test（Rust 版と wasm 版）、consh/test（Rust 版と wasm 版）、consh/js/test、Rust 版の VM の test/、test/host-api.sh。
* 流していないもの：transpiled VM（作り直していない）、Windows。
* 測ったもの：計画の 15 節〜19 節（19 節：呼び返しは 1 回 31 µs で、ConMa の 15 µs より遅い）。

### m4/conjs-host-b3（shima3/conma#76）

* 種類：機能（待つ関数も、wasm 版の VM のとき、ホストの JavaScript で動かす）。
* docs/dev/CONJS-HOST-PLAN.md の段階 H4c の B3（20 節）。sleep、join、nextEvent か、待つホストの関数を呼ぶ一番外の関数を、JavaScript のジェネレータにして、待つ所で VM に yield し、Conjs_hostfn がその処理（VProc）で頼みを動かして（待ってよい）、__JS_fn_resume__ で続ける（sjs/include/sjs_host.se、conjs/include/conjs_host.se）。conjs/README の 2 つに、待つ関数のことを足した。
* 作り直すもの：なし（VM は変えていない。変換器のモジュールは、bin/cache-dir の所で作り直される）。
* 新しいテスト：conjs/test/step20/case8（どの VM でも同じログ。case8.host.txt）。
* M4 が流したもの：conjs/test（Rust 版、wasm 版（`CONJS_HOST_STRICT=1`）、Python 版）、conjs/test/compare.sh、sjs/test（Rust 版と wasm 版）、doc-pairs.py。
* 流していないもの：consh/test と test/（ConJS の変換器とランタイムしか変えていない）、Windows。
* 測ったもの：待つ 1 回の費用と、待つ間に仕事のある繰り返し（計画の 20 節）。
* M1 のレビュー（650b45f）の指摘で、待った後（呼び返しの後も）に、ホストが確かめられない組み込み（ASCII でない文字列のメソッド、初めの値のない reduce、比べる関数のない sort、Math、Object.keys、配列でない値の for...of）を VM で動かすようにし、止まるときのメッセージにわけ（after waiting か after calling back）を出す。case8 に足した。M4 が流したもの：conjs/test（Rust 版、wasm 版（`CONJS_HOST_STRICT=1`）、Python 版）、compare.sh、sjs/test（Rust 版と wasm 版）。
