[English](README.md) | **日本語**

# ConJS：軽量スレッドとブロッキング呼び出しを持つ JavaScript のサブセット

はじめに読むもの：[クイックスタート](QUICKSTART-ja.md)（ツールを入れ、ビルドし、ConJS で書いたサーバを動かす）と[チュートリアル](TUTORIAL-ja.md)（言語を例で）。

ConJS は、JavaScript のサブセットで、協調的に切り替わる軽量スレッド（以下「処理」）と、その処理だけを止めるブロッキング呼び出しを持つ。待つ呼び出し（ホストの関数、アニメーションの 1 コマ）を、`await` を書かずに、ふつうの呼び出しとして書ける。待っている間は、その処理だけが止まり、ほかの処理は動く。今は SJS（[../sjs/README-ja.md](../sjs/README-ja.md)。JavaScript の安全なサブセット）の変換器とランタイムを使って作っているが、SJS とは別の言語で、これから受け付ける JavaScript を広げていく。計画と設計は [docs/dev/PLATFORM-PLAN-ja.md](../docs/dev/PLATFORM-PLAN-ja.md) の 4.1 節と 4.2 節、作ったものは 8.4 節（段階 P3b、2026-11-19）。作りとテストは、開発者向けの [docs/dev/CONJS.md](../docs/dev/CONJS.md)（英語）にある。

```js
const t = spawn(() => host.wait("test.later", 2, "slow"));   // 新しい処理。呼んだ側は動き続ける
console.log(host.wait("test.later", 1, "fast"));             // この処理だけが待つ
console.log(join(t));                                        // t が終わるのを待ち、戻り値を受け取る
```

## 使い方

```
conjs/conjs file.con.js           # 変換して、wasm 版の VM（Node.js）で動かす（vm/wasm/build.sh で作っておく）。CONMA_HOST でホストの関数を渡せる
conjs/conjs --emit file.con.js    # 変換した ConMa のソースを出す
conjs/test/run_all.sh             # テスト（wasm 版の VM で）
conjs/test/compare.sh             # step10 の場合を Node.js と比べる
conjs/bench/run.sh 名前           # ベンチマークを Node.js と比べる（2026-11-24 から。表は conjs/bench/README-ja.md）
conjs/conjs --prepare             # 変換器とランタイムのモジュールを作っておく（初めての実行が速くなる。2026-11-23 から）
conjs/conjs --version             # ConMa の版を出す（2026-11-24 から。ConMa の README の Version）
```

ConJS は、2026-11-25 から、wasm 版の VM だけで動かす（段階 P10。docs/dev/PLATFORM-PLAN-ja.md の 4.13 節）。`conjs/lang.conf` がそれを決め、作っていなければ、`conjs/conjs` が作り方を出す。それまでは、既定で Rust 版の VM で動かし、`CONMA_VM` で選んだ VM（wasm 版と Python 版）でも動かした。`CONMA_VM=rust` は、開発者のために今も使えるが、テストはしない。`async` と `await` を使うコードは、ホストの関数にする（docs/HOST-ja.md の 3.2 節）。変換は、Rust 版の VM を作ってあればそれで行い（速い）、変換したプログラムは wasm 版の VM で動かす。Rust 版の VM を作っていなければ、変換も wasm 版の VM（vm/wasm/build.sh で作る。Node.js）で行う（2026-11-23 から。vm-node.js の `build` と `link`）。どちらで変換しても、できるモジュールは 1 バイトも同じ。環境変数 `CONMA_TRANSLATE_VM=wasm` で、Rust 版の VM があっても wasm 版で変換する。変換器は SJS のもの（sjs/include）に、言語の印（PDict の `"sjs_lang"` を `"conjs"`）を付けて使う。変換器のモジュールは、bin/cache-dir の場所に conjsc.module.se として作っておく（sjs/sjs と同じ）。ランタイム（変換したソースが include する conjs_rt.se など）のモジュールも、同じ所に conjs-rt-ハッシュ.module.se として 1 度だけ作っておき、変換したソースは include の行を除いてモジュールにしてから、`conma link` で合わせる（bin/lang。docs/dev/PLATFORM-PLAN-ja.md の 4.4 節、段階 P5）。`console.log("hi")` だけのプログラムは、wasm 版の VM で約 0.18 秒、Rust 版の VM で約 0.08 秒で動く（Apple M1、2026-11-25。その前は、Rust 版の VM で約 0.1 秒。もっと前は、毎回ランタイムをフロントエンドに通して約 1.45 秒）。

