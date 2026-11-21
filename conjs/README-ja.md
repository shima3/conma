# ConJS：軽量スレッドとブロッキング呼び出しを持つ JavaScript の部分集合

ConJS は、JavaScript の部分集合で、協調的に切り替わる軽量スレッド（以下「処理」）と、その処理だけを止めるブロッキング呼び出しを持つ。待つ呼び出し（ホストの関数、アニメーションの 1 コマ）を、`await` を書かずに、ふつうの呼び出しとして書ける。待っている間は、その処理だけが止まり、ほかの処理は動く。今は SJS（[../sjs/README-ja.md](../sjs/README-ja.md)。JavaScript の安全な部分集合）の変換器とランタイムを使って作っているが、SJS とは別の言語で、これから受け付ける JavaScript を広げていく。計画と設計は [docs/dev/PLATFORM-PLAN-ja.md](../docs/dev/PLATFORM-PLAN-ja.md) の 4.1 節と 4.2 節、作ったものは 8.4 節（段階 P3b、2026-11-19）。

```js
const t = spawn(() => host.wait("test.later", 2, "slow"));   // 新しい処理。呼んだ側は動き続ける
console.log(host.wait("test.later", 1, "fast"));             // この処理だけが待つ
console.log(join(t));                                        // t が終わるのを待ち、戻り値を受け取る
```

## 使い方

```
conjs/conjs file.con.js           # 変換して、Rust 版の VM で動かす（vm で cargo build --release しておく）
conjs/conjs --emit file.con.js    # 変換した ConMa のソースを出す
CONMA_VM=wasm conjs/conjs f.con.js    # wasm 版の VM（Node.js）で動かす。CONMA_HOST でホストの関数を渡せる
CONMA_VM=python conjs/conjs f.con.js  # Python 版の VM で動かす
conjs/test/run_all.sh             # テスト（CONMA_VM で VM を選ぶ）
conjs/test/compare.sh             # step10 の場合を Node.js と比べる
```

変換はいつも Rust 版の VM で行い、変換したプログラムを選んだ VM で動かす。変換器は SJS のもの（sjs/include）に、言語の印（PDict の `"sjs_lang"` を `"conjs"`）を付けて使う。変換器のモジュールは、bin/cache-dir の場所に conjsc.module.se として作っておく（sjs/sjs と同じ）。ランタイム（変換したソースが include する conjs_rt.se など）のモジュールも、同じ所に conjs-rt-ハッシュ.module.se として 1 度だけ作っておき、変換したソースは include の行を除いてモジュールにしてから、`conma link` で合わせる（bin/lang。docs/dev/PLATFORM-PLAN-ja.md の 4.4 節、段階 P5）。`console.log("hi")` だけのプログラムは、Rust 版の VM で約 0.1 秒で動く（前は、毎回ランタイムをフロントエンドに通して約 1.45 秒）。

## 言語

SJS で書けるもの（sjs/README-ja.md の「書けるもの」）から、下を除き、下を足したもの。

* **除くもの**（変換するときに `conjs: not in ConJS: async` のようなエラー、終了ステータス 2）：`async`、`await`（`for await` も）、名前 `Promise`。SJS の範囲の外のもの（`import`、`with` など）も、「not in ConJS」と書く。`then` は、ふつうの名前として使える。3 つの名前も、オブジェクトのキー（`{ async: 1 }`）とメンバーの名前（`o.await`）には使える（省略形の `{ async }` は変数なので、エラー）。
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
* **`host.call(name, ...args)`**：すぐ返るホストの関数（`__Host_call__`）。**`host.wait(name, ...args)`**：待つホストの関数（`__Host_start__`）。**`host.available(feature)`**：`__Feature_available__`。渡せるのは、数、文字列、それらの配列で、ホストが返した LList は配列、null は `undefined` になる（空の配列も `undefined` になる）。ホストにない関数と、関数の失敗は、ホストのメッセージを持つ `Error` を投げる。テストの関数（`test.add`、`test.echo`、`test.fail`、`test.later`）は、どの VM にもある（docs/PRIMITIVES.md の `__Host_call__` と `__Host_start__`）。
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
        * ふつうのファイルは待たずに読む。パイプや標準入力で、まだ届いていなければ、呼んだ処理だけが待つ。読む時間は行の数に比例する（Rust 版の VM で、1 行およそ 22 マイクロ秒）。
    * **`argv`**（段階 P8）：プログラムに渡した引数の配列（`conjs/conjs file.con.js a b` の `["a", "b"]`。Node.js の `process.argv.slice(2)`）。
    * **`fetch(url, options)`**：HTTP の要求。options は `method`（文字列）、`headers`（オブジェクト）、`body`（文字列）。返すのは、本体を全部読んである Response のようなオブジェクトで、`status`、`ok`（状態が 200〜299）、`text()`、`json()` を持つ（どれも待たない）。404 などは例外にせず `ok` を偽にし、つながらないときは TypeError（`fetch failed`）を投げる（JavaScript と同じ）。wasm 版の VM ではホストの `fetch`（Node.js 18 以降とブラウザ）を、Rust 版と Python 版の VM では `curl` を、待つための VProc で動かす（curl がなければ TypeError）。`json()` は `JSON.parse` と同じ読み手を使う。curl の道では、HEAD は `-X HEAD` でなく `--head` で送る（`-X HEAD` の curl は、来ない本体を待って失敗する）。
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

