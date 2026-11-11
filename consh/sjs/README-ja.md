# SJS の変換器

JavaScript の安全なサブセット SJS（[../SAFEJS-PLAN-ja.md](../SAFEJS-PLAN-ja.md)）を ConMa のソースに変換し、ConMa の VM で動かす（段階 B）。変換器は ConMa で書いてある。今は段階 B3 で、ConSh の `(( ))` にはまだつないでおらず（段階 B4）、SJS のファイルを単独で変換して動かす。

```
consh/sjs/sjs file.sjs            # 変換して動かす（Rust 版の VM。vm で cargo build --release しておく）
consh/sjs/sjs --emit file.sjs     # 変換した ConMa のソースを出す
consh/sjs/test/run_all.sh         # テスト
consh/sjs/test/compare.sh f.sjs   # 段階 A（ConSh の (( )) でホストの JavaScript）と出力を比べる（Node.js と wasm 版の VM が要る）
```

SJS のファイルは、文の並びである（関数の本体と同じ。宣言していない名前への代入は、大域の名前になる）。`console.log` で出力する。出力の形は ConSh の段階 A（conma.js の `inspect`）と同じで、文字列はそのまま、配列とオブジェクトは `JSON.stringify`、ほかは `String` にする。受け手のない例外は、最後に実行した文の行と一緒に `sjs: line 4: TypeError: ...` のように標準エラーに出し、終了ステータス 2 で終える。

## ファイル

```
consh/sjs/
├── sjs              変換して動かすスクリプト
├── sjsc.se          変換器の main：SJS のファイルを読み、ConMa のソースを出す
└── test/            step1〜step3（段階 B1〜B3）の caseN.sjs と caseN.log、run_all.sh、compare.sh
consh/include/
├── sjs_lex.se       字句解析（conma.js の jsTokens と同じ区切り方）
├── sjs_parse.se     構文解析（conma.js の sjsParse と同じ形の再帰下降。節は "t" を持つ Dict）
├── sjs_gen.se       ConMa のソースを作る（Sjs_translate）
└── sjs_rt.se        実行時の支え：値の種類、演算の型の確認、比較、関数、配列、オブジェクト、メンバー、メソッド、組み込み、数の書き方、JSON.stringify、console.log
```

変換器と実行時の支えは、段階 B4 で ConSh から使うので、consh/include に置く（名前は `sjs_` で始める）。

## 段階 B1 で変換するもの

* 値：数、文字列、`true`、`false`、`null`、`undefined`、関数
* 式：算術（`+ - * / %`）、単項の `- + ! typeof`、大小の比較（数どうしか文字列どうし）、`===`、`!==`、`&&`、`||`、`??`、条件演算子、代入（`=`、`+= -= *= /= %=`）、`++`、`--`、呼び出し、アロー関数（本体は式かブロック）、`function` 式、`console.log`
* 文：`let`、`const`、`var`（名前だけ。分割代入はまだ）、ブロック、`if`/`else`、`while`、`return`、式の文。文の終わりは `;`、`}`、改行
* 演算の型は、段階 A と同じく確かめる（`+` は数どうしだけ、など。メッセージも同じ）

## 段階 B2 で足したもの

* 配列のリテラル（`[1, 2]`）、オブジェクトのリテラル（`{a: 1, b, [k]: v, "s": 1, m() {...}}`）
* メンバー（`o.a`、`o[k]`、`o?.a`、`o?.[k]`、`f?.()`）、メンバーへの代入（`=`、複合の代入、`++`、`--`）。計算したキーは、段階 A と同じく `constructor` などをエラーにする
* テンプレート文字列（`` `a${x}b` ``。入れ子も）。`${}` の値は `String` にする
* `for (初期化; 条件; 更新)`（`let` の変数は回ごとに作り直す）、`for...of`（配列と文字列）、`for...in`（オブジェクトの名前、配列と文字列の添字）、`break`、`continue`
* 配列のメソッド：`push`、`pop`、`slice`、`indexOf`、`includes`、`join`、`map`、`filter`、`forEach`、`some`、`every`、`find`、`findIndex`、`reduce`、`concat`、`reverse`、`length`
* 文字列のメソッド：`slice`、`indexOf`、`includes`、`startsWith`、`endsWith`、`split`、`trim`、`toUpperCase`、`toLowerCase`、`repeat`、`charAt`、`length`、添字
* 組み込み：`console.log`、`Math`（`floor`、`ceil`、`round`、`trunc`、`abs`、`sqrt`、`sign`、`max`、`min`、`PI`）、`JSON.stringify`、`Object.keys`、`values`、`entries`、`Array.isArray`、`String`、`Number`（`Number.isInteger`）、`Boolean`、`isNaN`、`parseInt`、`Infinity`、`NaN`。組み込みは、始めに G に置く SJS のオブジェクトと関数なので、ふつうのメンバーと呼び出しで使う
* オブジェクトの名前の順は、JavaScript と同じく、配列の添字の形の名前を小さい順に先に、ほかを足した順に
* 関数の名前：`const f = () => ...`、`f = ...`、`{m: () => ...}`、メソッド、名前のある `function` 式

## 段階 B3 で足したもの

* `throw 値`、`try { } catch (e) { }`（`catch { }` も）、`finally { }`
* 実行時のエラー（`TypeError` など）は、隠した `name` と `message` を持つオブジェクトとして投げる。段階 A の `Error` と同じく、`e.message`、`` `${e}` ``（`TypeError: ...`）が使え、`JSON.stringify(e)` は `{}`、`Object.keys(e)` は空
* 受け手のない例外のメッセージに、最後に実行した文の行を付ける（`sjs: line 4: ...`）。段階 A は ConSh の `(( ))` の位置を出す