wasm 版の VM では、外の変数に触れない関数を、ホストの JavaScript で動かす。結果は同じで、速い（2026-11-24 から。docs/dev/CONJS-HOST-PLAN.md）。待つ呼び出しのうち、そのような関数が呼べるのは、`sleep`、`join`、`nextEvent` と、ほかのそのような関数で、ConMa と同じく、その処理で待つ。そのような関数が、ConMa の関数を呼び返した後や、待った後に、ホストにできないことに出会うと、プログラムは `conjs: the host function NAME could not go on after calling back: ...` か `... after waiting: ...` と出して止まる（status 2）。そのときは、すべての関数を ConMa で動かす `CONMA_NO_HOSTJS=1` を付けて流し直し、そのプログラムを知らせてほしい。`CONJS_HOST_REPORT=1` を付けると、どの関数がなぜ ConMa に戻ったかを出す。

## 言語

SJS で書けるもの（sjs/README-ja.md の「書けるもの」）から、下を除き、下を足したもの。

* **除くもの**（変換するときに `conjs: not in ConJS: async` のようなエラー、終了ステータス 2）：`async`、`await`（`for await` も）、名前 `Promise`。SJS の範囲の外のもの（`with` など）も、「not in ConJS」と書く。`then` は、ふつうの名前として使える。3 つの名前も、オブジェクトのキー（`{ async: 1 }`）とメンバーの名前（`o.await`）には使える（省略形の `{ async }` は変数なので、エラー）。
* **`import` と `export`**（段階 P9、2026-11-23。docs/dev/PLATFORM-PLAN-ja.md の 4.12 節。conjs/include/conjs_modules.se）：JavaScript の ES Modules と同じく、プログラムをファイルに分けて書ける。SJS は除く。
    * **受け付ける形**：`import { a, b as c } from "./lib.con.js"`、`export const`、`export let`、`export var`、`export function`、`export class`、`export { a, b as c }`。ファイルの最上位にだけ書ける。パスは `./` か `../` で始まる、読み込む側のファイル（シンボリックリンクなら、その先の本物のファイル）からの相対パスで、拡張子まで書く（Node.js と同じ）。
    * **意味**（JavaScript と同じ）：読み込まれるファイルは、読み込む側より先に、ES Modules と同じ深さ優先の順で 1 度だけ動く。名前はファイルごとに分かれる。import した名前はライブバインディングで（出した側が後で変えた `let` の値が見える）、代入できない。同じファイルを指す 2 つのパス（`./a/../lib.con.js`、シンボリックリンク）は、同じファイルになる。読み込まれるファイルの最上位でも待てる（`sleep`、`join`。JavaScript の最上位の `await` と同じ振る舞い）。
    * **変換するときのエラー**（終了ステータス 2）：ないファイル、export していない名前、2 度の export、循環（`import cycle: a.con.js -> b.con.js -> a.con.js`）、import した名前への代入、最上位でない所の `import` と `export`。
    * **まだ受け付けないもの**（「not in ConJS」）：`export default` と `import x from`、`import * as m`、`export ... from` と `export *`、名前のない `import "x"`、`import()` と `import.meta`。パスでない名前（`"fs"`）はエラー（ConJS のライブラリは、今のまま大域の名前）。
    * conjs/test/step19 のプログラムは、Node.js 22 以降でも同じに動く（`package.json` がなくても、`.js` の中の `import` を読む）。
