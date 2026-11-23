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

2026-11-22 から（段階 P6c。docs/dev/PLATFORM-PLAN-ja.md の 4.11 節と 8.13 節）。Git Bash で流す。

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

次に全部流す人への申し送り。作業した人が、作業のブランチの見出し（`### m4/chan` など）の下に書いて alpha にマージし、テストの担当者は、その見出しを含む alpha を自分のブランチ（`m2/test` など。機械と `test`）で全部流して確かめたら、その見出しの行を消して記録（docs/dev/TESTS-RECORD-ja.md）の行を足す（docs/dev/CONTRIBUTING.md の「Branches and Git settings」）。見出しを書かないのは、変えたファイルがすべて文書（docs/dev/CONTRIBUTING.md の Branches の 5 の、`docs/` と `.github/` の下のファイルと、`*.md`）で、ベンチマークも頼まないブランチだけ（2026-11-21 から。設計の案など。変えたものは、コミットのメッセージと PR にある）。ほかのファイルを 1 つでも変えたブランチには、どの suite も読まないと思うファイル（`.gitignore`、見本のページなど）だけでも、見出しを書く（テストの担当者は、それも含めて全部流すので、何が変わったかを知る必要がある）。Full Bench（Full ConMa Bench、Full ConSh Bench、Full SJS Bench）も、ここで「ベンチマーク：」で始まる行で頼む。行には、3 つのどれか、測るコミット（7 桁。前後を比べるなら前のコミットも）、版（最適化版＋transpiled VM など）を書き、測る人は書かない。Full Bench は、ディレクトリのすべてのベンチマークを、書いた版だけで測る（すべての版で流す Full Test とは違う。版を書かなければ、表が比べるすべての版）。誰がいつ測るかはユーザーが決め、測る人は、その行を見ただけでは始めない。Full Bench でない測り直しは、変更した開発者が、ユーザーの指示を待たずに自分で測る（2026-11-18 から。それまでは、どの測り直しもここで頼んでいた）。その行は、測った人が、測ったブランチ（`m2/bench-1a2b3c4` など。機械と、測ったコミットの 7 桁）で消すまで残し（テストが済んでも消さない）、見出しは、その下の行がすべてなくなったときに消す（同じ文書の「Benchmarks」）。全部流している間に alpha に入ったブランチの見出しは、次に流すまで残す（記録には流したコミットだけを書く）。2026-11-16 までは、作業のブランチを作らずに alpha に書いていたので、見出しは alpha にした。

### m4/stars-js（shima3/conma#57）

* conjs/web：「星ひろい」（stars.con.js）を、ふつうのブラウザの JavaScript で 2 通りに書いたものを足した。stars-events.js（イベント駆動：画面を表す変数 state とキーの受け手の中の分岐、おばけの setInterval と clearInterval、描くのは requestAnimationFrame）と stars-async.js（async/await：ConJS 版と同じ形のループで、nextEvent、sleep、1 コマを待つことを Promise で自分で作る）。遊ぶページ stars-events.html と stars-async.html、3 つのうち 2 つを並べて見せる compare.html（行の数、コメントと空の行を除いた行の数、待つ所の行の数を出し、待つ所の行を黄色にする）。stars.html と docs/conjs/index.md にリンクを、conjs/README の 2 つに説明を足した。どちらも ConMa の VM を使わないので、build.sh は変えていない。決まり（盤、色、乱数、おばけの数と間隔、画面の順、結果の画面の 300 ミリ秒、面クリアの 800 ミリ秒）は ConJS 版と同じ。
* 作り直すもの：なし。
* 新しいテスト：なし（DOM を使うので、suite で比べられない）。
* M4 が流したもの：`node --check`。2 つの JS 版を、何も描かない canvas の代わりと、手で送る keydown と pointerdown の上で、Node.js から遊ばせた（描いたものから星と自分の位置を読み、近い星へキーとクリックで動く）。タイトル（Shift では始まらない）、1 面のクリアと「1 面クリア」の後の 2 面、おばけにぶつかった後の結果の画面、その直後のキーを捨てること、300 ミリ秒の後のキーでタイトル、Escape で「おしまい」。compare.html のスクリプトも、ファイルを読む fetch の代わりの上で動かし、数を確かめた（stars.con.js 263 行・214 行・待つ所 10、stars-events.js 251・206・5、stars-async.js 280・234・27。M2 のレビューの後、文字列、行の終わりのコメント、配列の .join を数えないようにした）。
* 流していないもの：本当のブラウザ（このセッションのブラウザは、M4 の clone のページを出せなかった）。
* 測ったもの：なし。

