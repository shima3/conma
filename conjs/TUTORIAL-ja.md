[English](TUTORIAL.md) | **日本語**

# ConJS チュートリアル

ConJS で書ける JavaScript の基本的な書き方と、JavaScript と違うところを、例で説明します。JavaScript を少し知っている人を考えて書いていますが、初めての人も、例を打って動かしながら読めます。ConJS を動かす準備がまだなら、先に[クイックスタート](QUICKSTART-ja.md)を済ませてください。

例は、どれも `conma` フォルダの中でファイル（`例.con.js`）に書いて、次のように動かします。

```
$ conjs/conjs 例.con.js
```

例の下の `出力` が、出てくる文字です。

## 1. ConJS とは

ConJS は JavaScript のサブセット（一部分）です。ConJS で書いたプログラムの多くは、JavaScript としても同じに動きます。違うのは、大きく 2 つです。

* **間違いやすい書き方を、エラーにする**：`"年齢" + 20` のような、型の違う値の `+` や、`==` などは使えません。間違いを、動かしたときにすぐ見つけられます。
* **待つ処理を、ふつうの呼び出しで書く**：JavaScript では `async` と `await` を付けて書く「待つ処理」（時間を待つ、通信の答えを待つなど）を、ふつうの関数呼び出しとして書けます。`async`、`await`、`Promise` は使いません。

前半（2〜6）は JavaScript と同じ書き方、後半（7〜9）は JavaScript と違うところです。

## 2. 値と変数

```js
const name = "ConJS";
let count = 3;
count = count + 1;
console.log(`こんにちは、${name}！ count は ${count}`);
console.log(7 / 2, 7 % 2, 2 ** 10, Math.floor(3.7));
console.log(typeof name, typeof count, typeof true, typeof undefined);
```

出力：
```
こんにちは、ConJS！ count は 4
3.5 1 1024 3
string number boolean undefined
```

* 変数は `const`（変えない）か `let`（変える）で作ります。`var` も使えますが、`let` と `const` を使いましょう。
* 文字列の中に値を入れるときは、**テンプレート文字列**（`` ` `` で囲み、`${...}` で値を入れる）を使います。ConJS では、これが文字列をつなぐ方法です（7.1）。
* 数は、整数も小数も同じ number です。`Math` には `floor`、`ceil`、`round`、`abs`、`sqrt`、`max`、`min`、`PI` などがあります。

## 3. 条件と繰り返し

```js
for (let i = 1; i <= 15; i++) {
  if (i % 15 === 0) console.log("FizzBuzz");
  else if (i % 3 === 0) console.log("Fizz");
  else if (i % 5 === 0) console.log("Buzz");
  else console.log(i);
}
let n = 0;
while (n < 3) n++;
console.log(`n = ${n}`);
```

出力（はじめと終わり）：
```
1
2
Fizz
…
14
FizzBuzz
n = 3
```

`if`、`else`、`for`、`while`、`do ... while`、`switch`、`break`、`continue` は JavaScript と同じです。等しいかどうかは `===`、等しくないかは `!==` で比べます（7.2）。

## 4. 配列とオブジェクト

```js
const scores = [72, 95, 58, 81];
scores.push(66);
console.log(scores.length, scores[0], scores[scores.length - 1]);
const passed = scores.filter((s) => s >= 60);
const doubled = scores.map((s) => s * 2);
const total = scores.reduce((sum, s) => sum + s, 0);
console.log(passed, doubled.slice(0, 2), total);
console.log([...scores].sort((a, b) => a - b).join(", "));
for (const s of scores) if (s > 90) console.log(`高得点：${s}`);

const student = { name: "Hanako", year: 2, courses: ["数学", "英語"] };
student.year = 3;
console.log(student.name, student.courses[1], Object.keys(student));
const { name, year } = student;
console.log(`${name} は ${year} 年生`);
console.log(JSON.stringify(student));
```

出力：
```
5 72 66
[ 72, 95, 81, 66 ] [ 144, 190 ] 372
58, 66, 72, 81, 95
高得点：95
Hanako 英語 [ 'name', 'year', 'courses' ]
Hanako は 3 年生
{"name":"Hanako","year":3,"courses":["数学","英語"]}
```

* 配列の `push`、`map`、`filter`、`reduce`、`slice`、`sort`、`join`、`includes`、`indexOf` や、`for ... of` は JavaScript と同じです。
* `[...scores]` はスプレッド（中身を広げて新しい配列を作る）、`const { name, year } = student` は分割代入です。
* `JSON.stringify` と `JSON.parse` も使えます。
* JavaScript の新しいメソッドの中には、まだないものがあります（たとえば配列の `at`）。ないメソッドを呼ぶと、動かしたときに `TypeError: ... is not a function` になります。

## 5. 関数

```js
function area(w, h = w) {
  return w * h;
}
const square = (x) => x * x;
console.log(area(3, 4), area(5), square(9));