* **クラス**（段階 P5b。docs/dev/PLATFORM-PLAN-ja.md の 4.5 節。conjs/include/conjs_class.se）：SJS は除くが、ConJS は受け付ける。
    * **`class`**（宣言と式）：`constructor`、メソッド、`static` のメソッド（親から継承する。`this` は、メソッドを読んだクラス）、インスタンスのフィールド（`x = 1;`。書いた順に、コンストラクタの前に入れる。初期化の式で `this` を使える）、`extends`（クラスか、組み込みのエラーのコンストラクタ）、`super(...)`、`super.m(...)`、ジェネレータのメソッド。`extends` のあるクラスでコンストラクタを書かなければ、引数をそのまま親に渡す。
    * **`new C(...)`**：C はクラスか、`Error`、`TypeError`、`RangeError`、`SyntaxError`、`ReferenceError`。ふつうの関数の `new` と、クラスの `new` なしの呼び出しは TypeError。エラーのコンストラクタは、`new` なしでも呼べる（JavaScript と同じ）。コンストラクタがオブジェクトを返せば、それが `new` の値になる（JavaScript と同じ）。
    * **`this`**：クラスのコンストラクタ、メソッド、フィールドの初期化と、その中のアロー関数の中だけ。ふつうの `function` の中とプログラムの上の階層では、変換するときのエラー（`conjs: this outside a class`）。`extends` のあるクラスのコンストラクタで、`super(...)` の前の `this` は ReferenceError。
    * **`instanceof`**：右はクラスか、エラーのコンストラクタ（ほかは TypeError）。ランタイムが投げるエラー（`null.x` の TypeError など）も、`Error` と、名前が同じエラーのコンストラクタで真。
    * **エラー**：エラーのオブジェクトは `name`、`message`、`toString()` を持つ。`name` と `toString` は、JavaScript のプロトタイプのものと同じく、メソッドと同じ道で見つけるので、`this.name = "MyErr"` は `Object.keys` と JSON に出る（JavaScript と同じ）。`message` は、`Object.keys` と JSON に出ない。
    * **足さないもの**（変換するときのエラー）：getter と setter、`#` で始まるプライベートなフィールド、`static` のフィールドとブロック、計算した名前のメソッド、`new.target`。プロトタイプ（`prototype`、`constructor`、`__proto__`）は、今までどおり、どの形でも読めない。
    * `console.log` は、Node.js と同じく、インスタンスを `Point { x: 1, y: 2 }`、クラスを `[class Point]`（`[class B extends A]`）と出す。`Object.keys`、`JSON.stringify`、スプレッドは、自分のプロパティだけを見る（JavaScript と同じ）。
