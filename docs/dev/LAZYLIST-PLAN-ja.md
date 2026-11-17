# LazyList の取り出しを VM に組み込む計画

LazyList の要素を取り出す手順（include/util.se の `__LazyList_uncons__`、`__LazyList_force__`、`__LazyList_force_claim__`）のうち、評価済みのセルを読む所を、拡張 ConMa の機能 `"lazylist"` のプリミティブにする計画である（2026-11-17）。[docs/dev/ARRAY-PLAN-ja.md](ARRAY-PLAN-ja.md)、[CHAN-PLAN-ja.md](CHAN-PLAN-ja.md) と同じく、拡張 ConMa の機能（docs/PRIMITIVES.md の 16）を 1 つ足す形にする。ConSh を速くする計画（[docs/consh/dev/COMPILE2-PLAN-ja.md](../consh/dev/COMPILE2-PLAN-ja.md)）の S6 として、ユーザーと決めた（2026-11-17）。

## 1. 目的と、測ったこと

### 1.1 今の取り出しの手間（2026-11-17、Rust 版の VM の `CONMA_PROFILE`）

LazyList は Ref で、値はサンク、null（評価済みの空）、LList のセル（先頭の要素と、残りの LazyList）、評価中か持っている VProc のどれかである（docs/PRIMITIVES.md の 14）。プリミティブは `__LazyList_claim__`、`__LazyList_hold__`、`__LazyList_publish__` の 3 つで、取り出す手順は util.se に ConMa で書いてある。`__LazyList_uncons__` は `__LazyList_force__` を、それは `__LazyList_force_claim__` を呼び、そこで `__LazyList_claim__` がサンクかセルかを返す。サンクなら、それを適用し、返った LazyList も同じ手順で評価して、セルを `__LazyList_publish__` で書き戻す。

サンクで 1 つずつ作る 10 万要素の LazyList を、`__LazyList_uncons__` で先頭からたどる小さいプログラムで、1 要素あたり：

| 取り出し方 | util.se の手順のステップ | 内訳 |
|---|---|---|
| サンクを評価して取り出す（初めて読む） | 48 | `__LazyList_force_claim__` 27（2 回。サンクが返した LazyList の評価を含む）、`__LazyList_uncons__` 11、`__LazyList_force__` 10 |
| 評価済みのセルを取り出す（2 回目から） | 25 | `__LazyList_uncons__` 11、`__LazyList_force_claim__` 9、`__LazyList_force__` 5 |

### 1.2 ConSh での割合（docs/consh/dev/COMPILE2-PLAN-ja.md の 1.1、S5 の後）

* `02`（`for i in $(seq ...)`）の 1 要素 約 234 ステップのうち、LazyList の手順が約 70（`__LazyList_force_claim__` 32、`__LazyList_uncons__` 17、`__LazyList_force__` 13、`__LazyList_cons__` 5 など）、外部コマンドの出力から行を切り出すのが 37（`Subst_lines_next`）。要素はどれも初めて読む（サンクを評価する）。
* `14`（`for j in ${xs}`）は、評価済みの LazyList を何度もたどるが、1 回の繰り返しの手間が大きく、LazyList の手順の割合は 1% ほど。
* `18`（関数どうしのパイプ）は、チャネルから読む LazyList で、要素はどれも初めて読む。

### 1.3 見込み

* 評価済みのセルを読むのは、プリミティブ 1 回（と、それを呼ぶ util.se の関数）で済む：25 → 3 前後。
* サンクの評価は、ConMa のクロージャ（サンク）を適用するので、VM の中だけでは済まない。サンクが返した LazyList の評価（たいてい評価済みのセル）が軽くなるだけで、48 → 40 前後。
* そのため、ConSh の `02` では約 4% の見込みで、COMPILE2-PLAN の S6 で期待した分（`02` の約半分を占める `$( )` の値）には遠い。`02` の要素の手間の大半は、サンクを 1 つずつ評価することと、行を 1 つずつ切り出すことにある。それを減らすのは、5 の L4（行の切り出しのプリミティブ）か、COMPILE2-PLAN の S6 の元の案（読んだ塊の行をまとめて評価済みのセルにする。取り消しの見え方が少し変わる）である。
* 評価済みの LazyList をたどることの多い ConMa のプログラム（同じ値を何度も読む ConSh のスクリプト、`__LazyList_lines__` の結果を何度も見るものなど）には、よく効く。

## 2. 方針

* **拡張 ConMa の機能 `"lazylist"` にする**。コア ConMa（docs/SPEC.md、docs/PRIMITIVES.md の 1〜15）は変えない。
* **評価済みのときだけ VM の中で済ませ、そうでなければ ConMa の手順に任せる**。プリミティブは、LazyList が評価済み（セルか null）なら結果を返し、そうでなければ（サンクか VProc）、引数で受け取った関数（今の util.se の手順）を、プリミティブの継続で適用する。claim や publish の意味（docs/PRIMITIVES.md の 14）は変えない。
* **代わりの定義は「いつも任せる」**：コア ConMa の VM（Python 版）と bin/to-core のために、include/core/lazylist.se に、同じ名前の関数として、いつも引数の関数を適用する（評価済みでも ConMa の手順を通る）定義を書く。結果は同じで、ステップの数だけが違う（ほかの機能の代わりの定義と同じ）。
* **util.se の手順は、プリミティブを使う形に書き換える**。`__LazyList_uncons__`、`__LazyList_force__`、`__LazyList_is_empty__` が、先にプリミティブで評価済みかを見る。util.se を使うすべてのモジュール（フロントエンド、ConSh、SJS の変換器など）が `(Uses "lazylist")` になる。
* **変換した VM でも、プリミティブを直接呼ぶ**（`d_lazylist_*`。融合した鎖の中で、評価済みなら Rust の中で値を取り出す）。