function sum(...xs) { return xs.reduce((a, b) => a + b, 0); }
console.log(sum(1, 2, 3, 4));

function makeCounter() {
  let c = 0;
  return () => ++c;
}
const next = makeCounter();
next(); next();
console.log(next());
```

出力：
```
12 25 81
10
3
```

`function` の宣言、アロー関数（`(x) => x * x`）、引数の既定値（`h = w`）、残りの引数（`...xs`）、クロージャ（`makeCounter` の中の `c` を覚えている関数）は、JavaScript と同じです。

## 6. クラスと例外

```js
class Point {
  constructor(x, y) { this.x = x; this.y = y; }
  length() { return Math.sqrt(this.x ** 2 + this.y ** 2); }
}
class Point3 extends Point {
  constructor(x, y, z) { super(x, y); this.z = z; }
}
const p = new Point(3, 4);
console.log(p, p.length());
console.log(new Point3(1, 2, 3) instanceof Point);

try {
  throw new RangeError("範囲の外");
} catch (e) {
  console.log(`${e.name}: ${e.message}`);
} finally {
  console.log("おわり");
}
```

出力：
```
Point { x: 3, y: 4 } 5
true
RangeError: 範囲の外
おわり
```

`class`、`constructor`、メソッド、`extends`、`super`、`new`、`instanceof`、`try`、`catch`、`finally`、`throw` は JavaScript と同じです。2〜6 の例は、Node.js でもそのまま動き、同じ出力になります（`node 例.con.js` で試せます）。

## 7. JavaScript と違うところ：エラーになる書き方

ConJS は、JavaScript では動いてしまう、間違いやすい書き方をエラーにします。エラーには 2 種類あります。

* **変換するときのエラー**（`conjs:` で始まる）：プログラムを動かす前に見つかります。
* **動かしたときのエラー**（`ファイル:行:桁:` で始まる）：その行を動かしたときに見つかります。

### 7.1. `+` は数どうしだけ

```js
const age = 20;
console.log("年齢：" + age);
```

出力：
```
例.con.js:2:13: TypeError: + needs numbers, not the string "年齢：" (join strings with a template literal)
```

JavaScript では `"年齢：20"` になりますが、`"1" + 2` が `"12"` になるような間違いのもとなので、ConJS では `+` は数どうしだけです。文字列をつなぐときは、テンプレート文字列を使います。

```js
console.log(`年齢：${age}`);
```

### 7.2. `==` と `!=` はない

```js
console.log(1 == 1);
```

出力：
```
conjs: not in ConJS: == (use ===)
```

`==` は、型の違う値を変換してから比べる（`0 == ""` が真になる）ので、ConJS にはありません。`===` と `!==` を使います。

### 7.3. 大小の比較は、数どうしか文字列どうし

```js
console.log(1 < "2");
```

出力：
```
例.con.js:1:13: TypeError: < compares two numbers or two strings, not the number 1 and the string "2"
```

### 7.4. ないもの

| 書き方 | ConJS では |
|---|---|
| 正規表現（`/-/g`） | `conjs: not in ConJS: a regular expression`。文字列のメソッド（`split`、`join`、`includes`、`indexOf`、`slice` など）を使う。置き換えは `s.split("-").join("+")` のように書ける（`replace` と `replaceAll` は、まだない） |
| `Set`、`Map` | `ReferenceError: Set is not defined`。オブジェクトや配列を使う |
| `this`（クラスの外） | `conjs: this outside a class`。`this` はクラスの中だけで使える |
| `Math.sin` など | 一部の `Math` の関数はまだない |
| `async`、`await`、`Promise` | `conjs: not in ConJS: async`。待つ処理は 8 の書き方で書く |

### 7.5. 文字列の長さは、文字の数

```js
console.log("ConJS".length, "日本語".length, "😀".length);
```

出力（ConJS）：
```
5 3 1
```

JavaScript（Node.js）では `5 3 2` です。JavaScript は UTF-16 の単位で数えるので、絵文字は 2 になりますが、ConJS は文字の数で数えます。

## 8. JavaScript と違うところ：待つ処理

ここが ConJS のいちばんの特長です。

### 8.1. `sleep`：時間を待つ

```js
console.log("3 秒数える");
for (let i = 3; i > 0; i--) {
  console.log(i);
  sleep(1000);
}
console.log("スタート！");
```

出力（1 秒ごとに 1 行）：
```
3 秒数える
3
2
1
スタート！
```

`sleep(ms)` は、ms ミリ秒待ってから戻ります。同じことを JavaScript で書くと、待つ関数を Promise で作り、`await` を付けて呼ぶ必要があります。

```js
// JavaScript（Node.js）の場合
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
console.log("3 秒数える");
for (let i = 3; i > 0; i--) {
  console.log(i);
  await sleep(1000);   // await を忘れると、待たずに先に進んでしまう
}
console.log("スタート！");
```

JavaScript では、`await` を使う関数はすべて `async` にし、その関数を呼ぶ所にも `await` を付ける必要があります。ConJS には `await` がないので、どの関数の中でも、ふつうに呼べば待ちます。

### 8.2. `spawn` と `join`：いくつかの処理を同時に動かす

```js
function worker(name, ms) {
  for (let i = 1; i <= 3; i++) {
    sleep(ms);
    console.log(`${name} ${i}`);
  }
  return `${name} おわり`;
}
const a = spawn(() => worker("はやい", 100));
const b = spawn(() => worker("おそい", 250));
console.log(join(a));
console.log(join(b));
```

出力：
```
はやい 1
はやい 2
おそい 1
はやい 3
はやい おわり
おそい 2
おそい 3
おそい おわり
```

* `spawn(f)` は、関数 f を新しい**処理**（軽量スレッド）として始め、処理を返します。呼んだ側は、そのまま先に進みます。
* `join(t)` は、処理 t が終わるまで待ち、f の戻り値を返します。
* ある処理が `sleep` などで待っている間は、ほかの処理が動きます。処理が切り替わるのは、待つ呼び出し（`sleep`、`join`、キーやファイルを待つ呼び出しなど）の所だけです。

JavaScript では、`worker` を `async function` にし、`sleep` の前に `await`、`join` の代わりに `await a` と書きます。

`setTimeout(f, ms)` もあります。JavaScript と違い、数ではなく処理を返すので、`join` で f の戻り値を受け取れます。

```js
const t = setTimeout(() => "2 秒たった", 2000);
console.log("待っている間も動く");
console.log(join(t));
```

出力：
```
待っている間も動く
2 秒たった
```

### 8.3. ファイルを 1 行ずつ読む

```js
fs.writeFile("memo.txt", "りんご\nみかん\nぶどう\n");
let n = 0;
for (const line of fs.readLines("memo.txt")) {
  n++;
  console.log(`${n}: ${line}`);
}
console.log(argv);
```

`conjs/conjs 例.con.js x y` で動かしたときの出力：
```
1: りんご
2: みかん
3: ぶどう
[ 'x', 'y' ]
```

* `fs` は、最初から使えます（`require` も `import` も要りません）。`fs.readFile`、`fs.writeFile`、`fs.readLines`、`fs.open` があります。
* 行は、ふつうの `for ... of` で回せます。Node.js では `readline` を使い、`for await` で回します。
* プログラムに渡した引数は `argv` にあります（Node.js の `process.argv.slice(2)`。ConJS には `process` がありません）。

Web サーバ（`http.listen`）、通信（`fetch`）、キーの入力（`nextEvent`）も、同じく待つ呼び出しとして書けます。[クイックスタート](QUICKSTART-ja.md)のサーバの見本や、[README](README-ja.md) を見てください。

## 9. ファイルを分ける：`import` と `export`

プログラムを複数のファイルに分けられます。書き方は JavaScript の ES Modules と同じです。

`lib/geometry.con.js`：
```js
export const PI = 3.14159;
export function circleArea(r) { return PI * r * r; }
```

`例.con.js`：
```js
import { circleArea, PI as pi } from "./lib/geometry.con.js";
console.log(pi, circleArea(2));
```

出力：
```
3.14159 12.56636
```

* 読み込む先は、`./` か `../` で始まり、拡張子（`.con.js`）まで書いたパスにします。
* 使えるのは、名前を付けた `import { ... }` と `export` です。`export default` と `import * as m` は、まだ使えません。

## 10. 練習

1. 3 の FizzBuzz を、1 から 30 まで、`for` の代わりに `while` で書いてみましょう。
2. 4 の `scores` の平均を、テンプレート文字列で「平均：○○ 点」と出してみましょう（`"平均：" + avg` はエラーになります）。
3. 8.2 の `worker` を 3 つ `spawn` し、それぞれ違う間隔で数えさせてみましょう。出る順を、動かす前に考えてから確かめましょう。
4. クイックスタートのサーバの見本に、`/hello` で `hello` と答える道を足してみましょう。

ConJS の言語とライブラリのすべては、[README](README-ja.md) にあります。
