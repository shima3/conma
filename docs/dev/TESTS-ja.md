# テストの流し方と記録

この文書は、テストを全部流す手順、前からある既知の違い、テストを流した記録を 1 か所にまとめる。コミットメッセージには、その変更を何で確かめたかを短く書く。「全部流す」のすべてを流したときは、誰が流しても、その結果をここの「記録」に 1 行足して、ふつうのコミットにする（結果を書くためだけの空のコミットは作らない）。一部だけを流したとき（変更に関わるテストだけ、ある VM を除いたもの）は、コミットメッセージに書くだけにし、ここには記録しない。

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

## 申し送り

次に全部流す人への申し送り。作業した人が、作業のブランチの見出し（`### m4/chan` など）の下に書いて alpha にマージし、テストの担当者は、その見出しを含む alpha を自分のブランチ（`m2/test` など。機械と `test`）で全部流して確かめたら、その見出しの行を消して記録の行を足す（docs/dev/CONTRIBUTING.md の「Branches and Git settings」）。Full Bench（Full ConMa Bench、Full ConSh Bench、Full SJS Bench）も、ここで「ベンチマーク：」で始まる行で頼む。行には、3 つのどれか、測るコミット（7 桁。前後を比べるなら前のコミットも）、版（最適化版＋transpiled VM など）を書き、測る人は書かない。Full Bench は、ディレクトリのすべてのベンチマークを、書いた版だけで測る（すべての版で流す Full Test とは違う。版を書かなければ、表が比べるすべての版）。誰がいつ測るかはユーザーが決め、測る人は、その行を見ただけでは始めない。Full Bench でない測り直しは、変更した開発者が、ユーザーの指示を待たずに自分で測る（2026-11-18 から。それまでは、どの測り直しもここで頼んでいた）。その行は、測った人が、測ったブランチ（`m2/bench-1a2b3c4` など。機械と、測ったコミットの 7 桁）で消すまで残し（テストが済んでも消さない）、見出しは、その下の行がすべてなくなったときに消す（同じ文書の「Benchmarks」）。全部流している間に alpha に入ったブランチの見出しは、次に流すまで残す（記録には流したコミットだけを書く）。2026-11-16 までは、作業のブランチを作らずに alpha に書いていたので、見出しは alpha にした。

### m1/platform-plan

* docs/dev/PLATFORM-PLAN-ja.md（計画書）を足し、docs/dev/README.md の一覧に載せただけ。文書だけなので、流すものはない（計画書が sjs/ を指すので、main に直接入れず、alpha から入れる）。

### m1/platform-p1

* 段階 P1（docs/dev/PLATFORM-PLAN-ja.md の 8.1）：コア ConMa のプリミティブ `__Feature_available__` を足し（Python 版、Rust 版。wasm 版と transpiled VM も）、フロントエンドが `__JS_call__` と `__JS_value__` を使うモジュールに `(Platform "js")` を付ける（bin/resolver、include/resolver.se、reference/resolver.se。vm/frontend.module.se を作り直した）。新しいテストは test/fast13。
* Rust VM を作り直し、ConSh を動かしてモジュールを作り直し（ConSh のモジュールにも `(Platform "js")` が付く）、それから transpiled VM を `bench/build.sh` で作り直す。wasm 版は `consh/js/build.sh` で作り直す。
* 流したもの：全部流す（Python 版、Rust 版、wasm 版、transpiled VM の test/、Rust 版の consh/test の最適化版と参照版、wasm 版と transpiled VM の consh/test、consh/js/test、sjs/test）。既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17）。`CONMA_REFERENCE=1` の test/ と `CONSH_IMPL=rust` の consh/test は流していない。

### m2/platform-plan-notes