* **`spawn(f)`**：引数のない関数 f を新しい処理として列の最後に入れ、処理（task）を返す。呼んだ側は、次に待つまで動き続ける。task は `console.log` で `[Task]`、`typeof` は `"object"`。
* **`join(t)`**：処理 t が終わるまで待ち、f の戻り値を返す。t が例外で終わっていれば、その例外を、t の中で投げた所の位置のまま投げ直す。何度でも、いくつの処理からでも join できる。自分自身の join は TypeError。
* **`host.call(name, ...args)`**：すぐ返るホストの関数（`__Host_call__`）。**`host.wait(name, ...args)`**：待つホストの関数（`__Host_start__`）。**`host.available(feature)`**：`__Feature_available__`。渡せるのは、数、文字列、真偽、オブジェクト（自分のプロパティだけ）、それらの配列で、ホストが返した LList は配列、Dict はオブジェクト、null は `undefined` になる（空の配列も `undefined` になる）。ホストにない関数と、関数の失敗は、ホストのメッセージを持つ `Error` を投げる。ホストの関数の足し方（ブラウザ、Node.js、Rust 版の VM）と、渡せる値の詳しいことは [docs/HOST-ja.md](../docs/HOST-ja.md)（段階 P7）。テストの関数（`test.add`、`test.echo`、`test.fail`、`test.later`）は、どの VM にもある。
* **待つ呼び出しのライブラリ**（段階 P5。conjs/include/conjs_lib.se）：名前は JavaScript にならうが、Promise を返さず、結果が届くまで、呼んだ処理だけが待つ。SJS が除くホストのものなので、ConJS だけにあり、SJS にはない。
    * **`sleep(ms)`**：ms ミリ秒、呼んだ処理だけが待つ（JavaScript にない名前）。ms が `undefined`、負の数、NaN、無限なら 0。数でなければ TypeError。`undefined` を返す。
    * **`setTimeout(f, ms, ...args)`**：`spawn(() => { sleep(ms); return f(...args); })` と同じ。JavaScript と違い、数ではなく処理を返すので、`join` で f の戻り値を受け取れる（`clearTimeout` はない）。f が投げた例外は、JavaScript のようにその場でプログラムを止めず、join されない処理の例外として、プログラムの終わりに報告する。
    * **`Date.now()`**：1970 年 1 月 1 日からのミリ秒（整数）。`Date` はこれだけを持つオブジェクトで、`new Date()` は書けない（`new` は、クラスとエラーのコンストラクタだけ）。
    * **`fs.readFile(path)`、`fs.writeFile(path, text)`**：ファイル全体を文字列で読み書きする（Node.js の名前。`fs` はグローバルにある）。相対パスは、プログラムを動かしたディレクトリから。ふつうのファイルはすぐ読み書きできるので、処理は切り替わらない。読み書きできなければ `fs.readFile: ` で始まるメッセージの Error。ブラウザのページ（ホストのファイルを渡さない wasm 版の VM）では、VM のメモリの中のファイルを読み書きする（ページを閉じると消える。まだ確かめていない）。
    * **ファイルを 1 行ずつ読む**（段階 P8。conjs/include/conjs_file.se）：
        * **`fs.open(path, mode)`**：ファイルを開き、ファイルのオブジェクトを返す。mode は `"r"`（既定）、`"w"`、`"a"`。開けなければ Error。Node.js の `fs.promises.open` が返す FileHandle に似せた（Node.js の `fs.open` は、ファイル記述子をコールバックに渡す別のもの）。
        * ファイルのオブジェクト（`console.log` では `File { path: 'a.txt' }`）：`readLine()`（次の 1 行。`\n` か `\r\n` を除く。終わりなら `undefined`）、`readLines()`（残りの行を返すジェネレータ。`for (const line of f.readLines())`。終わったときと、`break`、例外、`return` で抜けたときに閉じる）、`read()`（残りを全部）、`write(text)`、`close()`（何度でもよい）、`closed`（閉じたか）。閉じた後に読み書きすると Error。
        * **`fs.readLines(path)`**：`fs.open(path).readLines()` と同じ（Node.js にはない近道）。
        * **`fs.stdin`**、**`fs.stdout`**：標準入力（読むだけ）と標準出力（`write` だけ。改行を足さない）のファイルのオブジェクト。標準入力は、キー（`nextEvent`）と行の両方では読めない（先に使ったほうに決まり、もう一方は Error）。
        * ふつうのファイルは待たずに読む。パイプや標準入力で、まだ届いていなければ、呼んだ処理だけが待つ。読む時間は行の数に比例する（wasm 版の VM で、1 行およそ 31 マイクロ秒。Apple M1）。
    * **`argv`**（段階 P8）：プログラムに渡した引数の配列（`conjs/conjs file.con.js a b` の `["a", "b"]`。Node.js の `process.argv.slice(2)`）。
    * **`fetch(url, options)`**：HTTP の要求。options は `method`（文字列）、`headers`（オブジェクト）、`body`（文字列）。返すのは、本体を全部読んである Response のようなオブジェクトで、`status`、`ok`（状態が 200〜299）、`text()`、`json()` を持つ（どれも待たない）。404 などは例外にせず `ok` を偽にし、つながらないときは TypeError（`fetch failed`）を投げる（JavaScript と同じ）。wasm 版の VM ではホストの `fetch`（Node.js 18 以降とブラウザ）を、Rust 版と Python 版の VM では `curl` を、待つための VProc で動かす（curl がなければ TypeError）。`json()` は `JSON.parse` と同じ読み手を使う。curl の道では、HEAD は `-X HEAD` でなく `--head` で送る（`-X HEAD` の curl は、来ない本体を待って失敗する）。
    * **`http.listen(port, host)`**（段階 P6b。Node.js だけ）：HTTP のサーバを始め、サーバのオブジェクトを返す。port は数（書かなければ 0。OS が選ぶ）、host は文字列（書かなければ `"127.0.0.1"`。ほかの機械から届かない）。wasm 版の VM を Node.js で動かすとき（機能 `"server"`。vm/wasm/host/server.js）だけ使え、ほかの VM では Error（`http.listen: this platform has no server (Node.js only)`）。使えない port は Error（`server.listen: EADDRINUSE: address already in use`）。
        * サーバのオブジェクト：`port`（実際の番号）、`accept()`（次の要求。来るまで、呼んだ処理だけが待つ。受け取る処理がいない間に来た要求は、列にためる。閉じた後は `undefined`）、`close()`（新しい要求を受けなくなる。受け取った要求には、その後も答えられ、列にたまっていて受け取っていない要求は 503 で閉じる。待っている `accept()` は `undefined` を受け取る）。
        * 要求のオブジェクト：`method`、`url`（パスと `?` の後）、`headers`（Node.js の `req.headers` のまま：名前は小文字、同じ名前のヘッダはまとめ、`set-cookie` だけは配列）、`body`（全部読んだ文字列）。`respond(status, headers, body)`（答える。headers はオブジェクトで、省いて `respond(status, body)` とも書ける。body がなければ空。2 度目は Error。ヘッダの値が文字列、数、それらの配列でないか、改行を含むなど Node.js が受け付けなければ Error で、要求は残り、答え直せる）、`json()`（`JSON.parse(body)`）。本体が 1 MB を超える要求は、受け取らずに 413 で答える。
        * **終わり方**：サーバは、`close()` するか、Ctrl-C で終わるまで、要求を待ち続ける。`accept()` で待つ処理だけが残るのは、動いているサーバのふつうの状態なので、行き詰まりとはしない（Node.js の `http.createServer().listen()` と同じ）。
        * **JavaScript との違い**：Node.js の `http.createServer` は、要求ごとにコールバックを呼び、待つ所に `async` と `await` が要る。ConJS では、`for (;;) { const req = server.accept(); spawn(() => handle(req)); }` と、要求を順に受け取るループで書き、1 つの要求の中は上から下に書ける（Go の `net/http` や、スレッドのサーバと同じ形）。形が違うので、名前も `http.createServer` と分けた。
