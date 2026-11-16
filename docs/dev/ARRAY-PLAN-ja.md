# 配列のプリミティブの計画

添字で O(1) に読み書きできる配列を、拡張 ConMa の機能 `"array"` として足す計画である（2026-11-17）。docs/dev/CHAN-PLAN-ja.md、BITWISE-PLAN-ja.md と同じく、拡張 ConMa の機能（docs/PRIMITIVES.md の 16）を 1 つ足す形にする。コア ConMa には足さず、代わりの定義（include/core/array.se）は Dict で書く。Rust 版の VM では新しい値（案 1）にする。

## 1. 目的と、測ったこと

### 1.1 今、値を並べて持つもの

| 値 | できること | 添字で読み書き |
|---|---|---|
| LList | 先頭に足す、先頭を取り出す、つなぐ、長さ（O(n)） | できない（先頭からたどる） |
| MSeq | 空か、先頭を取り出す、先頭を書き換える、末尾に足す | できない。Rust 版の VM の中身は `Vec` と開始位置だが、`__MSeq_uncons__` の残りは別の MSeq（書き換えは、ほかと分けてから） |
| Dict | キーで引く、置く、消す | キーを `"0"`、`"1"`、…にすれば、代わりになる（引くたびにハッシュ） |
| LazyList | 先頭から順に読む | できない |

### 1.2 SJS の配列（2026-11-17、Rust 版の VM の `CONMA_PROFILE`、Apple M4）

SJS の配列は Dict で、`"items"` に添字の文字列から値への Dict、`"length"` に長さの Ref を持つ（consh/include/sjs_rt.se の「配列」）。1 万要素の配列を `push` で作り、10 万回読み書きするプログラムを、読み書きしないものと比べた（consh/sjs/sjsc.se で変換したモジュール）：

| 操作 | 1 回のステップ | 中身 |
|---|---|---|
| `a[i]` を読む | 約 37 | `Sjs_get_index` 19、`Sjs_need` 11 など |
| `a[i] = v` | 約 28 | |
| `a.push(v)` | 約 400 | 添字を文字列にする `Sjs_num_str` 128、`Sjs_get_member` 78、`Sjs_array_method` 50 など |

配列のプリミティブなら、読み書きは数ステップ、`push` は 1 ステップの手間になる見込みである（SJS の側の、値の種類を確かめる分は残る）。ただし、SJS の配列を配列の値で表すのは、この計画の後の段（A6）で、別に決める（2 の 5）。

### 1.3 見込み

* **ConMa のプログラム**：添字で読み書きする列を、Dict（数の文字列や数のキー）で書いていたものが、配列で書ける。Rust 版の VM では、どちらも 1 ステップのプリミティブなので、ステップは変わらず、プリミティブの中のハッシュと比べる分だけ速くなる。差が大きいのは、数を文字列にしてキーにしていた所（SJS の `Sjs_num_str`）。
* **変換した VM**：融合した鎖の中で、添字が `f64` の読み書きをその場で書ける（Dict の引きより軽い）。
* **Python 版の VM**：代わりの定義の Dict で動く。速さは今の Dict と同じ。

## 2. 方針

* **拡張 ConMa に足す**：コア ConMa（docs/SPEC.md、docs/PRIMITIVES.md の 1〜15）は変えない。機能 `"array"` の代わりの定義（include/core/array.se）を、Dict を使うコア ConMa の関数として書く。Python 版の VM は、bin/to-core がそれを入れたモジュールを動かす。
* **Rust 版の VM では新しい値（案 1）**：`Value::Array(Rc<ArrayObj>)`。`ArrayObj` は、番号（トレースと同一性）と `RefCell<Vec<Value>>` を持つ。vm/src/prim に作れば、wasm 版の VM にも入り、変換した VM の生成コードからも呼べる。
* **VM によって違う所を、仕様として決めて書く**：Rust 版、wasm 版、変換した VM では、配列の `__type_name__` は `"Array"` で、Dict のプリミティブは配列にエラーになる。代わりの定義では、配列は Dict で、`__type_name__` は `"Dict"` になり、Dict のプリミティブも通る。そこで、docs/PRIMITIVES.md に「配列には配列のプリミティブだけを使う。それ以外の結果は VM によって違う」と書く。チャネル（`"chan"`）は Dict と Ref のままにしたので、これは、型の違いが見える初めての機能になる。
* **待たない、VProc を切り替えない**：どのプリミティブも、被演算子を読んで結果を返すか、onError を呼ぶだけ。
* **SJS で使うかは後で決める**：配列の機能を作って測ってから、SJS の配列を配列の値で表すか（穴、数でない添字、`length` への代入を、どう Dict に戻すか）を決める（A6）。

