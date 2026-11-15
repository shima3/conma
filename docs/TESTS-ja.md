# テストの流し方と記録

この文書は、テストを全部流す手順、前からある既知の違い、テストを流した記録を 1 か所にまとめる。コミットメッセージには、その変更を何で確かめたかを短く書き、テストの担当者が流した結果は、ここの「記録」に 1 行足して、ふつうのコミットにする（結果を書くためだけの空のコミットは作らない）。

## 流し方

どのテストも、違いがなければ、各テストの `== 名前` の行だけを出す。`-j N` は N 個ずつ並べて流す。

### 先に作るもの

| 作るもの | コマンド | 要るテスト |
|---|---|---|
| Rust VM | `cd vm && cargo build --release` | `CONMA_VM=rust`、ConSh のモジュール（`conma build`） |
| wasm 版の VM と web のファイル | `consh/js/build.sh` | `CONMA_VM=wasm`、consh/js/test |
| 変換版の VM | `bench/build.sh` | `CONMA_VM=transpiled` |
| ConSh の Rust 版（なくてもよい） | `cd consh/rust && cargo build --release` | `CONSH_IMPL=rust` |

* ブランチを切り替えたり pull したりすると、ソースの時刻が変わる。VM がソースより古いと `consh/bin/consh` が警告を出すので、作り直してから流す。
* wasm 版は、`vm/src` を変えたら `consh/js/build.sh` で作り直す。古いまま流すと、VM の出すメッセージが古く、ログと違う（2026-11-15 に、名前を変える前の wasm 版で test/fast1 が違った）。
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
* consh/test は、`CONSH_IMPL` がなければ ConMa 版の最適化版を試す（consh/test/common.sh）。

## 既知の違い

これらは前からある違いで、変更の誤りではない。これ以外の違いが出たら、誤りを疑う。

| VM や版 | テスト | 理由 |
|---|---|---|
| wasm 版 | test/step40 | ログはネイティブの VM のもので、JavaScript のある wasm 版では case1 の呼び出しが Stream を受け取る（docs/ROADMAP.md の Step 40） |
| wasm 版 | consh/test の step17 | JavaScript のない版の `js` のテストで、wasm 版には JavaScript があるので、メッセージと `$?` が違う（`JavaScript is disabled`、2） |
| wasm 版 | consh/test の step8 | VProc を切り替える時の細かい所による（docs/FAST-PLAN-ja.md の 3.3） |
| どの VM でも、機械が混んでいるとき | test/step31 | SIGINT を 2 回送るテストで、混んでいると 2 回が 1 回にまとまることがある。単独で流せば通る（`-j 8` の wasm 版で見られた） |
| 標準ライブラリの参照版（`CONMA_REFERENCE=1`） | test/step18 の case12、step20 の case15、step21 の case6、fast3、fast9 | util.se そのものに依るログ：エラーの位置が reference/util.se の行になる、数える時点のステップがずれる、印を付けた Body の数や大域の番号が違う（reference/README-ja.md） |

## 記録

新しいものを上に足す。「全部」は「全部流す」のすべてで、既知の違いのほかに違いがなかったことを表す。

| 日付 | コミット | 流した人 | 流したもの | 結果 |
|---|---|---|---|---|
| 2026-11-15 | c6cb2e5 | Claude | 全部と、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の test/step40 と consh/test の step8、step17、参照版の 5 つ） |
| 2026-11-15 | 75aaa20 | Claude（変更した人） | consh/test（最適化版、参照版）と consh/sjs/test の step8、Rust VM | 違いなし（文書とコメントだけの変更） |
| 2026-11-15 | aa24fce | テストの担当者（b507491 に記録） | test/（Python VM、Rust VM、wasm 版）、consh/test（最適化版、参照版、wasm 版の step21 と examples）、consh/sjs/test、consh/js/test、`CONMA_REFERENCE=1` の test/（Rust VM） | 既知の違いだけ（wasm 版の step40、`-j 8` の step31、参照版の 5 つ）。はじめ wasm 版の fast1 が違ったが、wasm 版が名前の変更より古かったためで、作り直したら通った |
| 2026-11-15 | aa24fce | Claude（変更した人） | test/（Python VM、Rust VM）、consh/test（最適化版、参照版、Rust VM） | 違いなし（wasm 版と変換版の VM はテストの担当者に任せた） |
| 2026-11-15 | d0ddeb9 | Claude | 全部 | 既知の違いだけ |