* docs/dev/PLATFORM-PLAN-ja.md に、M2 が読んで気付いたことと、6 節の決めることへの M2 の案を書いた（14b2c28）。その後、レビューは計画書と分けることにして（m2/platform-plan-review）、計画書を ddf6520 の文に戻し、レビューを docs/dev/REVIEW-ja.md の 2026-11-19 の節に移した。計画書には、そこへのリンクを 1 行足しただけ。文書だけなので、流すものはない（m1/platform-plan と同じく、main に直接入れず、alpha から入れる）。
* docs/dev/REVIEW-ja.md に、段階 P1（m1/platform-p1）のレビューの節（2026-11-19（2））を足した（m2/platform-p1-review）。文書だけなので、流すものはない。alpha から入れる。
* docs/dev/REVIEW-ja.md に、見本の言語の案（直接スタイルの JS）と段階 P2 のレビューの節（2026-11-19（3））を足した（m2/platform-p2-review）。文書だけなので、流すものはない。alpha から入れる。
* docs/dev/REVIEW-ja.md に、ConJS の決定（m1/conjs-decision）のレビューの節（2026-11-19（4））を足した（m2/platform-conjs-review）。文書だけなので、流すものはない。alpha から入れる。

### m1/platform-p2

* 段階 P2（docs/dev/PLATFORM-PLAN-ja.md の 8.2）：コア ConMa のプリミティブ `__Host_call__` を足した（Python 版、Rust 版。wasm 版と transpiled VM も）。どの VM にもあるテストのホストの関数（機能 `"test"`）、wasm 版のホストの JavaScript の関数（vm/wasm/conma.js の `hostFunctions`、`hostFeatures`。vm-node.js は `CONMA_HOST`）。フロントエンドが `(Platform "host")` を付ける（vm/frontend.module.se を作り直した）。新しいテストは test/fast14。
* Rust VM、wasm 版（`consh/js/build.sh`）、transpiled VM（`bench/build.sh`。ConSh のモジュールは変わらない）を作り直す。
* 流したもの：全部流す（Python 版、Rust 版、wasm 版、transpiled VM の test/、Rust 版の consh/test の最適化版と参照版、wasm 版と transpiled VM の consh/test、consh/js/test、sjs/test）。既知の違いだけ。`CONMA_REFERENCE=1` の test/ と `CONSH_IMPL=rust` の consh/test は流していない。ブラウザでは試していない。

### m1/conjs-decision

* 文書だけ（docs/dev/PLATFORM-PLAN-ja.md と docs/dev/REVIEW-ja.md）。流すものはない。

### m1/platform-p3-design

* 文書だけ（docs/dev/PLATFORM-PLAN-ja.md の 3.4 節、段階 P3 の設計の案）。流すものはない。
* M2 の ConJS のレビュー（docs/dev/REVIEW-ja.md の 2026-11-19（4））への答えを、計画書の 4.1 節と、そのレビューの「対応」に書いた。Promise は受け付けないと決めた。これも文書だけ。
* M2 の PR（shima3/conma#1）のコメントを受けて、3.4 節を直した：`test.later` の終わる順をどの VM でも同じにする決め方、`__Host_start_tracked__`、プラットフォームの印、`canvas.frame` の時刻。これも文書だけ。
* M2 の次のコメントを受けて、`test.later` の終わる順を、どの VM でも同じ仮の時計（締め切り = 今の時計 + 待つ数）で決めることにした。これも文書だけ。

### m1/platform-p3

* 段階 P3（docs/dev/PLATFORM-PLAN-ja.md の 8.3 節）：待つホストの呼び出し（`__Host_start__`、`__Host_start_tracked__`、`__Host_result__`、`test.later`）と、canvas（vm/wasm/host/canvas.js、include/platform/、vm/wasm/web/）。
* 新しいテスト：test/fast15 と test/fast16。どちらも test/run_all.sh に入る（fast*）。CONMA_VM=python、rust、wasm で通ることを確かめた。
* VM が変わった（vm/src/sched.rs、vm/src/prim/host.rs、vm/python/vproc_step.py、vm/wasm/conma.js）ので、test/ と consh/test を、Python 版、Rust 版、wasm 版の VM と transpiled VM で流す。transpiled VM は、作り直してから（17〜21 分）。
* M1 が流したもの（Apple M1）：test/ を Python 版（122 秒）、Rust 版（25 秒）、wasm 版（39 秒）の VM で（`-j 8`）。consh/test を Rust 版と wasm 版の VM で。違いは既知のもの（wasm 版の test/step40、consh/test の step8 と step17）と、wasm 版の test/step31 が `-j 8` で 1 回だけ違ったこと（シグナルのタイミング。1 つずつ 5 回流し直して、すべて通った）。流していないもの：transpiled VM（作り直していない）と、Python 版の VM の consh/test。
* 見本のページ（vm/wasm/web/canvas.html）は、vm/wasm/web/build.sh の後、ブラウザで見る。測り方は vm/wasm/web/measure.sh（9 秒ほど）。