まだ変換しないもの（`SyntaxError: not yet translated (stage B3 of SJS): ...` になる）：`switch`、`do`、`**`、ビット演算、分割代入、スプレッド、既定値、残りの引数、配列の穴、ラベル、`$名前`（段階 B4）。getter と setter は SJS の範囲の外。

## 変換の形

`consh/include/sjs_gen.se` の始めにも書いた。

* **値**：数は Number（IEEE 754 の倍精度で、JavaScript と同じ）、文字列は String、`true` と `false` は `__TRUE__` と `__FALSE__`（ConMa では Closure）、`undefined` は null、`null` は Ref（SJS の値で Ref になるのは `null` だけ）。関数、配列、オブジェクトは Dict で、`"t"` に種類、`"id"` に作った順の番号を持つ（ConMa には値が同じものかを比べるプリミティブがないので、`===` は番号で比べる）。関数は `"f"` に引数の LList を受け取る Closure を、配列は `"items"` に添字の文字列から値への Dict と `"length"` の Ref を、オブジェクトは `"props"` に名前から値への Dict と、名前を足した順の LList の Ref（`"keys"`）を持つ。プロトタイプはなく、メソッドは名前で選ぶ（`Sjs_array_method` など）。
* **式**は、値を求める手順の並び（`演算子 被演算子 ,(t) ` をつないだもの）と、値を表す被演算子にする。手順の並びは、そのまま後ろにつなげられる。
* **変数**は Ref に入れ、名前ごとに一意の ConMa の変数名（`v番号_名前`）を付ける。ブロックで宣言した名前は、ブロックの始めに Ref を作っておく（JavaScript と同じく、ブロックの始めから見える。自分を呼ぶ `const fact = (n) => ... fact(n - 1)` のため）。宣言していない名前は大域の名前で、文脈 `G` の Dict に置く。
* **関数**は、引数の LList を受け取る Closure を `Sjs_function` で包む。本体の最初で `__Cont_get__ ,(return)` を取り、`return` 文はそれを呼ぶ。足りない引数は `undefined`、余った引数は捨てる。
* **if** は `__if_then_else__`。**ループ**は、Ref に入れた自分を末尾で呼ぶ thunk（CChain は伸びない）を、`__Cont_get__` で脱出の継続（`break`）を取る thunk で包み、各回の本体を、もう 1 つの脱出の継続（`continue`）を取る thunk で包む（`Gen_loop`）。`&&`、`||`、`??`、条件演算子は、`__if_then_else__` の 2 つの thunk が値を返す形にし、右辺は必要なときだけ求める。
* **例外**：受け手の継続のスタックを PDict に置く。`Sjs_try` は、`catch` へ戻る脱出の継続を積んで本体を実行し、`throw` と実行時のエラー（`Sjs_raise`）は、いちばん上の受け手を取り出して呼ぶ。`return`、`break`、`continue` の継続は、それを取った所（関数とループの始め）で受け手のスタックを覚えておき、呼ぶ前に戻す（`Gen_jump`）。`finally` は、本体を「例外を受けて `finally` を実行し、投げ直す」`Sjs_try` で包み、本体の中の `return`、`break`、`continue` を、`finally` を実行してから外へ出る Closure に置き換える（`Gen_try`）。
* **行**：字句解析が行を数え、文ごとに `Sjs_line` で PDict に置く（文 1 つに 1 ステップ）。
* **数の書き方**：ConMa の `__Number_toString__` は最も短い桁を出すが、指数の形（`1e-07`）と指数にする範囲が JavaScript と違うので、`Sjs_num_str` が桁と小数点の位置に分けて、JavaScript の `Number::toString` の規則で書き直す。`Infinity`、`-Infinity`、`NaN` も同じ。
* **`/` と `%`**：`__Number_divide__` は 0 で割るとエラーなので、`Sjs_div` が JavaScript と同じく `Infinity`、`-Infinity`、`NaN` にする。`%` のプリミティブはないので、`a - b * trunc(a / b)` で求める（大きな数では、JavaScript の `%` と下の桁が違うことがある）。

## 段階 A との違い（今わかっているもの）

* `for (let i ...)` の本体で `continue` したとき、本体の中で変えた `i` を、ループの変数に写し戻さない（`Gen_copy_in`）。
* `?.` は、その 1 つのメンバーだけを短絡する（`a?.b.c` で `a` が `null` なら、JavaScript は `undefined` だが、ここでは `.c` でエラー）。
* 文字列の長さと添字は、文字（コードポイント）で数える（JavaScript は UTF-16 の単位）。`toUpperCase` と `toLowerCase` は ASCII の英字だけ。
* `String(関数)` と、`console.log` に渡した関数は、関数のソースでなく `function () { [native code] }`（段階 A は、書き換えた後のソース）。

* `-0`：ConMa では `-0` と `0` を区別しないので、`console.log(-0)` は `0` を出す（Node.js は `-0`）。
* 関数を `console.log` に渡すと `[Function (anonymous)]` を出す（Node.js は名前を出すことがある）。
* `%` の精度（上）。
* 変換と実行は遅い：`consh/sjs/sjs` は、変換器のモジュールを毎回作るので、1 つのファイルに 2 秒くらいかかる（段階 B7 で測り、段階 B4 で覚えておく仕組みを作る）。

## テスト

`consh/sjs/test/step1`（段階 B1）の case1〜case6 と、`step2`（段階 B2）の case1〜case7 は、`compare.sh` で段階 A と同じ出力になることを確かめた（エラーの行の前置きを除く）。step1 の case7 は、まだ変換しない書き方のエラー。
