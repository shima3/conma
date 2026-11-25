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
conjs/test/run_all.sh                                  # ConJS（wasm 版の VM だけ。段階 P10、2026-11-25 から）
```

* ConJS のブラウザのページは、CI にブラウザがないので、手で確かめる（docs/dev/CONJS-HOST-PLAN.md の 32 節、段階 H7）：`conjs/web/build.sh` の後、`python3 -m http.server -d conjs/web -b 127.0.0.1 8768` で出し、bench.html の「すべて測る」で、出力の列がすべて「同じ」になること、stars.html?hostreport で遊ぶと、ブラウザのコンソールに `conjs: host: ` の行が出ること。conjs/test/run_all.sh は、step20 を、Node.js で new Function を止めて（`--disallow-code-generation-from-strings`）もう一度流す（== step20 (no eval)。eval を止めたページの代わり）。
* 標準ライブラリの参照版で test/ を流すときは、`CONMA_REFERENCE=1` を付ける（reference/README-ja.md）。下の既知の違いがある。
* consh/test は、`CONSH_IMPL` がなければ ConMa 版の最適化版を試す（consh/test/common.sh）。`CONMA_VM` がなければ、ConSh は Rust 版の VM を使う（作っていなければエラー。Python 版の VM で流すときは `CONMA_VM=python`）。

### CI（GitHub Actions）

PR ごとと alpha への push ごとに、GitHub Actions が、速い suite を Linux と macOS で流す（.github/workflows/ci.yml、2026-11-22 から。docs/dev/CONTRIBUTING.md の Tests）。流すのは、Rust 版と wasm 版の VM の test/、Rust 版と wasm 版の VM の consh/test、consh/js/test、sjs/test、wasm 版の VM の conjs/test、vm の `cargo test --release`、test/host-api.sh。それぞれを .github/ci-suite.sh で流し、下の既知の違いのほかに違いがあれば失敗にする（既知の違いが出なくても失敗にはしない）。既知の違いの表を変えたら、ci.yml の、その suite の行も変える。Python 版と transpiled VM、ConSh の参照版と Rust 版、`CONMA_REFERENCE=1` は流さないので、CI が通っても Full Test の代わりにはならず、記録（docs/dev/TESTS-RECORD-ja.md）にも書かない。Windows では、下の「Windows（I7）で流す」のとおりに用意して、vm の `cargo test --release --lib` と、Rust 版の VM の test/、consh/test、sjs/test、wasm 版の VM の conjs/test を流す（段階 P10 から、conjs/test は wasm 版。既知の違いは、下の表の I7 の行。段階 P6c から。docs/dev/PLATFORM-PLAN-ja.md の 4.11 節）。

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
sjs/test/run_all.sh
vm/wasm/build.sh                                       # wasm 版の VM（2026-11-23 から）
CONMA_VM=wasm test/run_all.sh -j 4                     # CI の windows ジョブは流さない
CONMA_VM=wasm consh/test/run_all.sh -j 4               # CI の windows ジョブは流さない
conjs/test/run_all.sh                                  # ConJS（wasm 版の VM。段階 P10 から。CI の windows ジョブも流す）
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

### m1/sched-dict（shima3/conma#88）

* 種類：直し（ConJS の処理の切り替えを、少し速くした。動きは変えていない）。
* include/lang/sched.se：処理とスケジューラの状態を、Ref に入れず Dict に直に置き、`__Dict_put__` で書き換える（`Kit_sched_new_ref` を `Kit_sched_init` にした）。conjs/include/conjs_rt.se：処理だけの 3 つの PDict のキーの保存と戻し（`Conjs_save`、`Conjs_restore`）を、名前の LList を回す再帰から、3 つを並べて書く形にした。sched.se を読み込むのは、conjs_rt.se と test/fast18 だけ（SJS と ConSh は読まない）。
* conjs/bench の tasks（Rust 版の VM の `CONMA_PROFILE`、大きさ 100 と 200 の差）：1 回 14,364 → 12,678 ステップ（−11.7%）。大きさ 15,000 で alpha（2750a43）と交互に 3 回ずつ（Apple M1）：wasm 版 10.04〜10.06 秒 → 9.45〜9.52 秒、Rust 版 7.53〜7.57 秒 → 7.11〜7.18 秒（約 −5.5%）。conjs/bench/README-ja.md の表は、次の Full ConJS Bench で。
* M1 が流したもの（Apple M1）：conjs/test（wasm 版）、conjs/test/compare.sh、test/host-api.sh、test/fast18（Python 版、Rust 版、wasm 版、transpiled VM）：違いなし。

### m1/h7（shima3/conma#87）

* 種類：機能（ConJS のページに、bench.html と `?hostreport` を足した）。
* docs/dev/CONJS-HOST-PLAN.md の段階 H7（32 節の設計、33 節の結果）：conjs/web/hostreport.js（`?hostreport`。stars.html、canvas.html、bench.html）、conjs/web/bench.html（conjs/bench の 7 つを、ホストの関数ありとなしで測る）、conjs/web/build.sh が bench_NAME_conjs.module.se も作る（.gitignore に足した）。docs/conjs/index.md からも指した（main に入ってから、サイトに出る）。
* conjs/test/run_all.sh：step20 を、`NODE_OPTIONS=--disallow-code-generation-from-strings` で、`CONJS_HOST_STRICT` を消して、もう一度流す（`== step20 (no eval)`。約 7 秒。`CONJS_HOST_STRICT=1` で run_all.sh を流しても通る）。全部流すときは、この行も違いがないこと。
* 作り直すもの：`conjs/web/build.sh`（ページを見るとき）。
* M1 が流したもの（Apple M1）：conjs/test（wasm 版。新しい行を含む）：違いなし。Claude のアプリの組み込みのブラウザ（Chromium）で、bench.html の「すべて測る」（7 つとも出力が同じ。時間は 33 節）、stars.html?hostreport で遊んで、コンソールに `conjs: host: same` などが出ること、canvas.html?hostreport と、`?hostreport` なしの stars.html が動くこと。
* 流していないもの：Safari、Firefox、Windows のページ（手で確かめる。上の「全部流す」の項）。

### m1/wasm-tty-exit（shima3/conma#83）

* 種類：直し（wasm 版の VM で、標準入力が端末のプログラムが、終わった後、次のキーまで終わらないことがあった）。
* vm/wasm/vm-node.js：標準入力が端末のときは、`fs.read` でなく `tty.ReadStream` で読む（`fs.read` は libuv のスレッドで待ち、`process.exit` がそのスレッドを待つ）。端末でない標準入力と Windows は、今の道のまま。docs/dev/PLATFORM-PLAN-ja.md の 4.13 節の 5 に、調べたことを書いた。
* M1 が流したもの（Apple M1）：conjs/test（wasm 版）、conjs/test/compare.sh、test/host-api.sh、`CONMA_VM=wasm` の sjs/test、test/run_all.sh、consh/test：違いなし。テストは標準入力が端末でないので、この道を通らない。pty（Python の pty）の中で、stars-term.con.js を wasm 版と Rust 版で矢印で動かして Escape で終えること、1 つのキー（文字、矢印、Escape）を読んで終えるプログラム、端末から行を読んで Ctrl-D で終えるプログラムを確かめた。直す前は、wasm 版が Escape の後に終わらなかった。
* 足していないもの：テスト（端末が要る）。Windows では試していない（道を変えていない）。
* 見つけたこと（直していない）：wordcount.con.js を標準入力から wasm 版で動かすと止まる（4.13 節の 5。段階 H6c の道）。

### m1/p10（shima3/conma#82）

* 種類：機能（ConJS を動かす既定の VM が、Rust 版から wasm 版に変わる。使う人から見て変わるので、機能とした）。
* 段階 P10（docs/dev/PLATFORM-PLAN-ja.md の 4.14 節）：ConJS を wasm 版の VM だけで動かす。bin/lang が lang.conf の `vm=` を読み（SJS には書かないので、SJS は今のまま）、conjs/lang.conf を `vm=wasm` にした。wasm 版の VM がなければ、変換する前に作り方を出して終える。bin/lang-test も、ログを選ぶ VM に `vm=` を使う。
* conjs/test：6 つの `case*.wasm.log` を `case*.log` にし、ログを 1 つずつにした。step21 を足した（async/await で書いたホストの関数を `host.wait` で呼ぶ。docs/HOST-ja.md の 3.2 節の例）。step18 の case2.node-check.js は case2.log を読む。test/host-api.sh の Rust 版の行に `CONMA_VM=rust` を付けた。
* CI：unix のジョブから conjs/test の Rust 版を除いた。windows のジョブで wasm 版の VM を作り（`rustup target add wasm32-unknown-unknown` と `vm/wasm/build.sh`）、conjs/test を wasm 版で流す。使われていない shasum の手順を除いた。windows のジョブで初めて wasm 版の conjs/test を流すので、違いがないこと（既知の違いの表に足す行がないこと）を、この PR の CI で確かめる。
* 全部流す：conjs/test は wasm 版の 1 行だけになった（上の流し方）。Windows Test も同じ。
* 文書：conjs/README と QUICKSTART の組、docs/dev/CONJS.md、docs/HOST-ja.md、TESTS-ja.md、CONTRIBUTING の Tests、conjs/bench（run.sh は Node.js と wasm 版の VM だけを測る）。
* M1 が流したもの（Apple M1、m1/p10 の作業中のワークツリー。Rust 版と wasm 版の VM を alpha の 584c044 で作り直して）：conjs/test（既定の wasm 版。違いなし。step21 を 4 回）、conjs/test/compare.sh、test/host-api.sh、`bin/lang-emit-check origin/alpha`（違いなし）、sjs/test を Rust 版と wasm 版の VM で（違いなし）。`console.log("hi")` の起動は、wasm 版で約 0.18 秒、Rust 版で約 0.08 秒（交互に 3 回ずつ）。wasm 版の VM がないときのメッセージと、`--emit` はそれでも動くことを、wasm のファイルを一時的に名前を変えて確かめた。
* sjs/test の Python 版の VM：違いが出た（標準出力と標準エラーの順、VM の Traceback）。alpha（584c044）の worktree で流しても、別のステップで違いが出て、出る所は流すたびに違う（M1 の 2 回）。P10 は SJS の道を変えていない（`vm=` は ConJS の設定だけ）ので、P10 の違いではないと見た。Python 版の VM の sjs/test は Full Test に入らない。
* 流していないもの：Windows（S5 か I7 に頼む）、Python 版と transpiled VM の conjs/test（P10 からテストしない）、ベンチマーク（startup のほかは測っていない）、ブラウザのページ（変えていない）。

### m1/stars-split（shima3/conma#78）

* 種類：なし（見本の書き直しだけ。ランタイムも VM も変えていない）。
* 星ひろいの端末版（conjs/examples/stars-term.con.js）とブラウザ版（conjs/web/stars.con.js）を分けた。端末とブラウザで違う関数（画面の順、面の進め方、描き方）はそれぞれのファイルに書き、両方で同じ関数（面の作り方、おばけ、キーで動くこと）は、端末版がブラウザ版から import する。描き方を ui にまとめて `play` に渡す形はなくした。入力の終わりの Error を受け止めるのは、端末版だけになった。conjs/test の step14 と step15 の case2 は、この 2 つへのリンクなので、ログは変わらない。
* 作り直すもの：conjs/web/build.sh の作るページの stars_conjs.module.se。
* M1 が流したもの：conjs/test の step14 と step15 を Rust 版、wasm 版、Python 版の VM で。端末版をパイプの入力で Rust 版と wasm 版の VM で動かし、前の版と同じふるまい（Rust 版は始める前に入力の終わり、wasm 版は Escape で `stars: bye`）。stars.html をブラウザで遊び、キーとクリックで動くこと、星を拾うこと、Escape で終わること。compare.html の表示。
* 流していないもの：ほかの suite（見本しか変えていない）。Windows（I7）。

### m4/host-forof（shima3/conma#84）

* 種類：直し（ConJS の一番外の for...of が、VM の値の次の値を待つと止まっていた）。
* docs/dev/CONJS-HOST-PLAN.md の 31 節。M1 が shima3/conma#83 で見つけた（wordcount.con.js を標準入力から読むと止まる）。段階 H6c から、一番外の文が VM へ渡れるようになり、一番外の for...of が、VM のジェネレータの次の値を入れ子の呼び返しで求めて、それが入力を待つと止まっていた。ジェネレータの本体（待つホストの関数と一番外の文）の for...of を、頼み（yield）で 1 つずつ進める形にした（sjs/include/sjs_host.se）。
* 作り直すもの：なし（変換器のモジュールは、bin/cache-dir の所で作り直される）。
* 新しいテスト：conjs/test/step20/case14（case14.feed.py が、標準入力をパイプで少しずつ渡す）。M1 の #84 のレビューで、ふつうのホストの関数の for...of も、呼び返しの前なら VM の値を回さずに ConMa 版に替えるようにした（標準入力の行を関数に渡して回すと止まっていた。B2 から）：case15（case15.feed.py）。
* M4 が流したもの：conjs/test（`CONJS_HOST_STRICT=1`）、conjs/test/compare.sh、sjs/test、conjs/bench の 8 つ（小さな大きさで Node.js と同じ出力）、wordcount.con.js をパイプの入力で、doc-pairs.py。
* 流していないもの：test/ と consh/test（ConJS の変換器しか変えていない）、Windows。