### m1/platform-p3b

* 段階 P3b（docs/dev/PLATFORM-PLAN-ja.md の 8.4 節）：ConJS（conjs/。conjs/README-ja.md）。SJS の変換器と実行時の支え（sjs/include の sjs_parse.se、sjs_gen.se、sjs_rt.se、sjs_inspect.se）にも足した。
* 新しいテスト：conjs/test（`conjs/test/run_all.sh`。`CONMA_VM` なし、wasm、python）。「全部流す」に足した。
* sjs/include を変えたので、sjs/test と、SJS を使う consh/test（`(( ))`）と consh/js/test も流す。VM は変えていない。
* M1 が流したもの（Apple M1）：conjs/test を Rust 版（30 秒）、wasm 版（29 秒）、Python 版（1 分 30 秒）の VM で。sjs/test（2 分 32 秒）。consh/test を Rust 版（39 秒）と wasm 版（56 秒）の VM で（`-j 6`、`-j 3`）。consh/js/test（17 秒）。違いは既知のもの（wasm 版の consh/test の step8 と step17）だけ。PR shima3/conma#4 のレビューで sjs_parse.se を直した後に、conjs/test（3 つの VM）、sjs/test、Rust 版の consh/test を流し直し、すべて通った。流していないもの：test/（VM を変えていない）、ConSh の参照版と transpiled VM と Python 版の VM の consh/test。sjs/bench は測っていない（ベンチマークが動かすコードは変わらない。変えたのは、処理の値の種類の検査と、変換するときの名前の検査だけ）。

### m1/platform-p4

* 段階 P4（docs/dev/PLATFORM-PLAN-ja.md の 8.5 節）：言語キット（include/lang/）、bin/lang、bin/lang-test、bin/lang-emit-check、bin/lang-step-check。SJS（sjs/include）と ConJS を、キットの部品を使う形に書き直した。consh/include/consh_jsstmt.se は、呼ぶ名前だけが変わった。
* 新しいテスト：test/fast17（キットだけで作った電卓の言語。test/run_all.sh に入る）。
* sjs/include と consh/include が変わったので、sjs/test、conjs/test、consh/test、consh/js/test を流す。ConSh のモジュールが変わるので、transpiled VM は作り直してから consh/test を流す。VM は変えていない。
* M1 が流したもの（Apple M1）：`bin/lang-emit-check --steps 68795c9`（作るコードは同じ、変換のステップは +0.05%）、`bin/lang-step-check 68795c9`（83 のプログラムで出力は同じ。例外を投げる 40 のプログラムだけ、ステップが 1〜11 多い。受け入れた違いなので、この比べ方では終了ステータス 1 になる）。sjs/test（2 分 19 秒）、conjs/test を Rust 版（27 秒）、wasm 版（28 秒）、Python 版（1 分 30 秒）の VM で。consh/test を Rust 版（33 秒）と wasm 版（50 秒）の VM で。consh/js/test（16 秒）。test/ を Rust 版の VM で（22 秒）。test/fast17 を Python 版、Rust 版、wasm 版の VM で。違いは既知のもの（wasm 版の consh/test の step8 と step17）だけ。流していないもの：Python 版と wasm 版と transpiled VM の test/、ConSh の参照版、transpiled VM と Python 版の VM の consh/test。
* ベンチマーク：sjs/bench の実行は、作るコードと、実行時のステップ数（例外を投げない所）が変わらないので、測っていない。sjs/bench/consh の nbody.sh で変換の時間を前と後で測ることも、まだしていない（変換のステップ数は +0.05%）。