ConJS の構文は JavaScript の部分集合だが、ConJS のプログラムは JavaScript のプログラムではない（`spawn`、`join`、`host` は JavaScript になく、JavaScript では待つ呼び出しが Promise を返す）。そのため、多くの場合は sjs/test のように Node.js と出力を比べない。それらと待つ呼び出しのライブラリを使わないプログラムは JavaScript のプログラムでもあるので、conjs/test/compare.sh が Node.js と標準出力を比べる（各ステップの node.txt に並べた場合。2026-11-20 までは step10 だけ）。ConJS と SJS の両方が受け付ける、待つ呼び出しを使わないプログラムは、どちらでも同じ結果になる（同じ変換器とランタイムで動くため）。ConJS を広げるときは、まず ConJS に足し、そのうち安全なものを SJS に移して共有する（docs/dev/PLATFORM-PLAN-ja.md の 4.1 節）。

## 作り

* **include/lang/sched.se**：スケジューラ。ConJS の構文にも SJS の名前にもよらない部分で、P4 で言語キットに移す。処理の継続の列（FIFO）、待つ呼び出し（`__Host_start__` で始め、待つための小さな VProc がパイプを待って、結果をチャネルに入れる）、join、終わり。処理だけの状態は、受け取った save と restore の関数で保存して戻す。
* **conjs/include/conjs_rt.se**：SJS のランタイムとのつなぎ。処理だけの状態にする PDict の名前（`"sjs_handlers"`、`"sjs_gen"`、`"sjs_pos"`）を 1 か所にまとめ、`spawn`、`join`、`host` と、待つ呼び出しのライブラリ（conjs/include/conjs_lib.se）を置く。ライブラリの `sleep` と `Date.now` は、ホストの関数 `time.sleep` と `time.now`（機能 `"time"`。どの VM にもある）を呼び、`fetch` は `http.fetch`（機能 `"fetch"`。wasm 版の VM だけ）か curl を使う。curl は、スケジューラの `Kit_sched_wait_work`（待つための VProc で、ホストの呼び出しでない仕事をする）で動かす。
* **conjs/include/conjs_class.se**：クラス（段階 P5b）。クラスの値は SJS の関数で、インスタンスは SJS のオブジェクトで、どちらも隠したキー `"cls"` にクラスの記録（メソッド、static のメソッド、コンストラクタ、フィールド、親）を持つ。SJS のランタイムは、メンバーを読んで自分のプロパティになかったときだけ、`"cls"` を見る（sjs/include/sjs_rt.se の `Sjs_class_member`）。SJS の値は `"cls"` を持たないので、SJS の結果は変わらない。`JSON.parse` と配列の `sort` は、プロトタイプにもホストにも触れないので、SJS のランタイムに置いて SJS と共有する。
* **conjs/conjsc.se**：変換器の入口（言語の印を付けて、SJS の変換器を呼ぶ）。sjs_gen.se は、印があれば、文の並びを `Conjs_run` の中に置いたソースを作る。
* **ファイルの見本**（段階 P8。conjs/examples）：merge.con.js は、ソートした 2 つのファイルを 1 行ずつ読み比べ、片方だけにある行と両方にある行を出す（Unix の `comm` と同じ。`-` は標準入力）。2 つの `readLine` を、比べた結果で交互に進めるだけで書ける。同じことを Node.js で書いた merge.node.js では、`readline` の Interface を 2 つ作り、`for await` が使えないので、2 つの非同期のイテレータの `next()` を `await` で呼び、関数を `async` にし、`try`／`finally` で閉じる。2 つが同じ出力になることを、conjs/test/compare.sh が確かめる（conjs/test/node-pairs.txt）。wordcount.con.js は、`fs.readLines` で語を数える、いちばん短い使い方の例。
* **見本**：vm/wasm/web/canvas_demo.con.js（canvas の跳ねる玉。`canvas.html?conjs` で見る）と、vm/wasm/web/stars.con.js（入力を待つ対話のゲーム「星ひろい」。stars.html で遊ぶ。段階 P5c。端末でも `conjs/conjs vm/wasm/web/stars.con.js` で遊べる。段階 P6a）。ページの分は、vm/wasm/web/build.sh が変換する。