## 3. 形（案）

| プリミティブ | 結果 | エラー（onError を呼ぶ） |
|---|---|---|
| `__Array_new__ ,(array)` | 長さ 0 の配列 | なし |
| `__Array_length__ array ,(n)` | 長さ（Number） | なし（配列でなければ VProc を止める） |
| `__Array_get__ onError array i ,(value)` | 添字 i の値 | i が 0 以上、長さ未満の整数でない |
| `__Array_set__ onError array i value ,()` | 添字 i を書き換える | 同じ（長さは変えない） |
| `__Array_push__ array value ,()` | 末尾に足す | なし |
| `__Array_pop__ onError array ,(value)` | 末尾を取り除いて返す | 長さ 0 |

* **添字の確かめ**：i が Number でないとき、整数でないとき、範囲の外のとき、onError をメッセージ（`"__Array_get__: index out of range"` など）で呼ぶ。代わりの定義も同じメッセージにする（test/ で、両方の結果を比べる）。
* **配列でない値**：`__Array_length__` と `__Array_push__` は VProc を止める。onError を取るものは onError を呼ぶ（MSeq のプリミティブと同じ）。代わりの定義では、中で使う Dict のプリミティブのエラーになり、名前が違う（チャネルの C1、BITWISE-PLAN の W1 と同じ）。
* **代わりの定義の表し方**：Dict に、数のキー `0`〜`n-1`（Number。文字列のキー `"0"` とは別のキー）で要素を、文字列のキー `"length"` で長さを置く。Dict は変えられて、参照で渡るので、配列と同じ意味になる。Python 版と Rust 版の VM で、Dict が数のキー `0` と文字列のキー `"0"` を別のキーとして持ち、`0.0` で引けば `0` が見つかることを確かめた（2026-11-17）。添字の `-0` は `0` として扱う（代わりの定義は、引く前に `0` に直す）。
* **同一性**：配列は、Dict と同じく、作ったものごとに別の値。Dict のキーにすると、同一性で比べる（`DictKey::Obj`）。代わりの定義でも、Dict なので同じ。
* **トレースの表示**：`--trace` は Python 版の VM と比べるためのもので、配列は Python 版にはないので、形は自由に決める（たとえば `(Array 番号 長さ)`）。
* **足さないもの（案）**：長さを指定して作る、範囲を写す、LList との変換。必要になったら、ConMa の関数で書くか、後で足す。

## 4. 決めたこと（2026-11-17、ユーザーと。どれも案のとおり）

1. **プリミティブの組**：3 の 6 つ（`__Array_pop__` を入れ、長さを指定して作るものは入れない）。
2. **範囲の外への書き込み**：`__Array_set__` は長さ未満だけ。足すのは `__Array_push__`。
3. **名前**：`__Array_*__` と機能の名前 `"array"`。
4. **型の違いの扱い**：2 の「配列には配列のプリミティブだけを使う」を仕様として書く（docs/PRIMITIVES.md の 16）。
5. **SJS**：A6 として、A5 の後に別に決める。

## 5. 作り方（段階）

各段階は、そこまでのテストが通り、test/ が、その段階に関わる VM で通ることで終わりとする（全部を流すのは、別の担当者がする。docs/dev/TESTS-ja.md）。

