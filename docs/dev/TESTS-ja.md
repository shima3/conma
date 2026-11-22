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
| wasm 版（ときどき） | consh/test の step12 の case8 | 疑似端末で Ctrl-C を押すテストで、Ctrl-C の後にプロンプトが 2 回出て、その後の出力が 1 つずつずれる（GitHub Actions の Ubuntu で 2 回、macOS で 1 回。2026-11-22）。ConSh の対話モードの競り合いで、前景の sleep が終わった後に VM が SIGINT に気づくと、見張りの VProc（consh/include/consh_repl.se の Repl_watch）は、実行中のコマンドがないので、プロンプトで押した Ctrl-C として、改行と新しいプロンプトを出し、読みかけの入力を捨てる。wasm 版（Node.js）では、SIGINT と子の終わりが別のイベントで来るので、順序が入れ替わることがある。遅れた SIGINT と、新しいプロンプトで押した Ctrl-C を見分ける方法を決めるまで、既知の違いにする |
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

### m4/consh-stale-build

* consh/bin/consh：consh-rs（ConMa 版なら VM の conma）がソースより古いときの警告を直した。対話モードで rlwrap を通すと、もう一度起動した consh も警告して、2 回出ていた。今は rlwrap を通す前の 1 回だけにした（ConMa 版の VM の警告も、通す前に確かめるように動かした）。パスは、カレントディレクトリがリポジトリの中なら、そこからの相対パスにし、ビルドの命令（`(cd consh/rust && cargo build --release)`）も書く。対話モードで、標準入力と標準エラーが端末で `cargo` があれば、今ビルドするかを聞く（`[Y/n]`、Enter でビルドする）。ビルドした時間を bin/cache-dir の場所の `build-seconds.consh-rs`（`build-seconds.conma`）に残し、次に聞くときに「前回は何秒」と示す。ビルドしないか、失敗すれば、古いまま使う。スクリプト、テスト、パイプでは聞かない。
* 作り直すもの：なし（台本だけ）。
* 新しいテスト：なし（端末から打つ suite がない）。
* M4 が流したもの：`script` の疑似端末と rlwrap で、n（古いまま使う）、Enter（約 2 秒でビルド）、2 回目（前回の時間を示す）、新しい consh-rs（聞かない）、`CONSH_IMPL=conma` で VM が古いとき。スクリプトをリポジトリの上、consh/rust、consh/include、リポジトリの外から起動した警告のパス。パイプの対話モード（聞かない）。`CONMA_VM=rust consh/test/run_all.sh -j 4` と `CONMA_VM=rust CONSH_IMPL=conma consh/test/run_all.sh -j 4`。
* 流していないもの：Windows（consh.ps1 の警告は変えていない）、ビルドが失敗するとき。
* 測ったもの：なし。

### i7/console-mode（shima3/conma#43）

* Windows の VM（vm/src/sys/windows.rs）の、コンソールを読むスレッドを直した。MSYS のプログラム（Git Bash の sleep、cat、grep など）は、動いている間、コンソールの入力モードの行入力、エコー、Ctrl-C の処理をオフにする（I7 で 0x1F7 が 0x2E8 になる）。ConSh がそれを Ctrl-C で止めると、元に戻らない。すると Enter が `\r` だけになり、行が終わらず、ConSh がそれから後の行を読まなくなっていた（shima3/conma#41 のレビューで見つけた）。コンソールを読むスレッドが、作られたとき（VM の起動時）の入力モードを覚え、読むたびに、違っていれば戻す。Unix は変わらない。M4 のレビューで挙がった、この直しの範囲の外のこと：戻すのは入力のモードだけで、出力のモード（ENABLE_VIRTUAL_TERMINAL_PROCESSING など）は戻さない。戻すのは読みを始める前だけなので、すでに始まった読みの間に変わったモードは、次の読みから戻る（標準入力を受け継ぐ子を起動する前に、VM は始まった読みを止めるので、ConSh のコマンドではこうならない）。背景のジョブ（`&`）がモードを変えて動いている間にプロンプトで読むと、そのモードも戻す（Unix のシェルは前景のジョブの後にだけ戻すが、背景のジョブは Unix でもコンソールから読まない）。
* 作り直すもの：Windows の Rust 版の VM と transpiled VM（vm/src を変えた）。
* 新しいテスト：なし（コンソールに打つ suite がない）。
* I7 が流したもの（Windows 11）：隠したコンソールで、WriteConsoleInput で行を打ち、GenerateConsoleCtrlEvent で Ctrl-C を送った。consh.cmd で、`sleep 1`、Ctrl-C で止めた `sleep 10`（動いている間は 0x2E8、その後は 0x1F7）、`echo`、プロンプトでの Ctrl-C、`sh -c 'exit 130'`、Ctrl-C、`echo`、`exit`。どの行も読んで答えた（直す前は、Ctrl-C の後の行を読まなかった）。vm の `cargo test --release --lib`（14 通る）、`CONMA_VM=rust test/run_all.sh -j 6` と `CONMA_VM=rust CONSH_IMPL=conma consh/test/run_all.sh -j 6`（既知の違いだけ）。
* 流していないもの：本当の端末での打鍵（この PR の前の shima3/conma#42 がまだ alpha にないので、端末の Ctrl-C で ConSh が終わる）、参照版の consh/test、macOS のすべて（変わるファイルはない）。
* 測ったもの：なし。