## テスト

conjs/test/stepN の `caseN.con.js` を変換して動かし、出力と終了ステータスを `caseN.log` と比べる（conjs/test/run_case.sh）。VM ごとに違うものは `caseN.wasm.log` のように置き、`caseN.host.js` があれば、wasm 版の VM の `CONMA_HOST` にする。

| ステップ | 確かめること |
|---|---|
| step1 | 動かす順：spawn の FIFO、`test.later` の締め切りの順、待っている間にほかの処理が動くこと |
| step2 | 協調的に切り替わること：共有のオブジェクトの読み書きが入り混じらない、待たないループに割り込まれない |
| step3 | 例外：join が位置ごと投げ直す、join されない例外の報告、main の例外ですぐ終わる、try の受け手と投げた位置が処理ごとに分かれる |
| step4 | 待つ所：map のコールバック、ジェネレータの本体、try と finally。待っている途中のジェネレータの next() |
| step5 | エラー：行き詰まり、自分自身の join、使い方の誤り、ホストの値とエラー、wasm 版の VM で Promise を返すホストの関数 |
| step6 | 受け付けない構文と、`then` をふつうの名前に、3 つの名前をキーとメンバーの名前に使えること |
| step7 | タイマーと時刻：`sleep` の間にほかの処理が動く、`Date.now()` の差、`setTimeout` と join、join されない例外、引数のエラー（本当の時計を使うので、負荷で揺れないことだけを確かめる） |
| step8 | ファイル：`fs.writeFile` と `fs.readFile`、読めないファイル、引数のエラー |
| step9 | fetch：GET、`json()`、404、POST と PUT とヘッダ、HEAD、2 つの処理で並べて待つ、つながらない所の TypeError、引数のエラー。step9/server.py の HTTP のサーバ（127.0.0.1）を使う |
| step10 | ConJS だけの名前を使わないプログラム（conjs/test/compare.sh が Node.js と比べる） |
| step11 | クラス：コンストラクタ、メソッド、フィールド、`static` とその継承、`extends` と `super`、`instanceof`、`in`、アロー関数の `this`、`console.log` と `Object.keys`。JavaScript との違い（case3） |
| step12 | エラーのコンストラクタと `instanceof`、`class ... extends Error`、受け手のないエラー、変換するときのエラー（`this`、計算した名前、getter）、実行時のエラー。コンストラクタが返すオブジェクト、エラーの `name` と `toString()`（case8） |
| step13 | `JSON.parse` と配列の `sort`（sjs/test の step12 と同じもの）。キーが `constructor` のオブジェクト（case2）。JSON でない文字列（case3） |
| step14 | 入力：`nextEvent` と `pollEvent`（case1。wasm 版の VM では、本物の vm/wasm/host/input.js を、入力のまね input_mock.js が手で進める時計で動かす。ほかの VM は機能がないことの TypeError）。見本の stars.con.js を、決まったキーで動かして終わり方を確かめる（case2）。VM ごとのログ（case*.wasm.log） |
| step15 | 端末の入力：標準入力のファイル（case*.in）からキーを読む（case1）。見本の stars.con.js を端末の道で動かし、Escape で終わる（case2）。ESC の並びが 2 回の読みに分かれる場合は、test/fast18 がキーに分ける関数を直に確かめる |
| step16 | ファイルを 1 行ずつ読む：`readLine`、`read`、`readLines` と、4,096 バイトの読みの区切りをまたぐ場合（case1）。書く、閉じる、エラー（case2）。`readLines()` を回し切る、`break`、例外、`return` で閉じること（case3）。`fs.stdin` と `fs.stdout`、キーと行の両方で読めないこと（case4）。見本の merge.con.js（case5。引数は case5.args）。少しずつ届く標準入力（case6.feed.py の出力をパイプで渡す）を、3 つの処理で取り合っても行が壊れないこと（case6） |

Node.js と比べる場合は、各ステップの node.txt に並べる（変換するときのエラーと、JavaScript との違いの場合は入れない）。step8 と step9（`copy.txt` のあるステップ）は、run_case.sh が一時ディレクトリに写してから動かす（ファイルを書くので、同じワークツリーで 3 つの VM のテストを同時に流しても混ざらない）。step9 は、写した所で server.py を立て（ポートは port.txt）、終わったら止める。step9 には python3 が、Rust 版と Python 版の VM では curl も要る。