### i7/wasm-windows（shima3/conma#58）

* vm/wasm/vm-node.js（wasm 版の VM のホスト）：Windows（I7、Git Bash）で動くようにした。どれも `process.platform` が `win32` のときだけで、Unix は変わらない。
  * パス：VM に渡すパス（今のディレクトリ、realpath、`--include`、モジュールのファイル名から除く上のディレクトリ）を、ネイティブの VM と同じ `C:/Users/...` の形にした（前は `C:\Users\...` で、test/step10 などが `--include directory not found` で止まっていた）。VM から来るパスは、`\` を区切りとし、wasm 版の VM が Unix のように今のディレクトリの後につないだ Windows のパス（`C:/a/C:/b`、TEMP からの `C:/a/C:\b`）を、最後のドライブから取る。`/dev/null` は NUL、`/dev/stdin`、`/dev/stdout`、`/dev/stderr` は fd 0、1、2 の写し（ネイティブの VM の open_file と同じ）。途中の名前がないときは ENOENT、ファイルなら ENOTDIR（check_on_the_way と同じ）。
  * errno：Node.js の errno は Windows では libuv の番号（ENOENT が -4058）で、VM が `operation successful (os error N)` と書いていた。名前（`e.code`）から Unix の番号にした。
  * プロセス：コマンドを PATH（`;` で区切る）から、拡張子がなければ `.exe` を付けて探す（Rust の Command と同じ）。引数は、ネイティブの VM の add_args と同じく、MSYS のプログラムが別に読む文字があれば引用する（`windowsVerbatimArguments`）。`__OS_kill__` はどのシグナルでもプロセスを終わらせ、終了コードはそのシグナルの番号の負（ネイティブの VM の kill_process と exit_code と同じ）、Ctrl-C で終わったプロセスは -2。読み手のいないパイプに書くプロセスに SIGPIPE を送らない（Windows にない。書くとエラーになる）。
* test/fast7/test.sh：Windows（`cygpath` がある）の wasm 版の VM では、case7 と case14 を流さず、その旨を 1 行ずつ出す。Git Bash の `kill -INT` が VM に届かず、終わらない関数を呼び続けて止まらない（ネイティブの VM は WebAssembly がないので、すぐ終わる）。既知の違いの表に足した。
* docs/dev/TESTS-ja.md：既知の違いの、Unix の絶対パスと `/etc/hosts`、`realpath /` の 2 行は、wasm 版にも当てはまる。「Windows（I7）で流す」に wasm 版の VM の suite を足した（CI の windows ジョブは、まだ流さない）。
* 作り直すもの：なし（vm-node.js と台本だけ。wasm の VM は変えていない）。
* 新しいテスト：なし。
* I7 が流したもの（Windows 11、Node.js 26、alpha 29c28d6 から）：`CONMA_VM=wasm test/run_all.sh -j 4`（I7 の既知の違いと wasm 版の step40、fast7 の 2 行だけ。直す前は 23 のステップが違った）、`CONMA_VM=wasm consh/test/run_all.sh -j 4`（I7 の既知の違いと wasm 版の step17 だけ）、`CONMA_VM=wasm conjs/test/run_all.sh`（違いなし）、`sjs/test/run_all.sh`（step10 の case2 だけ。shima3/conma#54 で直る、このブランチにない Windows のスタックの違い）。同じ機械の `CONMA_VM=rust` の test/ と consh/test と、違うステップと出力を比べた（wasm 版の既知の違いのほかは同じ）。
* 流していないもの：Unix のすべて（変えた行は、Windows でなければ前と同じ。CI が Linux と macOS で wasm 版を流す）。
* 気をつけること：test/ と consh/test のログは、リポジトリのディレクトリの名前が `conma` であることを前提にしている（`conma/test/step1/test.se`）。worktree（`.claude/worktrees/NAME`）で流すと、ほとんどのステップが違う。名前が `conma` のディレクトリに worktree を作って流した。
* 測ったもの：なし。

### i7/wasm-windows-stack（shima3/conma#54）

* vm/wasm/vm-wasm.sh：Windows（Git Bash。`cygpath` がある）で、Node.js に渡すスタック（`--stack-size`）を node.exe の主スレッドのスタック（実行ファイルに書かれた 8 MB）より小さくした。Git Bash の `ulimit -s` は上げたと言うだけで、node.exe のスタックは変わらないので、`--stack-size` が 943718 KB になり、深い再帰で Node.js が何も出さずに 127 で終わっていた（sjs/test/step10 の case2。wasm 版の VM を作ってあるときだけ動くテスト）。Unix は変わらない。
* 作り直すもの：なし（台本だけ）。
* 新しいテスト：なし（sjs/test/step10 の case2 が、I7 で通るようになった）。
* I7 が流したもの（Windows 11、Node.js 26）：`sjs/test/run_all.sh`（違いなし。直す前は step10 の case2 が違った）、`--stack-size` を 900、1828、4000、7000 KB にした case2（どれも通る）、直す前と後の `CONMA_VM=wasm test/run_all.sh -j 4` と `CONMA_VM=wasm consh/test/run_all.sh -j 4`（違うステップも出力も同じ。一時ディレクトリの名前だけが違う）、`CONMA_VM=wasm conjs/test/run_all.sh`（違いなし）。
* 流していないもの：macOS のすべて（変わる行は Windows でしか動かない）。wasm 版の VM の test/ と consh/test で、I7 で多くのステップが違っていたのは、このブランチの外で、shima3/conma#58（i7/wasm-windows）が直した。
* 測ったもの：なし。

### m4/conjs-web（shima3/conma#56）

* ConJS のブラウザの見本を、vm/wasm/web から conjs/web に移した（ConSh の consh/js/web と揃える）。移したのは stars.html、stars.con.js、canvas_demo.con.js（移すだけのコミットを分けた。conjs/test の step14 と step15 の case2.con.js のシンボリックリンクも、そのコミットで conjs/web/stars.con.js を指すようにした）。canvas.html は、ConMa 版（vm/wasm/web に残る）と ConJS 版（conjs/web/canvas.html。`?conjs` はもう要らない）に分けた。build.sh も分け、conjs/web/build.sh が ConJS の 2 つを変換してモジュールにし、vm/wasm/web/build.sh は ConMa の canvas_demo.se だけを作る。pages.yml は conjs/web を docs/conjs/web にコピーするので、星ひろいと canvas の URL は変わらない（ConMa 版の canvas のページは、Pages から外した）。bin/lang-emit-check、.gitignore、.claude/launch.json（conjs-web、8768）、conjs/README の 2 つ、docs/conjs/index.md、docs/dev/CONJS.md、docs/HOST-ja.md を直し、docs/dev/PLATFORM-PLAN-ja.md に移した日付の一言を足した（ほかの所は、その段階の記録として残した）。
* 作り直すもの：ページを見るなら conjs/web/build.sh と vm/wasm/web/build.sh。前に vm/wasm/web/build.sh が作った vm/wasm/web の input.js、canvas_demo_conjs.module.se、stars_conjs.module.se は、もう .gitignore にないので、ワークツリーに残っていれば消してよい。
* 新しいテスト：なし。
* M4 が流したもの：`conjs/test/run_all.sh`（Rust 版）と `CONMA_VM=wasm conjs/test/run_all.sh`、`conjs/test/compare.sh`、`bin/lang-emit-check HEAD`（conjs/web の .con.js を読む）、`test/fast16/test.sh`。conjs/web/build.sh と vm/wasm/web/build.sh。2 つの canvas のページが動かすモジュールを、ページと同じ引数で、何も描かない canvas の上で Node.js から 60 コマ動かした（ConJS 版と ConMa 版）。
* 流していないもの：本当のブラウザ（ページの HTML は、読むモジュールの名前と起動の引数のほかは変えていない）、Jekyll（Pages の build は PR の CI で動く）、Windows。
* 測ったもの：なし。

### m2/step1-module（shima3/conma#55）

* consh/test/step1：後半（dump-sexp と、壊れた構文木の case2 から case5）は、ConMa 版の最適化版のモジュール（consh.module.se、`CONSH_CONMA_DATA` なら consh.data.module.se）を直に読む。`CONSH_IMPL` が rust か reference の consh は、そのモジュールを作らないので、新しいクローンや worktree で、ConMa 版を流す前に `CONSH_IMPL=rust` で流すと、22 の場合が「No such file or directory … consh.module.se」で違いになっていた（shima3/conma#51 のレビューで M2 が見つけた。alpha でも同じ）。`CONSH_IMPL` が conma でなければ、step1 が始めに ConMa 版の consh を 1 度動かして、モジュールを作る（ソースより古ければ作り直す）。
* 作り直すもの：なし。
* 新しいテスト：なし（step1 の中の準備だけ）。
* M2 が流したもの：空の `CONSH_MODULE_DIR` で、`CONMA_VM=rust` の step1 を、`CONSH_IMPL` が rust、reference、conma のとき、と rust に `CONSH_CONMA_DATA=1` を付けたときに流し、どれも違いなし（直す前は、rust で 22 の違い）。
* 流していないもの：step1 のほかのステップ、ほかの VM、Windows。
* 測ったもの：なし。

### m4/consh-build-missing（shima3/conma#51）

* consh/bin/consh：consh-rs（ConMa 版なら VM の conma）をまだ作っていないときも、古いとき（shima3/conma#48）と同じく、対話モードで端末から起動して `cargo` があれば、今ビルドするかを聞く（shima3/conma#50 で consh.ps1 に入れたことに揃えた）。文のパスは相対パスにし、ビルドの命令も書く。consh-rs がないとき：ビルドすれば Rust 版で動き、しないか失敗すれば、これまでどおり ConMa 版で動く（VM もなければ止める）。Windows の Git Bash（`cygpath` がある）では consh-rs を作れないので、作り方を出さず、聞かずに「running the ConMa version on the rust VM (consh-rs is not built on Windows)」とだけ出す（I7 のレビュー）。consh-rs を作らなかったときに VM もなければ、続けて VM をビルドするかを聞く（I7 のレビュー。前は consh-rs の作り方だけを出して止まり、Git Bash でクローンしたばかりのとき、VM のことを聞かなかった）。「much slower」の文は、VM を確かめた後に出す。ConMa 版の VM がないとき：ビルドすれば動き、しないか失敗すれば止める（終了ステータス 2。`CONMA_VM=python` も勧める）。VM がないことの確かめを、rlwrap を通す前に動かした。rlwrap を通してもう一度起動した consh は、consh-rs がないことを出さない（前は「ずっと遅い」の文も 2 回出ていた）。聞くこととビルドすることを `offer_build` にまとめ、古いときの `stale_build` も使う。前回のビルドが 1 秒未満なら「less than 1 s」と示す。cargo の標準入力を /dev/null にした（端末から読まないように）。
* 作り直すもの：なし（台本だけ）。
* 新しいテスト：なし（端末から打つ suite がない）。
* M4 が流したもの：端末なしで、consh-rs がない（ConMa 版で動く）、両方ない（止まる、2）、`CONSH_IMPL=conma` で VM がない（止まる、2。リポジトリの上と consh/include から）。`script` の疑似端末と rlwrap で、consh-rs がないときの n（ConMa 版で動く）、Enter（ビルドして Rust 版で動く）、ビルドが失敗する（cargo のエラーを出して ConMa 版で動く）。両方ないときの n と n（止まる、2）、n と Enter（VM をビルドして ConMa 版で動く）。`cygpath` を関数で偽って、VM があるとき（Windows の 1 行だけ）とないとき（VM を聞く道）。`CONMA_VM=python` で consh-rs がないとき。`cargo` が PATH にないとき（聞かない）。`CONSH_IMPL=conma` で VM がないときの n（止まる）と Enter（ビルドして動く）。`CONMA_VM=rust consh/test/run_all.sh -j 4`（ConMa 版）と `CONMA_VM=rust CONSH_IMPL=rust consh/test/run_all.sh -j 4`（Rust 版。既知の違いの examples の nbody.sh だけ）。
* 流していないもの：Windows の Git Bash（consh-rs を聞かないこと、VM がないときに聞くこと）。
* 測ったもの：なし。

### m4/consh-rs-dead-code（shima3/conma#53）

* consh/rust（consh-rs）：使っていないメソッド `Sexp::is_list`、`Sexp::id`（ast.rs）と `Value::is_string`（value.rs）を消した。cargo が dead_code の警告を出していた（shima3/conma#52 で VM の警告を消したのと同じ）。expand.rs の `is_list` は別の型のもので、使っているので残した。振る舞いは変わらない。
* 作り直すもの：consh-rs（consh/rust で `cargo build --release`）。
* 新しいテスト：なし。
* M4 が流したもの：consh/rust の `cargo build --release` と `cargo test --release`（警告なし）、`CONMA_VM=rust consh/test/run_all.sh -j 4`（ConMa 版）。M2 がレビューで `CONMA_VM=rust CONSH_IMPL=rust consh/test/run_all.sh -j 4`（Rust 版。既知の違いの examples の nbody.sh だけ）。
* 流していないもの：Windows（consh-rs を作れない）。

### i7/vm-dead-code（shima3/conma#52）

* vm/src/module.rs と step.rs：VM をビルドするたびに cargo が出していた警告（`field `0` is never read`、module.rs の `CLCont::V(Sexp)`）を消した。`__Closure__` と書いた LCont はどこでも使えないので、その節を持たない `CLCont::V` にした。振る舞いは変わらない（使えば、今までどおり `an LCont written as a __Closure__ value is not supported`）。
* 作り直すもの：Rust 版の VM、transpiled VM、wasm 版の VM（vm/src を変えた）。
* 新しいテスト：なし。
* I7 が流したもの（Windows 11）：`cargo build --release`（警告なし）、`vm/wasm/build.sh`、vm の `cargo test --release --lib`、`CONMA_VM=rust test/run_all.sh -j 6`、`CONMA_VM=rust CONSH_IMPL=conma consh/test/run_all.sh -j 6`。
* 流していないもの：transpiled VM、macOS のすべて、consh-rs（Windows では作れない。consh-rs の警告はこのブランチの外）。
* 測ったもの：なし。

### i7/consh-ps1-build-missing（shima3/conma#50）

* consh/bin/consh.ps1（consh.cmd からも動く）：VM の conma.exe をまだ作っていないとき（クローンしたばかりなど）、`is not built (cargo build --release in vm)` と出して止めていた。今は、パスをカレントディレクトリからの相対パスにし、ビルドの命令（`cargo build --release --manifest-path ..\..\vm\Cargo.toml`）も書き、対話モードで、標準入力と標準エラーがコンソールで `cargo` があれば、古いときと同じく今ビルドするかを聞く（Enter でビルドする）。ビルドすれば、そのまま ConSh を起動する。ビルドしないか、失敗すれば、止める（終了ステータス 2）。聞くこととビルドすることは、古いときと同じ関数（Read-BuildAnswer、Invoke-Build）にまとめた。cache-dir の場所は、聞くときに要るので、VM を確かめる前に決めるようにした。transpiled VM がないときは、作り方が違うので、聞かずに止める。
* 作り直すもの：なし（台本だけ）。
* 新しいテスト：なし（コンソールに打つ suite がない）。
* I7 が流したもの（Windows 11）：conma.exe を脇に移して、アプリの端末のタブで、consh/include から `..\bin\consh.cmd` を起動し、WriteConsoleInput で答えた。n（止まる。終了ステータス 2）、Enter（ビルドして、ConSh が動く）、vm/src/main.rs にエラーを入れて Enter（cargo のエラーを出して止まる）。パイプで、consh/include とリポジトリの上から起動した、作っていないときと古いときの文。
* 流していないもの：PowerShell 7（I7 にない）、Unix のすべて（consh.ps1 は Windows だけ）、consh/bin/consh（Bash 版は変えていない。作っていないときは、今までどおり止める）。
* 測ったもの：なし。

### i7/consh-ps1-stale-build（shima3/conma#49）

* consh/bin/consh.ps1（consh.cmd からも動く）：shima3/conma#48 で consh/bin/consh に入れたことを、consh.ps1 にも入れた。VM の conma.exe がソースより古いときの警告のパスを、カレントディレクトリがリポジトリの中なら相対パスにし（`..\..\vm\target\release\conma.exe`）、ビルドの命令（`cargo build --release --manifest-path ..\..\vm\Cargo.toml`。cmd でも PowerShell でも動く）も書く。対話モードで、標準入力と標準エラーがコンソールで `cargo` があれば、今ビルドするかを聞く（`[Y/n]`、Enter でビルドする）。ビルドした時間は、consh/bin/consh と同じ `build-seconds.conma`（bin/cache-dir の場所）に残して示す。cargo の出力は、失敗したときだけ出す。ビルドしないか、失敗すれば、古いまま使う。答えは Read-Host で読む（`[Console]::In.ReadLine()` は、Ctrl-C で戻らなかった）。consh-rs は Windows では作れないので、確かめない。
* 作り直すもの：なし（台本だけ）。
* 新しいテスト：なし（コンソールに打つ suite がない）。
* I7 が流したもの（Windows 11）：アプリの端末のタブで、consh/include から `..\bin\consh.cmd` を起動し、WriteConsoleInput で答えた。n（古いまま使う）、Enter（2 秒でビルドし、2 行だけ出す）、vm/src/main.rs にエラーを入れて Enter（cargo のエラーを出して、古いまま使う）。聞いているときの Ctrl-C（consh.ps1 が終わり、cmd がいつものように聞く）。パイプで、consh/include、リポジトリの上、リポジトリの外から起動した警告のパス（外なら絶対パス）。`CONMA_VM=rust CONSH_IMPL=conma consh/test/run_all.sh -j 6`（既知の違いだけ）。
* 流していないもの：PowerShell 7（I7 にない）、Unix のすべて（consh.ps1 は Windows だけ）。
* 測ったもの：なし。

### m4/consh-stale-build（shima3/conma#48）

* consh/bin/consh：consh-rs（ConMa 版なら VM の conma）がソースより古いときの警告を直した。対話モードで rlwrap を通すと、もう一度起動した consh も警告して、2 回出ていた。今は rlwrap を通す前の 1 回だけにした（ConMa 版の VM の警告も、通す前に確かめるように動かした）。パスは、カレントディレクトリがリポジトリの中なら、そこからの相対パスにし、ビルドの命令（`(cd consh/rust && cargo build --release)`）も書く。対話モードで、標準入力と標準エラーが端末で `cargo` があれば、今ビルドするかを聞く（`[Y/n]`、Enter でビルドする）。ビルドした時間を bin/cache-dir の場所の `build-seconds.consh-rs`（`build-seconds.conma`）に残し、次に聞くときに「前回は何秒」と示す。ビルドしないか、失敗すれば、古いまま使う。cargo の出力（コンパイラの警告など）は、失敗したときだけ出す（I7 のレビュー）。スクリプト、テスト、パイプでは聞かない。
* 作り直すもの：なし（台本だけ）。
* 新しいテスト：なし（端末から打つ suite がない）。
* M4 が流したもの：`script` の疑似端末と rlwrap で、n（古いまま使う）、Enter（約 2 秒でビルド）、2 回目（前回の時間を示す）、新しい consh-rs（聞かない）、`CONSH_IMPL=conma` で VM が古いとき。スクリプトをリポジトリの上、consh/rust、consh/include、リポジトリの外から起動した警告のパス。パイプの対話モード（聞かない）。ビルドが失敗するとき（cargo のエラーを出して、古いまま使う）。`CONMA_VM=rust consh/test/run_all.sh -j 4` と `CONMA_VM=rust CONSH_IMPL=conma consh/test/run_all.sh -j 4`。
* 流していないもの：Windows（consh.ps1 の警告は変えていない）。
* 測ったもの：なし。