* **入力を待つ**（段階 P5c と P6a）：キーとクリックを、待つ呼び出しとして待つ。待っていない間に来た入力も捨てず、届いた順に列にためる。
    * ブラウザのページ（段階 P5c）：ホストの関数 `input.next` と `input.poll`（機能 `"input"`。vm/wasm/host/input.js）。`host.available("input")` は、この機能があるかを表す。
    * 端末（段階 P6a）：機能 `"input"` がなければ、標準入力からキーを読む（conjs/include/conjs_term.se。VM は変えていない）。標準入力が端末なら、初めて読むときに `stty -icanon -echo` で 1 文字ずつ、打った文字を出さずに読み、プログラムの終わりに元に戻す（ふつうに終わったとき、受け手のない例外のとき、Ctrl-C のとき。Ctrl-C は終了ステータス 130）。ファイルやパイプからも読む。キーの名前はブラウザに合わせる：ふつうの文字、`"Enter"`、`"Tab"`、`"Backspace"`、`"ArrowUp"` などの矢印、`"Escape"`、それ以外の ESC の並びと制御文字は `"Unidentified"`。クリックは来ない。標準入力が尽きると、`nextEvent` は Error（`nextEvent: the input has ended`）を投げる。macOS と Linux だけ（Windows は、まだ）。
    * 端末が元に戻らなかったとき（ConMa のエラーで VM が止まったときなど、まれな場合）は、別の端末から VM を止めて、`stty sane`（か `reset`）を打つ。
    * **`nextEvent(ms)`**：列の先頭の入力を返す。列が空なら、次の入力まで、呼んだ処理だけが待つ。入力は `{ type: "key", key: "ArrowUp" }`（`key` は `KeyboardEvent.key`。押し続けたときの繰り返しも 1 つずつ）か `{ type: "click", x: 10, y: 20 }`（canvas の座標）。ms の間に入力が来なければ `undefined`。ms が `undefined`（書かない）か `Infinity` なら時間を限らず、負の数と NaN は 0、数でなければ TypeError（`sleep` と違い、書かなければ限らない）。2 つの処理が同時に待てば、先に待ったほうが先の入力を受け取る。
    * **`pollEvent()`**：列の先頭の入力か、空なら `undefined`。待たない。画面を変えるときに `while (pollEvent()) {}` で残りを捨てると、前の画面で押したキーが次の画面に渡らない。