### i7/console-ctrl-c（shima3/conma#42）

* Windows の VM（vm/src/sys/windows.rs）の、コンソールを読むスレッドを直した。端末（Windows Terminal やエディタの端末。疑似コンソール）で対話モードの ConSh に Ctrl-C を打つと、ConSh が終了ステータス 130 で終わっていた。疑似コンソールの Ctrl-C は、読み込みを 0 バイトで、エラーなし（ERROR_OPERATION_ABORTED でなく）で止めるので、入力の終わりと見ていた（130 は、Ctrl-C で ConSh が設定した $?）。0 バイトのとき、Ctrl-C の数が増えたかを最大 100 ms（`CTRL_C_WAIT`）待って見て、増えていれば入力の終わりにしない。Windows がハンドラーを呼ぶのが 100 ms より遅れると（忙しい機械など）、前の症状（ConSh が 130 で終わる）がそのときだけ出る（M4 のレビュー）。Unix は変わらない。consh/README-ja.md と consh/bin/consh.cmd のコメントに、consh.cmd から起動して Ctrl-C を押すと、ConSh が終わったときに cmd が「バッチ ジョブを終了しますか」と聞くことを書いた（`.cmd` では避けられない）。
* 作り直すもの：Windows の Rust 版の VM と transpiled VM（vm/src を変えた）。
* 新しいテスト：なし（Ctrl-C を疑似コンソールに打つ suite がない）。
* I7 が流したもの（Windows 11）：このアプリの端末のタブで、直す前の consh.cmd に Ctrl-C を送ると「バッチ ジョブを終了しますか (Y/N)?」が出て終わること、直した VM では Ctrl-C のたびに新しいプロンプトを出して続くこと（2 回）。標準入力がファイルなら、終わりで終わること。vm の `cargo test --release --lib`（14 通る）。
* 流していないもの：Ctrl-Z と Enter（コンソールの入力の終わり）、macOS のすべて（変わるファイルはない）。
* 測ったもの：なし。

### i7/consh-cmd（shima3/conma#40）

* consh/bin/consh.cmd を足した：PowerShell の実行ポリシー（Windows の既定では台本を動かさない）によらず、cmd か PowerShell から consh.ps1 を起動する。PATH に `pwsh`（PowerShell 7）があればそれで、なければ Windows PowerShell 5.1 で動かす。`.gitattributes` に `*.cmd text eol=crlf` を足した（cmd は CRLF の台本を読む）。consh.ps1 は頭のコメントだけを変えた。Unix の suite は、どちらも使わない。
* 作り直すもの：なし。
* 新しいテスト：なし。
* I7 が流したもの（Windows 11）：cmd から対話モード（標準入力のファイルから `echo` とパイプ）とスクリプトのない `--dump`（使い方、exit 1）。PowerShell から、引数のあるスクリプト、ないスクリプト（exit 1）、別のカレントディレクトリからの起動。実行ポリシーが Restricted（`PSExecutionPolicyPreference` を消した）で、consh.ps1 は直接には動かず、consh.cmd からは動くこと。
* 流していないもの：`pwsh` の側（I7 に PowerShell 7 がない）、端末からの Ctrl-C、macOS のすべて（変わるファイルはない）。
* 測ったもの：なし。

### i7/consh-ps1（shima3/conma#39）

