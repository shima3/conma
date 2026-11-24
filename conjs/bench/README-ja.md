# ConJS のベンチマーク

ConJS のプログラムを、Node.js と比べる（2026-11-24 から）。ConJS は SJS と変換器とランタイムを共有するので、数の計算と関数の呼び出しは sjs/bench（loop、fib、nbody。sjs/README-ja.md）でも測っている。ここでは、ConJS で書くプログラムによく出るもの（文字列、辞書、高階関数、JSON、例外、タスク、クラス）と、起動の時間を測る。ここのすべてを測ることを **Full ConJS Bench** と呼ぶ（docs/dev/CONTRIBUTING.md の Benchmarks）。

## 測り方

```
conjs/bench/run.sh 名前 [N [M]]
```

* 同じプログラムを、Node.js と、`conjs/conjs`（Rust 版の VM と、wasm 版の VM。`CONMA_VM=wasm`）で動かし、時間（user + sys、3 回の最良）を出す。wasm 版の VM は vm/wasm/build.sh で作っておく。
* プログラムの `N_ARG` を大きさに置き換える。Node.js は N、VM の 2 つ（表の `*`）は M で測る（Node.js が約 1 秒、遅い方が 1 分を超えないように。CONTRIBUTING の Benchmarks の 1）。N と M を書かなければ、run.sh の中の名前ごとの既定。
* Node.js では、`名前.node.js` があればそれを、なければ `名前.con.js` そのものを動かす（`spawn` と `join` を使う tasks だけ、`async` 関数と `await` で書いた tasks.node.js がある）。大きさ M で、VM の出力が Node.js と同じことも確かめる。
* ConJS の時間は、`conjs/conjs` の全体（プログラムの変換、ランタイムのモジュールとの link、実行）。変換器とランタイムのモジュールは、測る前に作っておく（`conjs/conjs --prepare`）。起動の時間は startup で分かる（Rust 版で約 0.06 秒）。

## プログラム

| 名前 | 測るもの | 1 回の中身 |
|---|---|---|
| startup | 起動の時間 | `console.log("hi")` だけ（大きさなし） |
| strings | 文字列のメソッド | テンプレート リテラルで 1 行を作り、`split`、`slice`、`indexOf`、`join`、`replaceAll`、`includes` |
| dict | オブジェクトを辞書にする | 3 音節の語（512 種）を作って数える。最後に、`Object.keys` を数で `sort` して上の 5 つ |
| hof | 配列の高階関数 | 100 個の数の配列に `map`、`filter`、比べる関数を渡した `sort`、`reduce` |
| json | JSON | 20 個のオブジェクトの配列を持つオブジェクトを `JSON.stringify` して `JSON.parse` |
| exceptions | 例外 | 関数の中で、3 回に 1 回、`Error` を継いだクラスの例外を `throw` し、`try`、`catch`（`instanceof`）、`finally` |
| tasks | タスク | 10 個のタスクを `spawn` して `join` する |
| nbody_class | クラスのメソッド | sjs/bench の nbody の物体をクラスにしたもの（段階 P5b。docs/dev/PLATFORM-PLAN-ja.md の 8 節） |

## 結果

2026-11-24、a102031（alpha）、Apple M4（M4）。Node.js v26.13.0、rustc 1.99.1。user + sys、3 回の最良（起動を含む）。測っている間、ほかのセッションも動いていた（load average 約 2）。

| 名前 | N | Node.js | M | Rust 版 * | wasm 版 * | 1 回の時間（Rust 版、wasm 版） | Node.js の何倍（Rust 版、wasm 版） |
|---|---:|---:|---:|---:|---:|---|---|
| startup | — | 0.039 s | — | 0.06 s | 0.141 s | — | 1.5、3.6 |
| strings | 3,000,000 | 1.002 s | 180,000 | 21.0 s | 29.5 s | 117 µs、164 µs | 349、490 |
| dict | 15,000,000 | 0.966 s | 600,000 | 20.8 s | 29.4 s | 35 µs、49 µs | 538、762 |
| hof | 350,000 | 1.036 s | 6,000 | 23.4 s | 32.6 s | 3.9 ms、5.4 ms | 1,319、1,838 |
| json | 200,000 | 1.031 s | 4,500 | 20.4 s | 29.0 s | 4.5 ms、6.5 ms | 881、1,251 |
| exceptions | 2,000,000 | 0.959 s | 1,200,000 | 22.0 s | 30.4 s | 18 µs、25 µs | 38、53 |
| tasks | 1,600,000 | 0.974 s | 60,000 | 22.4 s | 30.8 s | 373 µs、514 µs | 613、844 |
| nbody_class | 20,000,000 | 1.084 s | 27,000 | 19.2 s | 27.1 s | 711 µs、1.0 ms | 13,125、18,511 |

* 「Node.js の何倍」は、1 回あたりの時間の比（VM の時間 ÷ M を、Node.js の時間 ÷ N で割る）。VM の時間には起動（約 0.06 秒と 0.14 秒）が入るが、20 秒の中では小さい。
* wasm 版は、どれも Rust 版の約 1.4 倍の時間。
* 例外は、Node.js の例外が重いので（スタックの記録を作る）、差が小さい。nbody_class は、Node.js が数とフィールドの読み書きを機械語にするので、差が大きい。