| 段階 | 内容 | 確かめ方 |
|---|---|---|
| A1 | 代わりの定義：include/core/array.se に、3 のプリミティブを同じ名前のコア ConMa の関数として書く | test/step47：作る、足す、読む、書く、取り除く、長さ、範囲の外と整数でない添字と数でない添字の onError、配列を 2 か所から持って書き換える、Dict のキーにする。Python 版と Rust 版の VM で同じログ |
| A2 | Rust 版の VM：value.rs の `Value::Array` と `ArrayObj`、`value_type_name`、`DictKey` の同一性、gc.rs（中の値をたどる。配列は循環を作れるので、登録する）、trace.rs の表示、prim/array.rs と prim/mod.rs の表、module.rs の `EXT_FEATURES` に `"array"`。wasm 版を作り直す | A1 のテストを `(Uses "array")` のモジュールで、Rust 版と wasm 版の VM で。代わりの定義と同じ出力（`__type_name__` を除く）。循環する配列を作って捨てるテスト（`CONMA_GC_STATS`） |
| A3 | bin/to-core が `"array"` を代わりの定義に直す。フロントエンド（bin/resolver の `EXT_PRIMITIVES` と include/resolver.se の `__Resolver_features__`）が `(Uses "array")` を書く。vm/frontend.module.se を bin/update-frontend で作り直す | test/fast12（test/fast11 と同じ形：フロントエンドが書いた `(Uses ...)`、`conma build` が同じモジュールを作る、bin/to-core で直したモジュールも同じログ） |
| A4 | 変換した VM：6 つを conma2rust.py の `CHAIN` と transpiled.rs の `SAFE_PRIMS` に入れる。融合した鎖では、添字が `f64` の `__Array_get__` と `__Array_set__` をその場で書く（範囲の外は、その Body に戻して VM に任せる） | test/transpiled に case3（A1 のテストを順に）。bench/build.sh で作り直した VM で |
| A5 | 測って書く：配列と、数のキーの Dict で同じ計算をする小さいプログラム（添字の読み書き、push）を、Rust 版、変換した、wasm 版、Python 版の VM で。docs/PRIMITIVES.md の 16 に機能 `"array"`（型の違いを含む）、docs/dev/ROADMAP.md に Step 47、vm/README.md | 表にする |
| A6 | （A5 の後に決める）SJS の配列を配列の値で表す。数の添字の読み書き、`push`、`pop`、`length` を配列で行い、穴や数でない添字などは今の Dict に戻す。consh/sjs/test、consh/sjs/bench | 1.2 のプログラムと consh/sjs/bench の nbody で測る |

## 6. 今の状態

計画を書き、4 を決めた（2026-11-17）。