## 記録

新しいものを上に足す。「全部」は「全部流す」のすべてで、既知の違いのほかに違いがなかったことを表す。2026-11-15 までは一部だけを流したものも記録していたので、その行は「全部」でない。

| 日付 | コミット | 流した人 | 流したもの | 結果 |
|---|---|---|---|---|
| 2026-11-20 | m2/docs-pr-flow（文書だけ） | Claude（M2） | 流していない（文書だけ。docs/dev/CONTRIBUTING.md の Branches の 5） | プルリクエストで、作者のほかの人がレビューしてテストを流す決まりを、docs/dev/CONTRIBUTING.md の Branches の 6 に足し、説明の型 .github/pull_request_template.md を作った。役目ではなく、書いた機械を本文の初めに書く（テストと開発の担当は変わるため）。PR shima3/conma#7 の M4 のレビューを受けて、流す状態（ブランチの先頭。alpha と衝突するときは作者が先に alpha をマージ）、GitHub の承認が使えないことと「指摘なし」の合図、型を body.md に写すこと、開いた後に申し送りへ番号を足すこと、文書だけの PR を gh でマージした後の手順、申し送りに PR で流したものを短く書くことを足した。記録と申し送り、マージの後の Full Test は変えていない。確かめたこと：足したリンクとパス（docs/dev/TESTS-ja.md、.github/pull_request_template.md）があること、Liquid の開き括弧を増やしていないこと。Jekyll は動かしていない |
| 2026-11-19 | 449a0db | Claude（M4、m4/test） | Full Test と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。m1/sjs-top で consh/sjs から移った sjs/test も通った（`sjs/test/run_all.sh`）。申し送りの m1/sjs-top の、流していなかった組み合わせ（`CONMA_REFERENCE=1` の test/、`CONSH_IMPL=rust` の consh/test）も通った。申し送りの m1/sjs-bench-move（suite は変えたファイルを読まない）と、m2/bench-a471875、m1/note-sjs-path も含む。transpiled VM は、consh/include が変わったので、Rust VM の consh/test で ConSh のモジュールを作り直してから、449a0db で作り直した（11 分 24 秒、`transpiled code: used`）。Apple M4、rustc 1.99.1、Node.js v26.13.0、CPython 3.9.6 |
| 2026-11-19 | a471875 | Claude（M4、m4/test） | Full Test と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。申し送りの m2/lazy-rest と m1/seq-varlist-gc の、流していなかった組み合わせ（test/ と consh/sjs/test、Python 版の VM、`CONSH_IMPL=rust`）も通った。transpiled VM は、Rust VM の consh/test で ConSh のモジュールを作り直してから、a471875 で作り直した（10 分 41 秒、`transpiled code: used`）。Apple M4、rustc 1.99.1、Node.js v26.13.0、CPython 3.9.6 |
| 2026-11-19 | m2/docs-full-bench（文書だけ） | Claude（M2） | 流していない（文書だけ。docs/dev/CONTRIBUTING.md の Branches の 5） | Full Bench の定義を、表が比べるすべての版ではなく、頼んだ版で、ディレクトリのすべてのベンチマークを測るものに直した（docs/dev/CONTRIBUTING.md の Benchmarks と 3、この文書の申し送り）。すべての版で流す Full Test との違いと、版を書かない頼みはすべての版であることを書いた。リンクは足していない |
| 2026-11-19 | m1/docs-dev-off-pages（文書だけ） | Claude（M1） | 流していない（文書だけ。docs/dev/CONTRIBUTING.md の Branches の 5） | GitHub Pages に docs/dev と docs/consh/dev を出さないようにした（docs/_config.yml の `exclude`）。開発者の文書は GitHub の alpha で読む。docs/index.md と docs/consh/index.md の開発者向けの一覧を、新しい docs/dev/README.md と docs/consh/dev/README.md に移し（GitHub がフォルダの画面に出す）、index からは GitHub の alpha へのリンクにした。docs/dev と docs/consh/dev の文書の、ほかの文書への `.html` のリンク（22 か所、GitHub では切れていた）を `.md` にした。docs/dev/CONTRIBUTING.md と README.md にも書いた。確かめたこと：変えた 8 つのファイルのリンクの先がすべてあること、docs/ の Markdown がすべてどれかの一覧からリンクされていること、Liquid の開き括弧が増えていないこと。Jekyll は動かしていない（`exclude` が効くことは、main に入った後の Pages で見る） |
| 2026-11-19 | m4/docs-transpiled-cache（文書だけ） | Claude（M4） | 流していない（文書だけ。docs/dev/CONTRIBUTING.md の Branches の 5） | 変換版の VM を作る前に ConSh のモジュールを作り直しておくことの説明に、ほかの suite と並べて流すときも作り直し終えてから `bench/build.sh` を始めること、そうしないと出る警告、`transpiled code: used` の確かめ方を足した（442e41d の Full Test の 1 回目の作り直しで、古いモジュールを変換した）。確かめたこと：リンクは足していないこと、Liquid の開き括弧を増やしていないこと。Jekyll は動かしていない |
| 2026-11-19 | 442e41d | Claude（M4、m4/test） | Full Test と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。申し送りの m2/rest-read、m2/gc-single、m1/transpiled-size の、流していなかった組み合わせ（transpiled VM、wasm 版の VM の consh/test、`CONSH_IMPL=rust`、参照版、Python 版の VM）も通った。transpiled VM は 442e41d で、`CONMA2RUST_COMPACT` なしで作り直した（10 分 54 秒）。1 回目は ConSh を動かす前に `bench/build.sh` を動かしたので、キャッシュの古い consh.module.se を変換し、consh/test が変換したコードを使わなかった（警告の行の違い）。ConSh を動かしてモジュールを作り直してから作り直し、`transpiled code: used` を確かめた。Apple M4、rustc 1.99.1、Node.js v26.13.0、CPython 3.9.6 |
| 2026-11-18 | 4d396cb | Claude（M4、m4/test） | Full Test と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。申し送りの m1/transpiled-build のとおり、transpiled VM を 4d396cb で作り直した（`bench/build.sh` が 11 分 51 秒。ad3044d では 23 分）。`CONMA_TRANSPILED_REPORT` で `transpiled code: used` を確かめた。Apple M4、rustc 1.99.1、Node.js v26.13.0、CPython 3.9.6 |
| 2026-11-18 | deeff23 | Claude（M4、m4/test） | Full Test と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。申し送りの m1/bash-19-20 と m1/consh-start（`consh/bin/consh` が変わった）を含む。transpiled VM は ad3044d で作ったものを使い直した（ad3044d からの差分に vm、include、bin、consh/include、bench、test/transpiled の変更がない）。Apple M4、rustc 1.99.1、Node.js v26.13.0、CPython 3.9.6 |
| 2026-11-18 | m1/docs-index（文書だけ） | Claude（M1） | 流していない（文書だけ。docs/dev/CONTRIBUTING.md の Branches の 5） | docs/index.md に、どこからもリンクされていなかった docs/dev/ARRAY-PLAN-ja.md と docs/dev/LAZYLIST-PLAN-ja.md へのリンクを足し、ConSh の文書の一覧を新しい docs/consh/index.md に移した（同じくリンクされていなかった docs/consh/dev/COMPILE2-PLAN-ja.md も足した）（docs/index.md の「ConSh」の節は、それへのリンクと ConSh for Bash users へのリンクだけにし、一覧を 1 か所にした）。確かめたこと：2 つのファイルのリンクの先がすべてあること（`consh/web/` は pages.yml が作る）、docs/ の Markdown がすべて 2 つのどちらかからリンクされていること、Liquid の開き括弧がないこと。Jekyll は動かしていない |
| 2026-11-18 | ad3044d | Claude（M4、m4/test） | Full Test と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。前の 2 回にあった `CONSH_IMPL=rust` の consh/test の step2 の case20 の違いは、m1/case20-rust で消えた。申し送りの m1/compile2-argmax と m1/compile2-pump の、流していなかった組み合わせ（transpiled VM）も通った。transpiled VM は ad3044d で作り直したもの（23 分）。Apple M4、rustc 1.99.1、Node.js v26.13.0、CPython 3.9.6 |
| 2026-11-18 | m1/docs-bench-rule（文書だけ） | Claude（M1） | 流していない（文書だけ。docs/dev/CONTRIBUTING.md の Branches の 5） | docs/dev/CONTRIBUTING.md の Benchmarks の 3（Who measures）を、Full Bench でない測り直しは変更した開発者がユーザーの指示を待たずに自分で測り、Full ConMa Bench、Full ConSh Bench、Full SJS Bench だけを申し送りで頼む（3 つのどれか、測るコミットと前のコミット、版を書き、測る人は書かない）ように書き換えた。Branches の 2 と 4、Benchmarks の 2（作ってある transpiled VM を使える条件、作る時間を約 40 分に）と 4、このファイルの申し送りの説明も合わせた。確かめたこと：リンクは足していないこと、docs/ の Markdown に Liquid の開き括弧が増えていないこと。Jekyll は動かしていない |
| 2026-11-18 | m4/docs-contributing-page（文書だけ） | Claude（M4） | 流していない（文書だけ。docs/dev/CONTRIBUTING.md の Branches の 5） | docs/dev/CONTRIBUTING.md の頭に `title: Contributing` だけの front matter を足した。GitHub Pages の Jekyll は、front matter のない CONTRIBUTING.md をページにせず（jekyll-optional-front-matter が README、LICENSE、CONTRIBUTING などの名前を外す）、目次からの dev/CONTRIBUTING.html が 404 だった。確かめたこと：front matter のある Markdown はほかにないこと、リンクは変えていないこと、Liquid の開き括弧を増やしていないこと。Jekyll は動かしていない（push 後に公開されたページを見る） |
| 2026-11-18 | m4/docs-terms2（文書だけ） | Claude（M4） | 流していない（文書だけ。docs/dev/CONTRIBUTING.md の Branches の 5） | docs/dev/CONTRIBUTING.md の Benchmarks に、ディレクトリごとの呼び名 Full ConMa Bench（bench/）、Full ConSh Bench（consh/bench）、Full SJS Bench（consh/sjs/bench）を書き、Full Bench をその 3 つを合わせたものにした。確かめたこと：リンクは足していないこと、docs/ の Markdown に Liquid の開き括弧が増えていないこと。Jekyll は動かしていない |
| 2026-11-18 | m4/docs-terms（文書だけ） | Claude（M4） | 流していない（文書だけ。docs/dev/CONTRIBUTING.md の Branches の 5） | docs/dev/CONTRIBUTING.md に、全部の suite を流すことの呼び名 Full Test（Tests）と、3 つのディレクトリの全部のベンチマークを全部の版で測ることの呼び名 Full Bench（Benchmarks）を書いた。確かめたこと：リンクは足していないこと、docs/ の Markdown に Liquid の開き括弧が増えていないこと。Jekyll は動かしていない |
| 2026-11-18 | m1/pages-escape（文書だけ） | Claude（M1） | 流していない（文書だけ。docs/dev/CONTRIBUTING.md の Branches の 5） | GitHub Pages のワークフローで、Markdown を raw で囲む .github/pages-raw.py を、Liquid の 2 つの開き括弧を 1 つずつ Liquid の文字列の出力に置き換える .github/pages-escape.py に替えた（後ろに閉じ括弧のない開き括弧が、raw の終わりの閉じ括弧まで読まれ、raw が閉じずに失敗していた。このファイルの 1 つ上の行）。確かめたこと：docs/ の写しに流し、Liquid 4 の字句の区切りと同じ正規表現で、どの Markdown にも置き換えたもののほかのタグが残らず、置き換えたものを戻すと元と同じになること（難しい例も足した）。Jekyll は動かしていない |
| 2026-11-18 | m4/docs-ebnf（文書だけ） | Claude（M4） | 流していない（文書だけ。docs/dev/CONTRIBUTING.md の Branches の 5） | docs/consh/PLAN-ja.md の文法の `{{` を `{ {` にした（3 か所。EBNF の意味は同じ）。確かめたこと：docs/ の Markdown に `{{` と `{%` が残っていないこと（GitHub Pages の Jekyll が Liquid のタグと読まない）。リンクは変えていない。Jekyll は動かしていない |
| 2026-11-18 | 1ec4c7b | Claude（M4、m4/test） | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）と、前と同じ `CONSH_IMPL=rust` の consh/test の step2 の case20（`$(( ))` を Rust 版の ConSh が読めない）。申し送りの m1/compile2-capture と m1/pages-module-mode の、流していなかった組み合わせ（変換版の VM、wasm 版の VM の consh/test）も通った。変換版の VM は 1ec4c7b で作ったもの。Apple M4、rustc 1.99.1、Node.js v26.13.0、CPython 3.9.6 |
| 2026-11-18 | 2f1c8e1 | Claude（M4、m4/test） | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。ほかに、`CONSH_IMPL=rust` の consh/test の step2 が違う：m1/compile2-dq が足した case20 が `$(( 1 + 2 ))` を使い、Rust 版の ConSh は `(( ))` を読めないので 6 行目で止まる（examples と同じ理由。ConMa 版の誤りではない。作者に知らせる）。申し送りの m2/pages-liquid、m1/compile2-dq、m1/compile2-cmd の、流していなかった組み合わせ（変換版の VM（`bench/build.sh` で作り直した）、wasm 版の VM の consh/test）も通った。Apple M4、rustc 1.99.1、Node.js v26.13.0、CPython 3.9.6 |
| 2026-11-18 | 0748a95 | Claude（M4、m4/test） | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。申し送りの m1/jsstmt-cont の、流していなかった変換版の VM（`bench/build.sh` で作り直した）と wasm 版の VM の consh/test も通った。Apple M4、rustc 1.99.1、Node.js v26.13.0、CPython 3.9.6 |
| 2026-11-17 | 79006fa | Claude（M2、m2/test） | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。申し送りの m4/transpiled-clone2、m4/compile2-s1〜s5 の、流していなかった組み合わせ（wasm 版と作り直した変換版の VM の test/ と consh/test、consh/js/test、consh/sjs/test、参照版）も通った。m4/compile2-plan と m2/pages-actions（ワークフローは GitHub でしか試せない）も含む。m4/chan-head のベンチマークの依頼は残した |
| 2026-11-17 | ceef8ab | Claude（M2、m2/test） | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。申し送りの m4/consh-heavy2、m4/sjs-array-msg、m4/transpiled-clone、m4/chan-head、m2/consh-docs-move の、流していなかった組み合わせ（wasm 版と作り直した変換版の VM の test/ と consh/test、consh/js/test、consh/sjs/test、参照版）も通った。m4/js-step5 で直した consh/js/test の step5 も通った。m1/bench-sizes2、m1/sjs-bench-sizes、m4/types-t1、m4/types-hold、m4/array-a6 と、m2/bench-944c5df（m4/consh-heavy2 のベンチマーク）も含む。m4/chan-head のベンチマークの依頼は残した |
| 2026-11-17 | 3507874 | Claude（M2、m2/test） | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。申し送りの m4/vproc-wait の、流していなかった変換版の VM（`bench/build.sh` で作り直した）と wasm 版の VM の test/ と consh/test、consh/js/test、consh/sjs/test、参照版も通った。新しい test/step48 は 4 つの VM で通った。m1/bench-sizes、m4/bench-who、m1/bench-cd7d3a4 を含む |
| 2026-11-17 | b00beda | Claude（M2、m2/test） | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。m1/run-vm-path と m4/bench-names（どちらもテストは変わらない）を含む |
| 2026-11-17 | a8c7ab1 | Claude（M2、m2/test） | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。申し送りの m4/transpiled-inplace、m4/switch-counts、m4/dict-num-hash、m4/array、m1/static-cont の、流していなかった組み合わせ（Rust 版、wasm 版、作り直した変換版の VM の test/ と consh/test、consh/js/test、consh/sjs/test、参照版）も通った。m4/array-measure、m4/array-plan も含む |
| 2026-11-17 | 361a1bd | Claude（M2、m2/test） | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。申し送りの m1/lazy-fast-path の、流していなかった wasm 版と変換した VM（`bench/build.sh` で作り直した）の consh/test、consh/js/test、consh/sjs/test も通った。m1/bench-recursion も含む |
| 2026-11-17 | 6066d8b | Claude（M2、m2/test） | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。申し送りの m4/consh-heavy の、流していなかった wasm 版と変換した VM（`bench/build.sh` で作り直した）の consh/test、consh/js/test、consh/sjs/test も通った。P1（0c9d687）も含む |
| 2026-11-16 | 5d2c759 | Claude（M2、m2/test） | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。新しい consh/sjs/test の step2 の case8 も通った。申し送りの alpha（f90117c から 7c9911a まで）と m4/sjs-charat を確かめた |
| 2026-11-16 | 30789d6 | Claude | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM）、`CONSH_IMPL=rust` の consh/test | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ、Rust 版の ConSh の examples）。新しい test/step46 もどの VM でも通った。申し送り（2026-11-16）の作り直すものと流していないものを確かめた |
| 2026-11-16 | 5d057a8 | Claude | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ） |
| 2026-11-16 | f512194 | Claude | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ） |
| 2026-11-16 | 94da137 | Claude | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ） |
| 2026-11-15 | 30756a3 | Claude | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ） |
| 2026-11-15 | 7663e83 | Claude | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ） |
| 2026-11-15 | ee81b3e | Claude | 全部 | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17） |
| 2026-11-15 | 589674f | Claude | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ） |
| 2026-11-15 | bf6ded9 | Claude（変更した人） | 全部（変換版の VM を除く）と、jsc の consh/js/test の step7、serve.py で開いたブラウザ版の ConSh の `$(( 6 * 7 ))` | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17。wasm 版の VM を vm/wasm に移した。変換版の VM はテストの担当者に任せた） |
| 2026-11-15 | 3b3808b | Claude | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ） |
| 2026-11-15 | a2e5947 | Claude（変更した人） | test/（Python VM、Rust VM）、consh/test（最適化版、Rust VM）、test/step2 の case1、case5、case8 の test/vm_compare.sh | 違いなし（Python 版の VM を vm/python に移した。wasm 版と変換版の VM、ConSh の参照版はテストの担当者に任せた） |
| 2026-11-15 | 8972a31 | Claude | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ） |
| 2026-11-15 | 450f780 | Claude（変更した人） | test/ の fast7〜fast10（Python VM、Rust VM、wasm 版）、consh/test（最適化版、Rust VM）、consh/sjs/test の step9 | 違いなし（コメントと文書だけの変更。変えたファイルを読むテストだけ） |
| 2026-11-15 | 6544b04 | Claude | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ） |
| 2026-11-15 | c6cb2e5 | Claude | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ） |
| 2026-11-15 | 75aaa20 | Claude（変更した人） | consh/test（最適化版、参照版）と consh/sjs/test の step8、Rust VM | 違いなし（文書とコメントだけの変更） |
| 2026-11-15 | aa24fce | テストの担当者（b507491 に記録） | test/（Python VM、Rust VM、wasm 版）、consh/test（最適化版、参照版、wasm 版の step21 と examples）、consh/sjs/test、consh/js/test、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の step40、`-j 8` の step31、参照版の 5 つ）。はじめ wasm 版の fast1 が違ったが、wasm 版が名前の変更より古かったためで、作り直したら通った |
| 2026-11-15 | aa24fce | Claude（変更した人） | test/（Python VM、Rust VM）、consh/test（最適化版、参照版、Rust VM） | 違いなし（wasm 版と変換版の VM はテストの担当者に任せた） |
| 2026-11-15 | d0ddeb9 | Claude | 全部 | 既知の違いだけ |