* **JavaScript との違い**（ファイル）：Node.js で行を読む `readLines()` は `for await` で回すが、ConJS では `for` で回す（結果が届くまで、呼んだ処理だけが待つ）。行の終わりは `\n` と `\r\n` だけで、Node.js の readline と違い、単独の `\r` は行の終わりにしない（`"a\rb\nc"` は、Node.js では 3 行、ConJS では `"a\rb"` と `"c"` の 2 行。そのため、単独の `\r` を含むファイルでは、merge.con.js と merge.node.js の出力が違う）。標準入力、標準出力、引数は、Node.js では `process.stdin`、`process.stdout`、`process.argv` だが、ConJS には `process` がないので、`fs.stdin`、`fs.stdout`、`argv` に置く。
* **JavaScript との違い**（クラス）：メソッドは、メンバーを読むたびに、`this` を付けた関数として作る。そのため、取り出したメソッドを後で呼んでも、`this` は元のオブジェクトのまま（JavaScript では `undefined`）で、`o.m === o.m` は偽（JavaScript では真）。プロトタイプがないので、メソッドは、メンバーを読んで自分のプロパティになかったときだけ、クラスと親から探す。`extends` のあるクラスのコンストラクタが `super(...)` を呼ばずに終わっても、エラーにしない（JavaScript では ReferenceError。親とこのクラスのフィールドは入らない）。親のコンストラクタがオブジェクトを返しても、子の `this` にはならない（JavaScript ではなる。`new` の値になるのは、`new` で書いたクラスのコンストラクタが返したものだけ）。
* **関数の色がない**：どの関数の中でも待てる（配列の `map` に渡した関数、ジェネレータの本体、try と finally の中）。
* **割り込まれない**：処理は、1 つの VProc の中で、待つ呼び出し（`host.wait` と `join`）の所でだけ切り替わる。2 つの待つ呼び出しの間のコードは、割り込まれずに動く。待たない長いループは、その間ほかの処理を止める（JavaScript と同じ）。
* **動かす順**：動かせる処理の列は FIFO。待つ呼び出しの結果が届いた処理も、列の最後に入る。結果は、動かせる処理がなくなったときに 1 つずつ受け取るので、spawn し続けて列が空にならないプログラムには、ホストの結果が届かない（Node.js でマイクロタスクを積み続けたときと同じ）。
* **終わり**：main（ファイルの文の並び）が受け手のない例外で終われば、ほかの処理を待たずに、すぐに終える（終了ステータス 2）。main が終わっても、残っている処理は動かし続ける。すべてが終わったら、join されないまま例外で終わった処理を、終わった順に `ファイル:行:桁: Uncaught (in task) メッセージ` で出し、終了ステータス 2 にする。join で待つ処理だけが残ったら、`conjs: Deadlock: tasks waiting in join: N` を出し、終了ステータス 2 にする。

ConJS の構文は JavaScript のサブセットだが、ConJS のプログラムは JavaScript のプログラムではない（`spawn`、`join`、`host` は JavaScript になく、JavaScript では待つ呼び出しが Promise を返す）。そのため、多くの場合は sjs/test のように Node.js と出力を比べない。それらと待つ呼び出しのライブラリを使わないプログラムは JavaScript のプログラムでもあるので、conjs/test/compare.sh が Node.js と標準出力を比べる（各ステップの node.txt に並べた場合。2026-11-20 までは step10 だけ）。ConJS と SJS の両方が受け付ける、待つ呼び出しを使わないプログラムは、どちらでも同じ結果になる（同じ変換器とランタイムで動くため）。ConJS を広げるときは、まず ConJS に足し、そのうち安全なものを SJS に移して共有する（docs/dev/PLATFORM-PLAN-ja.md の 4.1 節）。

