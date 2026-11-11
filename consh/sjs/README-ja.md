# SJS の変換器

JavaScript の安全なサブセット SJS（[../SAFEJS-PLAN-ja.md](../SAFEJS-PLAN-ja.md)）を ConMa のソースに変換し、ConMa の VM で動かす（段階 B）。変換器は ConMa で書いてある。SJS のファイルを単独で変換して動かせ（下）、段階 B4 から、ConSh の `(( ))` と `$(( ))` も、変換したコードで動く（ConMa 版の ConSh のネイティブの VM と Node.js の wasm 版。下の「段階 B4」）。

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
├── bench/           段階 B7 の測定：loop.sjs、fib.sjs、nbody.sjs（bench/ と同じ計算）と run.sh
└── test/            step1〜step5（段階 B1〜B3、B5a、B5b）の caseN.sjs と caseN.log、run_all.sh、compare.sh
consh/include/
├── sjs_lex.se       字句解析（conma.js の jsTokens と同じ区切り方）
├── sjs_parse.se     構文解析（conma.js の sjsParse と同じ形の再帰下降。節は "t" を持つ Dict）
├── sjs_gen.se       ConMa のソースを作る（Sjs_translate）
└── sjs_rt.se        実行時の支え：値の種類、演算の型の確認、比較、関数、配列、オブジェクト、メンバー、メソッド、組み込み、数の書き方、JSON.stringify、console.log
```

変換器と実行時の支えは、ConSh からも使うので、consh/include に置く（名前は `sjs_` で始める）。ConSh の側は consh/include/consh_jsstmt.se の `Ar_eval` と `Sjsb_` で始まる関数。

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

## 段階 B4 で足したもの

* 書き方：`switch`、`do...while`、`**`、ビット演算（`& | ^ ~ << >> >>>` と、その複合の代入）、分割代入（宣言、代入、`for...of`、引数。既定値と残りの要素も）、スプレッド（呼び出し、配列、オブジェクト）、引数の既定値と残りの引数、`$名前`（シェル変数を読む。演算子の隣と、数のリテラルとの `===`、`!==` では数にする）
* 字句と構文の確認を、構文解析の前にする（段階 A と同じく、`this` などの名前、`constructor` などのメンバー、`==` と `!=` を `not in SJS` にする）
* ConSh の `(( ))` と `$(( ))`：式を `Sjs_translate_module` で、`Sjs_entry` だけを定義するモジュールのソースにし、`$TMPDIR/consh-sjs-ハッシュ.se`（ハッシュはソースから求めるので、同じ式は同じファイルになる）に書いて、`__VProc_spawn__` で新しい VProc として動かす。式ごとに、そのファイル名を覚えておく（文脈の global の `"sjsB"`）。モジュールは、実行時の支えの関数（`Sjs_rt_dict`）と文脈 G（組み込みと値の番号。ConSh 全体で 1 つ）を引数で受け取り、`"setup"`（`Sjsb_setup`）で、その VProc の PDict に、SJS変数の Dict（シェルの文脈の `"sjsvars"`）、`$名前` を引く関数、`console.log` の出力先（ConSh のリダイレクトに従う）、受け手のない例外の行き先を置く
* SJS の値は ConMa の値のまま、SJS変数とシェル変数に入る（ホストの表はない）。シェル変数には、文字列はそのまま、ほかは `{"sjsv": 値}` の Dict で入れ、`$` で読むたびにシェルの値にする（`Sjsb_to_shell`。配列は語の並び、ほかは `console.log` と同じ文字列）。SJS関数をコマンドとして呼ぶ（`$f 1 2`）のも、この値で動く
* 段階 B は、ホストの JavaScript に触れないので、`CONSH_ENABLE_JS` を要らない（`js` コマンドは、今までどおり `CONSH_ENABLE_JS=js` が要る）。モジュールを作れないときは、`CONSH_ENABLE_JS` が `sjs` か `js` なら段階 A で動かし、なければエラー（`SJS cannot run here`）にする

## 段階 B5a で足したもの：ジェネレータ（2026/11/11）

* `function*` の式と宣言、オブジェクトのメソッド `*m() { }`、`yield`、`yield 値`、`yield* 値`（配列、文字列、ジェネレータ。値はジェネレータの戻り値）
* ジェネレータのオブジェクト：`next(x)`、`return(x)`（`finally` を実行し、`catch` は通る）、`throw(x)`、`typeof` は `"object"`、`String` は `[object Generator]`、`JSON.stringify` と `console.log` は `{}`。走っているジェネレータの `next` は `TypeError: Generator is already running`
* `for...of` は、ジェネレータから値を 1 つずつ読む（終わらないジェネレータも `break` で抜けられる）。スプレッドと分割代入は、すべて読む
* 関数の宣言（`function f() { }`、`function* g() { }`）：JavaScript と同じく、ブロックの始めで値を入れる
* ConSh：ジェネレータは、作った `(( ))` の後も、別の `(( ))` で続けられる（継続は、作った VProc が終わった後も、別の VProc で呼べる）。シェルの値（`$g`）にすると、要素を読むたびに `next` を呼ぶ LazyList になる（2026/11/11 にユーザーと決めた）。そのときの本体は、ジェネレータを作ったときの SJS変数、`$名前`、`console.log` の出力先で動き、エラーなら、メッセージを標準エラーに出して、リストをそこで終える
* 段階 A にはジェネレータがない（`not in SJS: a generator`）ので、テスト（step4）のログは、Node.js で動かした結果と比べた

## 段階 B5b で足したもの：async と await（2026/11/11）

* `async function`（式と宣言）、`async (引数) => 本体`、`async 名前 => 本体`、`async` のメソッド、`await 式`（async 関数の本体の中だけ）
* Promise：`then`、`catch`、`finally`、`Promise.resolve`、`Promise.reject`、`Promise.all`。`typeof` は `"object"`、`String` は `[object Promise]`、`JSON.stringify` と `console.log` は `{}`
* マイクロタスクの列：`await` の後の続きと `then` の受け手は、JavaScript と同じ順で列に入り、`consh/sjs/sjs` はプログラムの最後に、ConSh は `(( ))` と `$(( ))` の評価ごと（SJS関数をコマンドとして呼んだときも）に、列を最後まで動かす（SAFEJS-PLAN-ja.md の 2.2 の 6）。順序は、async 関数が Promise を返すときの 2 つ多い段階や、仕様どおりの `finally` も含めて、Node.js と同じ（step5）
* 列が空になったとき、受け手のない拒否された Promise があれば、受け手のない例外（`Uncaught (in promise) ...`）にする。ConSh では、その評価のエラー（`$?` は 2）
* ConSh：シェルの値（`$p`）にした Promise は、成功していればその値、拒否されていれば空文字列になる（評価が終われば決着している）。`(( ))` の `$?` は、Promise は真なので 0
* 段階 A には `async` がない（`not in SJS: async`）ので、テストのログは Node.js で動かした結果と比べた

## 段階 B6：ブラウザ版（2026/11/11）

* ブラウザ版の ConSh（consh/js/web）でも、`(( ))` を段階 B で動かす。`__VProc_spawn__` が要るモジュールは、ページか Worker の中で、もう 1 つの VM のインスタンスに ConMa のフロントエンド（`frontend.module.se`。`build.sh` が `vm/frontend.module.se` を写す）を動かして作る（`web/consh-core.js` の `buildModule`。Node.js 版の `vm-node.js` と同じ）。フロントエンドは、最初に使うときにサーバーから読む
* 変換したソースは、メモリの `/tmp`（`web/serverfs.js`）に書く
* ホストの JavaScript を使わないので、`config.json` がなくても `(( ))` が使える。新しい式は 0.3 秒くらい、前に作った式は 0.15 秒くらいで動いた（2026/11/11、Worker で動かすページ）

まだ変換しないもの（`SyntaxError: not yet translated to ConMa: ...` になる）：async のジェネレータ、配列の穴、ラベル、コンマ演算子、タグ付きテンプレート、`for` の最初の部分の分割代入。getter と setter は SJS の範囲の外。

## 段階 B7：測定（2026/11/11）

`consh/sjs/bench/run.sh NAME N` で測る（NAME は loop、fib、nbody。bench/ の同じ計算を SJS で書いたもの。N_ARG を N にして動かす）。Apple M4、Rust 版の VM、user + sys 秒の 3 回の最良。Node.js と CPython は、同じ計算の JavaScript（SJS のファイルそのもの）と bench/ の Python。手で書いた ConMa は bench/ の `.se`。出力が Node.js と同じことも確かめる。

| | Node.js | CPython | 手で書いた ConMa | SJS（測ったとき） | SJS（直した後） |
|---|---|---|---|---|---|
| loop 100000 | 0.10 秒（ほぼ起動） | 0.017 秒 | 0.056 秒 | 1.28 秒 | 0.79 秒 |
| fib 20 | 0.10 秒（ほぼ起動） | 0.015 秒 | 0.020 秒 | 0.50 秒 | 0.31 秒 |
| nbody 2000 | 0.10 秒（ほぼ起動） | 0.022 秒 | 0.175 秒 | 9.94 秒 | 3.52 秒 |

* **ステップの数**（CONMA_PROFILE。N を 10 増やしたときの差から）：loop は 1 回に、手で書いた ConMa が 13、SJS が 273 から 167。fib は 1 回の呼び出しに 18.5 と 437 から 259。nbody は 1 回の時間の刻みに 1,766 と 100,994 から 36,402。直した後で、手で書いた ConMa の 13〜21 倍。起動（N が 1 か 0）は 0.01〜0.02 秒で、時間のほとんどは計算そのもの。
* **直したもの**（プロファイルで見つけた。直す前の nbody は、ステップの 45% が配列の添字に使われていた）：`Sjs_get_member` が、配列の数の添字を、JavaScript の数の書き方の文字列（`Sjs_num_str`）にしてから読み戻していたので、数のキーはそのまま `items` を引くようにした。`Sjs_kind` は Dict を最初に調べる。算術の型の確認（`Sjs_need2`）、`===`（`Sjs_seq`）、条件の真偽（`Sjs_truthy`）に、数と真偽値の速い道を足した。
* **残りの内訳**（直した後）：算術の型の確認（`Sjs_need2`。1 回に 19 ステップ）と `===` が、loop の約半分。手順の並び自体（Ref に入れた変数の読み書き、文の行の記録、ループの `continue` の継続）が約 35%。nbody では、メンバーの読み書き（種類を調べて、配列なら添字を確かめて Dict を引く）が多い。
* **大きさ**：SJS 1 行が、変換した ConMa の約 200 バイトになる（loop 199 バイト → 1,743 バイト、nbody 3,003 バイト → 26,649 バイト。どちらも約 9 倍）。モジュールは、実行時の支え（sjs_rt.se。約 0.99 MB）を含んで 1.0〜1.3 MB。ConSh の `(( ))` のモジュールは、実行時の支えを含まない（`Sjs_rt_dict` を受け取る）ので、式の大きさだけになる。
* **変換の時間**：変換器（Rust 版の VM で動かす sjsc.se）で、loop（9 行）が 0.03 秒、nbody（81 行）が 0.22 秒。ブラウザの ConSh では、新しい式が 0.26 秒（モジュールを作る時間を含む）。
* **エラーの位置**：VM の位置（SInfo）は、変換した ConMa の位置（1 つの関数が 1 行の長いソース）なので、SJS を書いた人には使えない。そのため、変換器が文ごとに `Sjs_line` で SJS の行を記録し、受け手のない例外のメッセージに付ける（1 つの文に 2 ステップ）。測っていて、関数の呼び出しの後の式のエラーが、呼ばれた関数の最後の文の行になる誤りを見つけ、呼び出しの後に行を記録し直すようにした（呼び出し 1 つに 2 ステップ）。残る限界：桁はない。複数の行にまたがる文は、文の始めの行になる（Node.js は式の行）。ConSh では、`(( ))` の位置だけを出し、中の行は出さない。実行時の支えの誤りは、VM が sjs_rt.se の位置で出す。
* REVIEW-ja.md（ConMa のレビュー、2026-11-06）の 5 への答えとして、同じ所の「対応」にまとめた。

## 変換の形

`consh/include/sjs_gen.se` の始めにも書いた。

* **値**：数は Number（IEEE 754 の倍精度で、JavaScript と同じ）、文字列は String、`true` と `false` は `__TRUE__` と `__FALSE__`（ConMa では Closure）、`undefined` は null、`null` は Ref（SJS の値で Ref になるのは `null` だけ）。関数、配列、オブジェクトは Dict で、`"t"` に種類、`"id"` に作った順の番号を持つ（ConMa には値が同じものかを比べるプリミティブがないので、`===` は番号で比べる）。関数は `"f"` に引数の LList を受け取る Closure を、配列は `"items"` に添字の文字列から値への Dict と `"length"` の Ref を、オブジェクトは `"props"` に名前から値への Dict と、名前を足した順の LList の Ref（`"keys"`）を持つ。プロトタイプはなく、メソッドは名前で選ぶ（`Sjs_array_method` など）。
* **式**は、値を求める手順の並び（`演算子 被演算子 ,(t) ` をつないだもの）と、値を表す被演算子にする。手順の並びは、そのまま後ろにつなげられる。
* **変数**は Ref に入れ、名前ごとに一意の ConMa の変数名（`v番号_名前`）を付ける。ブロックで宣言した名前は、ブロックの始めに Ref を作っておく（JavaScript と同じく、ブロックの始めから見える。自分を呼ぶ `const fact = (n) => ... fact(n - 1)` のため）。宣言していない名前は大域の名前で、文脈 `G` の Dict に置く。
* **関数**は、引数の LList を受け取る Closure を `Sjs_function` で包む。本体の最初で `__Cont_get__ ,(return)` を取り、`return` 文はそれを呼ぶ。足りない引数は `undefined`、余った引数は捨てる。
* **if** は `__if_then_else__`。**ループ**は、Ref に入れた自分を末尾で呼ぶ thunk（CChain は伸びない）を、`__Cont_get__` で脱出の継続（`break`）を取る thunk で包み、各回の本体を、もう 1 つの脱出の継続（`continue`）を取る thunk で包む（`Gen_loop`）。`&&`、`||`、`??`、条件演算子は、`__if_then_else__` の 2 つの thunk が値を返す形にし、右辺は必要なときだけ求める。
* **例外**：受け手の継続のスタックを PDict に置く。`Sjs_try` は、`catch` へ戻る脱出の継続を積んで本体を実行し、`throw` と実行時のエラー（`Sjs_raise`）は、いちばん上の受け手を取り出して呼ぶ。`return`、`break`、`continue` の継続は、それを取った所（関数とループの始め）で受け手のスタックを覚えておき、呼ぶ前に戻す（`Gen_jump`）。`finally` は、本体を「例外を受けて `finally` を実行し、投げ直す」`Sjs_try` で包み、本体の中の `return`、`break`、`continue` を、`finally` を実行してから外へ出る Closure に置き換える（`Gen_try`）。
* **ジェネレータ**：ジェネレータの関数を呼ぶと、引数を入れて、本体を thunk にしたジェネレータのオブジェクト（`Sjs_generator`）を返す。最初の `next` は、その `next` の上で本体を始める。`yield`（`Sjs_yield`）は、本体に戻る継続を覚え、今の `next` の継続に `{value, done}` を渡す。次の `next` は、そのときの継続を覚え直して、本体に戻る継続を呼ぶ（`Sjs_gen_resume`）。本体の受け手のスタックは、`next` を呼んだ所のものと別にし、いちばん下の受け手（境）が、本体から出た例外を、今の `next` を呼んだ所に投げ直す。`return(x)` は、`yield` の所から「return の印」を投げ、`catch` は通し（`Sjs_try_catch`）、`finally` は実行して、境で終える。`for...of` は、繰り返しの手順（`Sjs_iter_open`、`Sjs_iter_more`、`Sjs_iter_take`）で読む
* **async**：async 関数を呼ぶと、本体の thunk をジェネレータの本体にして、最初の `await` まで動かし（`Sjs_async`）、結果の Promise を返す。`await v` は `yield v` と同じで（`Sjs_await`）、止まるたびに、`v` の Promise に、本体を続ける（拒否なら、本体に投げる）受け手を付ける（`Sjs_async_step`）。Promise の決着は、受け手を呼ぶジョブを PDict の列（`"sjs_jobs"`）に入れ、`Sjs_run_jobs` が動かす
* **行**：字句解析が行を数え、文ごとに `Sjs_line` で PDict に置く（文 1 つに 1 ステップ）。
* **数の書き方**：ConMa の `__Number_toString__` は最も短い桁を出すが、指数の形（`1e-07`）と指数にする範囲が JavaScript と違うので、`Sjs_num_str` が桁と小数点の位置に分けて、JavaScript の `Number::toString` の規則で書き直す。`Infinity`、`-Infinity`、`NaN` も同じ。
* **`/` と `%`**：`__Number_divide__` は 0 で割るとエラーなので、`Sjs_div` が JavaScript と同じく `Infinity`、`-Infinity`、`NaN` にする。`%` のプリミティブはないので、`a - b * trunc(a / b)` で求める（大きな数では、JavaScript の `%` と下の桁が違うことがある）。

## 段階 A との違い（今わかっているもの）

* `for (let i ...)` の本体で `continue` したとき、本体の中で変えた `i` を、ループの変数に写し戻さない（`Gen_copy_in`）。
* `?.` は、その 1 つのメンバーだけを短絡する（`a?.b.c` で `a` が `null` なら、JavaScript は `undefined` だが、ここでは `.c` でエラー）。
* 文字列の長さと添字は、文字（コードポイント）で数える（JavaScript は UTF-16 の単位）。`toUpperCase` と `toLowerCase` は ASCII の英字だけ。
* `String(関数)` と、`console.log` に渡した関数は、関数のソースでなく `function () { [native code] }`（段階 A は、書き換えた後のソース）。

* `js 変数名=` の JS の値を `$名前` で読むと、その時点の写しになる（段階 A は同じ値を共有する）。数、真偽値、`null`、`undefined` はその値、オブジェクトはホストが出す JSON を読んだもの、配列は、要素を文字列か配列にしたもの（`[1, {}]` の要素は `"1"` と `"{}"`）。関数は読めない（`undefined variable`）。
* ジェネレータ：`for...of` を `break` などで抜けても、ジェネレータの `return()` を呼ばない（JavaScript は呼び、ジェネレータの `finally` が動く）。分割代入（`const [a, b] = gen()`）は、ジェネレータをすべて読む（JavaScript は要る分だけ読んで `return()` を呼ぶ）。`yield*` は、`next(x)` の `x`、`throw`、`return` を、委ねた先に渡さない。
* Promise：`new Promise`（`new` は SJS の範囲の外）、`Promise.race`、`allSettled`、`any`、`for await`、async のジェネレータはない。受け手のない拒否は、Node.js ではプロセスを終了ステータス 1 で終えるが、ここではメッセージを出して 2 にする。
* シェルの値にしたジェネレータのエラーのメッセージに、ConSh の位置が付かない（`consh: sjs: ...`）。
* `-0`：ConMa では `-0` と `0` を区別しないので、`console.log(-0)` は `0` を出す（Node.js は `-0`）。
* 関数を `console.log` に渡すと `[Function (anonymous)]` を出す（Node.js は名前を出すことがある）。
* `%` の精度（上）。
* 変換と実行は遅い：`consh/sjs/sjs` は、変換器のモジュールを毎回作るので、1 つのファイルに 2 秒くらいかかる。ConSh の `(( ))` は、初めての式ごとに、変換し、モジュールを作る（ネイティブ版は別のプロセスの `conma build`）。Rust 版の VM で、25 行ほどのスクリプトに 1.4 秒くらい、Python 版の VM では、`(( ))` の多いスクリプトに数十秒かかる（段階 B7 で測る）。

## テスト

`consh/sjs/test/step1`（段階 B1）の case1〜case6 と、`step2`（段階 B2）の case1〜case7 は、`compare.sh` で段階 A と同じ出力になることを確かめた（エラーの行の前置きを除く）。step1 の case7 は、まだ変換しない書き方のエラー。ConSh の `(( ))` は、consh/test/step21（ConMa 版の ConSh。ネイティブの VM と wasm 版で同じログ）と consh/js/test/step6（Node.js の wasm 版）で確かめる。