* **A1（作った、2026-11-17）**：include/core/array.se に、6 つを同じ名前のコア ConMa の関数として書いた（使う関数の名前は `Array_core_` で始める）。onError を取る 3 つは、配列（`"length"` のある Dict）でない値なら onError を `"名前: not an Array"` で呼ぶ（プリミティブと同じ）。test/step47 の 5 つのケース（作る、足す、読む、書く、取り除く、長さ。添字の誤り：-1、長さ、0.5、文字列、無限大、NaN、-0。配列でない値：文字列、Dict、null。同じ配列を 2 か所から持つ、配列の中の配列、自分を含む配列、null と関数の要素、Dict のキー。1 万個）が、Python 版、Rust 版、wasm 版の VM で同じログになる。無限大と NaN は「整数でない」になる。
* **A2（作った、2026-11-17）**：Rust 版の VM（value.rs の `Value::Array` と `ArrayObj`、`value_type_name` と `type_name` の `"Array"`、同一性、gc.rs の `Node::Array`、trace.rs の `(__Array__ "番号")`、prim/array.rs、prim/mod.rs の表、module.rs の `EXT_FEATURES`）。wasm 版の VM と consh/js/web を consh/js/build.sh で作り直した。自分と互いを含む配列を 40 万個作って捨てるプログラムが、最大 8.5 MB で終わる（`CONMA_GC_STATS` で、配列が集められている）。
* **A3（作った、2026-11-17）**：bin/to-core が `"array"` を include/core/array.se の定義に直す（`"control"` と `"boolean"` も入れる）。フロントエンドの Python 版（bin/resolver の `EXT_PRIMITIVES`）と ConMa 版（include/resolver.se の `__Resolver_features__`）が `(Uses "array" ...)` を書く。vm/frontend.module.se を bin/update-frontend で作り直した。test/fast12 は、step47 と同じプログラムを、代わりの定義を入れずに、フロントエンドが書いた `(Uses "array" "boolean" "control")` で動かし、`conma build` が同じモジュールを作ること、bin/to-core で直したモジュールも同じログになること、`__type_name__` が VM ごとに決めたもの（Python 版は `"Dict"`、ほかは `"Array"`。bin/to-core で直したものはどれも `"Dict"`）になることを確かめる。Python 版、Rust 版、wasm 版の VM で通る。test/frontend、test/selfhost1〜7、test/module1〜2、fast10、fast11、step45 も通る。
* **A4（作った、2026-11-17）**：変換した VM。6 つを conma2rust.py の `CHAIN` と transpiled.rs の `SAFE_PRIMS` に入れ、`__Array_length__`、`__Array_get__`、`__Array_set__`、`__Array_push__` は直接呼ぶ（`DIRECT`、transpiled.rs の `d_array_*`。prim/array.rs の `*_core` をプリミティブと共有する）。test/transpiled に case3（step47 の 5 つのケースを順に）を足した。
* **A5 の途中で見つけた、VM の前からの不具合（直した、2026-11-17、ブランチ m4/dict-num-hash）**：Rust 版の VM（wasm 版も同じ）の Dict は、数のキーを引くのに、キーの数に比例する時間がかかっていた。Dict の索引の `FxHasher` の `finish` が、掛け算の結果をそのまま返し、小さい整数の f64 のビットは下の桁が 0 なので、hashbrown が桶を選ぶ下の桁がどのキーでも同じになるため。rustc-hash 2 と同じく、`finish` で 26 ビット回すようにした。数のキーの Dict に 2.5 万、5 万、10 万個を入れて 10 回ずつ読み書きすると、0.92 秒、3.53 秒、14.73 秒だったのが、0.21 秒、0.37 秒、0.77 秒になった（Rust 版の VM、user）。
* **A5（測った、2026-11-17、Apple M4、user の 3 回の最小）**：n 個を足し、k 回、添字で全部を読んで足し合わせ、1 ずつ増やして書くプログラムを、配列のプリミティブで書いたものと、数のキーの Dict（`__Dict_put__`、`__Dict_lookup_or__`）で書いたもので比べた（ループと数の計算は同じ。モジュールは bench/module.sh で作った）。

| VM | n、k | 配列 | 数のキーの Dict |
|---|---|---|---|
| Rust 版 | 10 万、10 | 0.77 秒 | 0.76 秒 |
| wasm 版（Node.js） | 10 万、10 | 1.20 秒 | 1.19 秒 |
| 変換版（この 2 つだけを vm/conma2rust/build.sh で組み込んだもの） | 10 万、10 | 0.13 秒 | 0.16 秒 |
| Python 版（配列は bin/to-core の代わりの定義） | 1 万、1 | 9.36 秒 | 2.76 秒 |

  * Rust 版と wasm 版の VM では、どちらも 1 回 1 ステップのプリミティブで、時間の大半はステップの手間なので、差がない（数のキーの Dict は、上の不具合を直した後）。変換版では、配列の読み書きを直接呼ぶので、約 2 割速い。Python 版では、配列は代わりの定義の関数を呼ぶので、Dict を直接使うより約 3.4 倍遅い。
  * そのため、配列の利点は、Dict より速いことより、添字の範囲を確かめること、`push` と `pop`、長さを持つことと、SJS のように添字を文字列にしてキーにしていた所（1.2 の `Sjs_num_str` の 128 ステップ）をなくせることにある。A6（SJS で使う）は、これを踏まえて決める。