* consh/bin/consh.ps1 を足した：Windows の PowerShell から ConSh の ConMa 版を起動する台本（consh/README-ja.md の「実行」）。consh/bin/consh と同じ引数（`--python-parser` を除く）、`CONSH_IMPL`、`CONMA_VM`（`rust` か `transpiled`）、`CONSH_CONMA_DATA`、`CONSH_MODULE_DIR` を取り、モジュールを bin/cache-dir と同じ場所（同じチェックサム）に `conma build` で作る。Unix の suite は、この台本を使わない。
* consh/bin/consh は、Windows の Git Bash（cygpath がある）では、モジュールのファイル名から取るリポジトリの上のディレクトリを `pwd -W`（`C:/...` の形）で作り、置き換えを perl でする（M4 のレビュー）。それまでは `/c/...` の形で、VM が書く `C:/...` と一致せず、ファイル名が絶対パスのまま残っていた。一致するようになると、1 行のモジュール（7 MB）で置き換える所が多く、Git Bash の sed は数分かかった（perl は 1 秒かからない）。Unix は sed のままで変わらない。
* 作り直すもの：なし。
* 新しいテスト：なし（suite に PowerShell で流すものはない）。
* I7 が流したもの（Windows 11、Windows PowerShell 5.1）：台本の手で、対話モード（標準入力のファイルから `echo` とパイプ）、引数のあるスクリプト、`--dump`、`CONSH_IMPL=reference`、誤った `CONSH_IMPL`、引数のない `--dump`、ないスクリプト。モジュールを空のディレクトリに作り直したもの（約 25 秒）と、consh/bin/consh が作ったものを使うもの（約 3 秒）。チェックサムが bin/cache-dir と同じになること。consh/bin/consh が空のディレクトリに作ったモジュール（約 20 秒）が、consh.ps1 が作ったものとバイトまで同じになること。PowerShell 5.1 から空の引数と `"` を含む引数を渡すと、スクリプトの `$#` が Git Bash からと違うこと（consh.ps1 の頭のコメントと README に書いた）。
* 流していないもの：端末から打つ対話モード、`CONMA_VM=transpiled`、`CONSH_CONMA_DATA`、PowerShell 7、macOS のすべて（変わるファイルはない）。
* 測ったもの：なし。

### m4/late-sigint（shima3/conma#41）

* ConSh の対話モードで、端末の Ctrl-C が前景の外部コマンドとインタプリタの両方に届き、インタプリタが気づく前に外部コマンドが終わったとき、遅れた SIGINT をプロンプトの Ctrl-C と取り違えて、改行とプロンプトを余分に出し、次の入力を捨てていた（consh/test/step12 の case8 が `^C$ echo status $?` になり、後がずれる。wasm 版の VM で多く、2026-11-22 の GitHub Actions で Ubuntu と macOS に出た）。止めていないコマンドが `$?` 130 で終わったら、止めたときと同じく改行を出して "late" を真にし、コマンドの実行中でない SIGINT が "late" の間に来たら何もしないようにした（consh/include/consh_repl.se、consh/reference/consh_repl.se、consh/rust/src/repl.rs）。"late" は、次の行を入力したときにも偽に戻す。SIGINT がシェルに届かなかったとき（`kill -INT` を外部コマンドにだけ送った、`exit 130` など）は、そのすぐ後のプロンプトで行を入力する前の Ctrl-C を 1 回だけ無視し、`exit 130` の後にも改行が出る。
* 確かめたこと：step12 を、最適化版と参照版の Rust 版、wasm 版の VM、最適化版の Python 版の VM、ConSh の Rust 版で流して違いなし。consh/test を Rust 版の VM で 3 つの版について流して、既知の違いだけ。`CONMA_VM=wasm consh/test/run_all.sh -j 3` を 15 回流して、step12 の違いは 0 回（既知の step17 と step8 だけ）。ただし、直す前の alpha でも、この Mac（M4）では 15 回で 0 回で、この競合はここでは起きにくい。それで、疑似端末で順番を作って確かめた：`sh -c 'kill -INT $$'`（SIGINT で終わり、シェルには届かない）の後のプロンプトで、シェルのプロセスグループに SIGINT を送ると、直す前は改行とプロンプトを余分に出し、直した後は出さない（ConSh の最適化版と参照版を Rust 版と wasm 版の VM で、最適化版を Python 版の VM で、ConSh の Rust 版）。`sleep 10` を Ctrl-C で止めた後のプロンプトで、もう一度 Ctrl-C を押すと読みかけの行を捨てることも、同じ版で確かめた（Python 版の VM を除く）。
* CI（PR #33、`.github/workflows/ci.yml`）で wasm 版の consh/test の step12 を既知の違いにしたのと、既知の違いの表の「wasm 版（ときどき）| consh/test の step12 の case8」の行は、まだ alpha にない（PR #33 は main に向けて開いている）。PR #33 が入ったら、両方を消す。
* 作り直すもの：ConSh の Rust 版（consh/rust で `cargo build --release`）。
* 新しいテスト：なし。
* 測ったもの：なし。