## 見本

* **ファイルの見本**（段階 P8。conjs/examples）：merge.con.js は、ソートした 2 つのファイルを 1 行ずつ読み比べ、片方だけにある行と両方にある行を出す（Unix の `comm` と同じ。`-` は標準入力）。2 つの `readLine` を、比べた結果で交互に進めるだけで書ける。同じことを Node.js で書いた merge.node.js では、`readline` の Interface を 2 つ作り、`for await` が使えないので、2 つの非同期のイテレータの `next()` を `await` で呼び、関数を `async` にし、`try`／`finally` で閉じる。2 つが同じ出力になることを、conjs/test/compare.sh が確かめる（conjs/test/node-pairs.txt）。wordcount.con.js は、`fs.readLines` で語を数える、いちばん短い使い方の例。
* **サーバの見本**（段階 P6b。conjs/examples）：counter_server.con.js は、要求ごとに処理を `spawn` する小さなサーバ（`/count`、`/slow`、`/file?name=...`、`/stop`）。`/slow` が `sleep(200)` で待つ間も、ほかの要求に答える。数は共有の変数に持つが、読んでから書くまでに待つ所がないので、ほかの処理に割り込まれない。`demo` を引数にすると、OS が選ぶ port で答え、自分に要求を送って答えを出し、`/stop` で終わる。同じことを Node.js で書いた counter_server.node.js では、`http.createServer` のコールバックを `async` にし、`setTimeout` の Promise と、`fs.promises` と `readline` の `for await` を使う。2 つが `demo` で同じ出力になることを、conjs/test/step18 の case2.node-check.js が確かめる（conjs/test/compare.sh が動かす）。
* **ホストの関数の見本**（段階 P7。conjs/examples）：host_demo.con.js は、ホストの関数 `demo.*` を呼ぶ。同じ関数を、Rust の vm/examples/host_demo.rs と、JavaScript の host_demo.host.js（wasm 版の VM の `CONMA_HOST`）で書いてあり、test/host-api.sh が、2 つで同じログになることを確かめる（Rust のほうは `CONMA_VM=rust` で。Rust の API は、段階 P10 から、ConMa のプログラムのための口）（[docs/HOST-ja.md](../docs/HOST-ja.md)）。
* **ページの見本**（conjs/web。2026-11-23 までは vm/wasm/web）：canvas_demo.con.js（canvas の跳ねる玉。canvas.html で見る）と、stars.con.js（入力を待つ対話のゲーム「星ひろい」。stars.html で遊ぶ。段階 P5c。端末では conjs/examples/stars-term.con.js で遊ぶ。`conjs/conjs conjs/examples/stars-term.con.js`。段階 P6a。ゲームの決まり（両方で同じ関数：面の作り方、おばけ、キーで動くこと）を stars.con.js から import し、画面の順と描き方はそれぞれのファイルが持つので、stars.con.js に端末の分はなく、stars-term.con.js に canvas の分はない。2026-11-23 から 2026-11-24 までは、画面の順（`play`）も import し、描き方を渡した。その前は stars.con.js が両方で動いた）。ページの分は、conjs/web/build.sh が変換する。同じゲームを、ふつうのブラウザの JavaScript で 2 通りに書いたものもある。stars-events.js（イベント駆動：画面を表す変数と、キーの受け手の中の分岐、おばけの setInterval）と stars-async.js（async/await：ConJS 版と同じ形だが、待つ所すべてに await が要り、キーを待つ Promise を自分で作る）で、stars-events.html と stars-async.html で遊ぶ。compare.html は、3 つのうち 2 つを並べて見せ、待つ所の行に印を付ける（2026-11-23 から）。
