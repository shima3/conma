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

次に全部流す人への申し送り。作業した人が、作業のブランチの見出し（`### m4/chan` など）の下に書いて alpha にマージし、テストの担当者は、その見出しを含む alpha を全部流して確かめたら、見出しを消して記録の行を足す（docs/dev/CONTRIBUTING.md の「Branches and Git settings」）。2026-11-16 までは、作業のブランチを作らずに alpha に書いていたので、見出しは alpha にした。

### alpha（2026-11-16、f90117c から 7c9911a まで、M4 の Claude）

SJS の文字列の添字の近道（consh/include/sjs_rt.se）と、文字列の位置の速い求め方（docs/dev/STRING-INDEX-PLAN-ja.md。Rust VM の文字列に「すべて ASCII か」と文字数を覚えさせる）を入れた。チャネルのプリミティブ（docs/dev/CHAN-PLAN-ja.md）までは、5d057a8 の全部の記録で確かめられている。

* **見つけた VM の不具合（前から）**：`__VProc_wait__` を継続なしで（関数の最後に）書いた VProc は、待つ相手が終わって起きたとき、終わったと扱われずに 1 歩進められ、`Null Operator: CChain empty, no LCont` のエラーになる（Python VM と Rust VM の両方。docs/dev/CHAN-PLAN-ja.md の C2）。step45 と fast11 は、待つ所に継続を付けて避けている。このメッセージが待っていた VProc で出たら、この不具合を疑う。
* **SJS の前からの違い**：`"héllo".charAt(-1)` が、JavaScript の `""` でなく `"o"` を返す（consh/sjs/README-ja.md）。テストのログには出ない。
* 7c9911a より後の alpha のコミット（測った機械の名前の修正、ブランチの決まり、この申し送り）は、文書だけ。この後の作業は、作業のブランチを作って alpha にマージする（docs/dev/CONTRIBUTING.md）。

## 記録

新しいものを上に足す。「全部」は「全部流す」のすべてで、既知の違いのほかに違いがなかったことを表す。2026-11-15 までは一部だけを流したものも記録していたので、その行は「全部」でない。

| 日付 | コミット | 流した人 | 流したもの | 結果 |
|---|---|---|---|---|
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
