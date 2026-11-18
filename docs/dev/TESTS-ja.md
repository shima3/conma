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
* 変換版の VM には、`bench/build.sh` を動かしたときにキャッシュのディレクトリ（`bin/cache-dir`）にある ConSh のモジュールが組み込まれる。ConSh のテストを変換版の VM で流すときは、先に Rust VM で consh/test を流してモジュールを作っておき、それから `bench/build.sh` を動かす。

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
consh/sjs/test/run_all.sh
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

次に全部流す人への申し送り。作業した人が、作業のブランチの見出し（`### m4/chan` など）の下に書いて alpha にマージし、テストの担当者は、その見出しを含む alpha を自分のブランチ（`m2/test` など。機械と `test`）で全部流して確かめたら、その見出しの行を消して記録の行を足す（docs/dev/CONTRIBUTING.md の「Branches and Git settings」）。ベンチマークの測り直しも、ここで「ベンチマーク：」で始まる行で頼む。誰がいつ測るかはユーザーが決め、測る人は、その行を見ただけでは始めない。その行は、測った人が、測ったブランチ（`m2/bench-1a2b3c4` など。機械と、測ったコミットの 7 桁）で消すまで残し（テストが済んでも消さない）、見出しは、その下の行がすべてなくなったときに消す（同じ文書の「Benchmarks」）。全部流している間に alpha に入ったブランチの見出しは、次に流すまで残す（記録には流したコミットだけを書く）。2026-11-16 までは、作業のブランチを作らずに alpha に書いていたので、見出しは alpha にした。

### m1/compile2-dq

* ベンチマーク：consh/bench の `06` を、最適化版の変換版の VM で（前は alpha の 0748a95）。ステップは 1 回 1,297 → 1,041（−19.7%、Rust 版の VM）。

### m1/compile2-cmd

* ベンチマーク：consh/bench の `06`、`13`、`14`、`18` を、最適化版の変換版の VM で（前は alpha の e687929）。ステップは 1 回あたり −9.3%、−3.4%、−4.6%、−5.6%（Rust 版の VM）。m1/compile2-dq の `06` の依頼と、まとめて測ってよい。

### m1/compile2-capture

* ベンチマーク：consh/bench の `13`、`14` を、最適化版の変換版の VM で（前は alpha の 2f1c8e1）。ステップは 1 回あたり −5.9%、−4.1%（Rust 版の VM）。m1/compile2-dq と m1/compile2-cmd の依頼と、まとめて測ってよい。

## 記録

新しいものを上に足す。「全部」は「全部流す」のすべてで、既知の違いのほかに違いがなかったことを表す。2026-11-15 までは一部だけを流したものも記録していたので、その行は「全部」でない。

| 日付 | コミット | 流した人 | 流したもの | 結果 |
|---|---|---|---|---|
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
