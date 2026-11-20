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
```

変換はいつも Rust 版の VM で行い、変換したプログラムを選んだ VM で動かす。変換器は SJS のもの（sjs/include）に、言語の印（PDict の `"sjs_lang"` を `"conjs"`）を付けて使う。変換器のモジュールは、bin/cache-dir の場所に conjsc.module.se として作っておく（sjs/sjs と同じ）。

## 言語

SJS で書けるもの（sjs/README-ja.md の「書けるもの」）から、下を除き、下を足したもの。

* **除くもの**（変換するときに `conjs: not in ConJS: async` のようなエラー、終了ステータス 2）：`async`、`await`（`for await` も）、名前 `Promise`。SJS の範囲の外のもの（`this`、`class` など）も、「not in ConJS」と書く。`then` は、ふつうの名前として使える。3 つの名前も、オブジェクトのキー（`{ async: 1 }`）とメンバーの名前（`o.await`）には使える（省略形の `{ async }` は変数なので、エラー）。
* **`spawn(f)`**：引数のない関数 f を新しい処理として列の最後に入れ、処理（task）を返す。呼んだ側は、次に待つまで動き続ける。task は `console.log` で `[Task]`、`typeof` は `"object"`。
* **`join(t)`**：処理 t が終わるまで待ち、f の戻り値を返す。t が例外で終わっていれば、その例外を、t の中で投げた所の位置のまま投げ直す。何度でも、いくつの処理からでも join できる。自分自身の join は TypeError。
* **`host.call(name, ...args)`**：すぐ返るホストの関数（`__Host_call__`）。**`host.wait(name, ...args)`**：待つホストの関数（`__Host_start__`）。**`host.available(feature)`**：`__Feature_available__`。渡せるのは、数、文字列、それらの配列で、ホストが返した LList は配列、null は `undefined` になる（空の配列も `undefined` になる）。ホストにない関数と、関数の失敗は、ホストのメッセージを持つ `Error` を投げる。テストの関数（`test.add`、`test.echo`、`test.fail`、`test.later`）は、どの VM にもある（docs/PRIMITIVES.md の `__Host_call__` と `__Host_start__`）。
* **関数の色がない**：どの関数の中でも待てる（配列の `map` に渡した関数、ジェネレータの本体、try と finally の中）。
* **割り込まれない**：処理は、1 つの VProc の中で、待つ呼び出し（`host.wait` と `join`）の所でだけ切り替わる。2 つの待つ呼び出しの間のコードは、割り込まれずに動く。待たない長いループは、その間ほかの処理を止める（JavaScript と同じ）。
* **動かす順**：動かせる処理の列は FIFO。待つ呼び出しの結果が届いた処理も、列の最後に入る。結果は、動かせる処理がなくなったときに 1 つずつ受け取るので、spawn し続けて列が空にならないプログラムには、ホストの結果が届かない（Node.js でマイクロタスクを積み続けたときと同じ）。
* **終わり**：main（ファイルの文の並び）が受け手のない例外で終われば、ほかの処理を待たずに、すぐに終える（終了ステータス 2）。main が終わっても、残っている処理は動かし続ける。すべてが終わったら、join されないまま例外で終わった処理を、終わった順に `ファイル:行:桁: Uncaught (in task) メッセージ` で出し、終了ステータス 2 にする。join で待つ処理だけが残ったら、`conjs: Deadlock: tasks waiting in join: N` を出し、終了ステータス 2 にする。

ConJS の構文は JavaScript の部分集合だが、ConJS のプログラムは JavaScript のプログラムではない（`spawn`、`join`、`host` は JavaScript になく、JavaScript では待つ呼び出しが Promise を返す）。そのため、sjs/test のように Node.js と出力を比べない。待つ呼び出しを使わない ConJS のプログラムは SJS のプログラムでもあり、同じ結果になる。

## 作り

* **include/lang/sched.se**：スケジューラ。ConJS の構文にも SJS の名前にもよらない部分で、P4 で言語キットに移す。処理の継続の列（FIFO）、待つ呼び出し（`__Host_start__` で始め、待つための小さな VProc がパイプを待って、結果をチャネルに入れる）、join、終わり。処理だけの状態は、受け取った save と restore の関数で保存して戻す。
* **conjs/include/conjs_rt.se**：SJS のランタイムとのつなぎ。処理だけの状態にする PDict の名前（`"sjs_handlers"`、`"sjs_gen"`、`"sjs_pos"`）を 1 か所にまとめ、`spawn`、`join`、`host` を置く。
* **conjs/conjsc.se**：変換器の入口（言語の印を付けて、SJS の変換器を呼ぶ）。sjs_gen.se は、印があれば、文の並びを `Conjs_run` の中に置いたソースを作る。
* **見本**：vm/wasm/web/canvas_demo.con.js（canvas の跳ねる玉。`canvas.html?conjs` で見る。vm/wasm/web/build.sh が変換する）。

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