## 3. 形（案）

| プリミティブ | 結果 | 評価済みでないとき |
|---|---|---|
| `__LazyList_cell__ onLater lazyList ,(cell)` | セル（LList）か null | `onLater` を引数なしで、このプリミティブの継続で適用する |
| `__LazyList_uncons_now__ onLater onEmpty lazyList ,(head tail)` | 先頭の要素と残りの LazyList。null なら `onEmpty` を引数なしで適用する | `onLater` を引数なしで、このプリミティブの継続で適用する |

* util.se（案）：`__LazyList_force__ lazyList` は `__LazyList_cell__ (,() __LazyList_force_claim__ lazyList) lazyList`、`__LazyList_uncons__ onEmpty lazyList` は `__LazyList_uncons_now__ (,() 今の手順) onEmpty lazyList`。
* LazyList でない値（Ref でない、Ref の値が LazyList の値でない）は、今の `__LazyList_claim__` と同じく、`onLater` の先の手順がエラーにする（プリミティブは判断せず、`onLater` を適用する）。
* 代わりの定義：`(define __LazyList_cell__ ,(onLater lazyList) onLater)`、`(define __LazyList_uncons_now__ ,(onLater onEmpty lazyList) onLater)`。
* 2 つ目は、1 つ目と `__is_null__`、`__LList_uncons__` で書けるが、`__LazyList_uncons__` は最もよく呼ばれる（ConSh の for 文、`__LazyList_lines__` を読む所）ので、1 回で済む形も持つ。

## 4. 決めること

1. **機能の名前とプリミティブ**：`"lazylist"` と、3 の 2 つ（`__LazyList_cell__`、`__LazyList_uncons_now__`）でよいか。
2. **評価済みでないときの形**：引数の関数に任せる（案）か、プリミティブがサンクの評価まで受け持つ（VM が claim し、サンクを適用し、その後の publish を VM の中の継続でする）か。後者はサンクの評価も軽くなる（48 → 15 前後の見込み）が、VM の中で ConMa の継続を作ることになり、作りが大きく変わる（評価を抜けたかの判断、kill との関係。docs/PRIMITIVES.md の 14 と `__VProc_kill__`）。
3. **行の切り出し（5 の L4）を、この計画に入れるか**：外部コマンドの出力の塊から、そろった行を切り出すプリミティブ（たとえば `__String_lines_from__ text start ,(lines next)`）。ConSh の `02` の 37 ステップ／行と、`__LazyList_lines__` を使う ConMa のプログラムに効く。
4. **COMPILE2-PLAN の S6 の元の案（行をまとめて評価済みのセルにする）を、この後で入れるか**：取り消し（`Subst_finish`）の後も、読み終えた塊の行が要素として残る、という見え方の違いを受け入れるか。

## 5. 作り方（段階、案）

各段階は、そこまでのテストが通り、test/ が、その段階に関わる VM で通ることで終わりとする（全部を流すのは、別の担当者がする。docs/dev/TESTS-ja.md）。

| 段階 | 内容 | 確かめ方 |
|---|---|---|
| L1 | 代わりの定義：include/core/lazylist.se に、3 の 2 つを、いつも `onLater` を適用する関数として書く。util.se を、それを使う形に書き換える | test/ の Python 版と Rust 版の VM（bin/to-core で直したモジュールで）。test/step に、評価済み、サンク、空、評価中（2 つの VProc から読む）、`__LazyList_hold__` の LazyList を取り出すケースを足す |
| L2 | Rust 版の VM：prim/lazylist.rs と prim/mod.rs の表、module.rs の `EXT_FEATURES` に `"lazylist"`。wasm 版を作り直す | L1 のテストを Rust 版と wasm 版の VM で。1.1 のプログラムのステップ |
| L3 | フロントエンド（bin/resolver の `EXT_PRIMITIVES`、include/resolver.se の `__Resolver_features__`）が `(Uses "lazylist")` を書く。vm/frontend.module.se を bin/update-frontend で作り直す。変換した VM：conma2rust.py の `CHAIN` と `DIRECT`、transpiled.rs の `SAFE_PRIMS` と `d_lazylist_*` | test/fast（フロントエンドが書いた `(Uses ...)`、`conma build` が同じモジュールを作る）、test/transpiled に case を足す |
| L4 | （4 の 3 で決めれば）行の切り出しのプリミティブと、それを使う util.se の `__LazyList_lines__` と ConSh の `Subst_lines_*` | consh/test、test/ の `__LazyList_lines__` のテスト |
| L5 | 測って書く：1.1 のプログラム、consh/bench の `02`、`14`、`18`（ステップと、変換した VM の時間）。docs/PRIMITIVES.md の 16、vm/README.md、docs/consh/dev/COMPILE2-PLAN-ja.md の S6 | 表にする |

## 6. 今の状態

計画を書いた（2026-11-17）。4 を決めてから L1 を作る。
