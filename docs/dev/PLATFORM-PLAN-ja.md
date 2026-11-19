# いろいろなプラットフォームと、ConMa に変換する言語の計画

ConMa を、いろいろなプラットフォームで、そのプラットフォームに固有の機能（ブラウザの canvas、OS のファイル、モバイルの通知など）も使って動かせるようにする。また、実験的な言語を ConMa に変換すれば、その言語を、ConMa が動くどのプラットフォームでも試せるようにする（2026-11-19）。

**状態**：段階 P1 と P2 を作った（2026-11-19。8 節）。6 節の 1〜3 を決めた（P3 はブラウザの canvas と、待つホストの呼び出しの形から。P5 の見本の言語は代数的エフェクトとハンドラの小さな言語。SJS の次の段として ConJS を作る。4.1 節）。6 節の 4 と 5 は、まだ決めていない。M2 のレビュー（気付いたことと、6 節への案）は、[docs/dev/REVIEW-ja.md](REVIEW-ja.md) の 2026-11-19 の節にある（P1 で答えたことは、その「対応」に書いた）。

## 1. 目的

* **A. プラットフォーム**：同じ ConMa のプログラムが、どの VM とプラットフォームでも同じに動く（今のコア ConMa と拡張 ConMa）うえに、プラットフォームに固有の機能も使える。使えない所では、プログラムがそれを知って、別の道を選べる。
* **B. ConMa に変換する言語**：新しい言語を、変換器と実行時の支えを ConMa で書いて作れば、それだけで ConMa のすべてのプラットフォームで動き、A の固有の機能も使える。SJS（sjs/README-ja.md）がその最初の例で、その作りを、次の言語を作る道具一式（言語キット）にする。

## 2. 今あるもの

### 2.1. VM とプラットフォーム

| VM | 動く所 | 固有の機能 |
|---|---|---|
| Python 版（vm/python、基準） | CPython のある所 | OS（`__OS_*`。ファイル、プロセス、パイプ） |
| Rust 版（vm/） | macOS、Linux など、Rust の動く所 | OS |
| transpiled VM（vm/conma2rust） | Rust 版と同じ | OS。変換したモジュールが速い |
| wasm 版（vm/wasm） | Node.js、JavaScriptCore、ブラウザ（consh/js/web。Worker でもページでも） | JavaScript（`__JS_call__`、`__JS_value__`）、WebAssembly（機能 `"wasm"`）、ホストの仮想ファイル |

* wasm 版の VM は、ホストの関数を import して呼ぶ（vm/wasm/src/sys.rs の `host_write`、`host_js_call`、`host_wasm_compile` など）。どれも、そのために作った専用の口である。
* `__JS_call__` は、どの VM にもあるが、ネイティブの VM では `"__JS_call__: JavaScript is not available"` で onError に行く。JavaScript のコードを文字列で渡し、結果を S 式の行で受け取るので、1 回の呼び出しが重い（パイプ、Worker とページの間のメッセージ、文字列と S 式への変換）。

### 2.2. 拡張 ConMa の機能

docs/PRIMITIVES.md の 16 節（docs/dev/FAST-PLAN-ja.md）。モジュールが機能のプリミティブ（`"array"`、`"bitwise32"`、`"boolean"`、`"control"`、`"chan"`、`"wasm"`）を使うと、フロントエンドが `(Uses "機能")` を付ける。その機能を持つ VM はプリミティブで動かし、持たない VM（Python 版）には、`bin/to-core` がコア ConMa の代わりの定義（include/core）を入れる。

* つまり、「機能に名前を付け、モジュールが使う機能を記し、VM が持たない機能は代わりの定義で補う」仕組みが、もうある。`"wasm"` の代わりの定義は、計算をまねず、「使えない」と onError に行く。プラットフォームの機能も、初めはこの形に載せるつもりだったが、P1 で、言語の層とは別の軸（`(Platform ...)`。どの VM もプリミティブを持つ）にした（3.1 節、8.1 節）。

### 2.3. SJS

sjs/ に、JavaScript の安全なサブセットを ConMa に変換する変換器（sjs/include の `sjs_lex`、`sjs_parse`、`sjs_gen`、`sjs_wasm`）と、実行時の支え（`sjs_rt`、`sjs_bits`、`sjs_inspect`）がある。どれも ConMa で書いてあり、ConSh の `(( ))` も使う。

* 変換したコードに SInfo で SJS の行と桁を付けるので、エラーは SJS の位置を示す。
* テストは Node.js と出力を比べ（sjs/test）、ベンチマークは Node.js、CPython、手で書いた ConMa と比べる（sjs/bench）。
* 動かす道具（sjs/sjs）は、変換器のモジュールを作ってキャッシュし（bin/cache-dir）、変換して動かす。
* 数だけの関数は、wasm 版の VM では WebAssembly にして動かす（docs/consh/dev/SJS-WASM-PLAN-ja.md）。プラットフォームの機能を、言語の側で使い分けている例でもある。

## 3. 目標 A：プラットフォームの機能

### 3.1. 3 つの層

| 層 | 例 | どこで動くか | 持たない VM では |
|---|---|---|---|
| コア ConMa | docs/SPEC.md、docs/PRIMITIVES.md の 1〜15 節 | すべての VM | — |
| 拡張の機能 | `"array"`、`"chan"`、`"bitwise32"` | すべての VM | 代わりの定義で、同じ意味に動く |
| プラットフォームの機能（新しく足す） | `"js"`、`"canvas"`、`"wasi"`、`"android"` など | その機能を持つプラットフォームだけ | どの VM もプリミティブを持ち、「使えない」と onError に行く |

* プラットフォームの機能も、名前を付け、モジュールに `(Platform "機能")` を付ける（`(Uses ...)` の後）。どの機能を使うプログラムかが、モジュールを見れば分かる。初めは `(Uses "機能")` に入れるつもりだったが、P1 で別の印にした（8 節）。
* プログラムが、使う前に機能の有無を調べられるようにする（`__Feature_available__ 名前 ,(Boolean)`。P1 で作った。8.1 節）。ブラウザなら canvas に描き、ほかでは文字で出す、といった使い分けができる。
* 今の `__JS_call__`、`__JS_value__` は、機能 `"js"` としてこの層に移す（使い方は変えない）。

### 3.2. ホストを呼ぶ共通の口

固有の機能ごとに VM にプリミティブを足すと、機能が増えるたびに、すべての VM を変えることになる。そこで、ホストの関数を呼ぶ共通の口を 1 つ作り、固有の機能は、ホストの側に登録する関数と、それを包む ConMa のライブラリで作る。

* **案**：`__Host_call__ onError 名前 引数 ,(結果)`。名前は `"canvas.lineTo"` のような文字列。引数と結果は、数、文字列、真偽値、null、それらの LList（と、必要なら Dict）で、そのまま渡す（文字列や S 式にしない）。ホストに名前の関数がなければ onError に行く。
* **ホストの側**：
    * wasm 版：import の 1 つ（`host_call`）が、JavaScript の側に登録した関数を名前で呼ぶ。値は、VM のメモリの上の決まった形で渡す。
    * Rust 版と transpiled VM：Rust の関数の表に登録する（VM に組み込むものと、組み込む人が足すもの）。
    * Python 版：Python の関数の表。テストのためのホストの関数（決まった値を返す）も、ここと Rust 版に同じものを置き、トレースを比べられるようにする。
* **待つ呼び出し**：すぐ終わらないもの（ファイルを読む、ネットワーク、タイマー）は、今の `__JS_call__` と同じく Stream で結果を返し、VProc が待つ間、ほかの VProc が動く。すぐ終わるもの（canvas に描く）は、待たずに値を返す。
* **ConMa のライブラリ**：プラットフォームごとに、`__Host_call__` を包む関数を置く（たとえば `include/platform/browser/canvas.se`）。プログラムは、それを include して使う。
* **安全**：ホストの関数は、そのプラットフォームでできることを何でもできる。ホストが登録する関数を選び、ConSh の `CONSH_ENABLE_JS` のように、使ってよい機能を利用者が決める。
* **速さ**：1 回の呼び出しが、wasm から JavaScript への呼び出し 1 回分になる。ブラウザで点や線をたくさん描くとき、今の `js` コマンド（1 回ごとにメッセージの往復）よりずっと速くなる見込み。

### 3.3. プラットフォームの候補

| プラットフォーム | VM | 主な固有の機能 | 手間 |
|---|---|---|---|
| ブラウザ | wasm 版 | canvas、DOM、イベント（キー、マウス）、fetch、音 | 小さい：wasm 版と consh/js/web がある |
| Node.js、JavaScriptCore | wasm 版 | ファイル、ネットワーク、npm のライブラリ | 小さい |
| WASI（wasmtime など） | wasm 版を WASI 向けに作る | ファイル、時刻、環境変数。サーバーやエッジ | 中くらい：ホストの口を WASI に替える |
| デスクトップ（macOS、Linux、Windows） | Rust 版、transpiled VM | OS、GUI（ウィンドウと描画のライブラリを Rust で包む） | 中くらい：Windows の OS の部分（パイプ、プロセス）を作る |
| モバイル（iOS、Android） | Rust 版を組み込む | 画面、通知、センサー | 大きい：アプリの殻と、プラットフォームの言語との橋が要る |

### 3.4. 段階 P3 の設計（案、2026-11-19。作る前に決める）

**待つホストの呼び出し**（6 節の 3）

* **プリミティブ**：
    * `__Host_start__ onError name args ,(inputStream)`：ホストの関数 name を args で呼び始め、パイプの読む側を返す。関数が終わると、ホストが結果を VM に渡し、VM がパイプの書く側を閉じる。値の種類と名前は `__Host_call__` と同じ。
    * `__Host_result__ onError inputStream ,(result)`：終わった呼び出しの結果。関数が失敗したら、そのメッセージで onError に行く。まだ終わっていなければ onError に行く。結果を取ると、Stream を閉じる。
    * 待つのは、今ある `__OS_wait_readable__ onError inputStream ,()`（パイプが EOF になると読める）。包むライブラリ（下）が、3 つを 1 つの関数にする：`Host_wait onError name args ,(result)`。
    * `__Host_start_tracked__ onError name args Ref ,(inputStream)`：`__Host_start__` と同じで、同じステップの中で、Stream を Ref の LList の前に入れる（`__OS_open_tracked__` と同じ形）。待っている VProc が殺されても、記録した Stream を閉じられる（M2 の PR のコメントの 3）。閉じた Stream の呼び出しが後で終わったら、VM は結果を捨てる。`Host_wait` は、こちらを使う。
    * プラットフォームの印：`__Host_start__`、`__Host_start_tracked__`、`__Host_result__` も、`__Host_call__` と同じく `(Platform "host")` にする（bin/resolver、include/resolver.se、reference/resolver.se）。transpiled VM は、この 3 つを VM に任せる（`__Host_call__` と同じ）。
* **パイプにする理由**：`__JS_call__` と同じ仕組みなので、スケジューラは「外を待っている」と見なし、wasm 版の VM は Idle で返って、ホストの Promise が解決するのを待てる。待つ VProc のほかは動き続ける。Ref の変化を待つ形（`__Ref_wait_change__`）では、すべての VProc が待つとき、スケジューラが行き詰まり（Deadlock）と報告するので、その扱いを足すことになる。
* **wasm 版のホスト**：`hostFunctions` の関数が Promise を返したら（`__Host_start__` のときだけ受け付ける。`__Host_call__` では今までどおりエラー）、解決したときに、VM の新しい export（`conma_host_done(job, 結果の JSON)`）で結果を渡す。VM は、その job のパイプの書く側を閉じる。ホストは、その後 VM をもう一度動かす（`jsDone` と同じ形のフック）。
* **テストの関数**：どの VM にもあるテストの関数（機能 `"test"`）を、`__Host_start__` でも呼べる。その場で呼んで結果を取っておき、書く側をすぐ閉じる（読めるようになっている）。待つ関数 `test.later`（引数を、そのまま LList で返す。最初の引数は待つ数）を足す。
* **終わる順を決める**（M2 の PR のコメントの 2）：`test.later` の終わる順は、どの VM でも同じにする。ConJS の動かす順のテスト（4.1 節）のログを、VM によらず同じにするため。
    * ネイティブの VM と Python 版の VM：`test.later` の呼び出しは、すぐには終わらせずに持っておく。動かせる VProc がなくなったとき（スケジューラが外を待つ所）に、待つ数がいちばん小さいもの（同じ数なら呼んだ順）を 1 つだけ終わらせる（書く側を閉じる）。そのあと、また動かせる VProc がなくなるまで動かす。
    * wasm 版の VM：`test.later` は、待つ数だけマイクロタスクを待ってから終わる。終わった呼び出しの結果は、conma.js が列に入れ、VM が Idle で返るたびに 1 つずつ渡す。待つ数と呼んだ順で、ネイティブの VM と同じ順になる。
    * 待つことが本当に並行に進むのは、wasm 版の VM の本物のホストの関数（fetch など）のときで、そのときの順はホストが決める（4.1 節の「決定的でないのはそこだけ」）。

**canvas**（6 節の 1）

* **機能の名前**：`"canvas"`。docs/PRIMITIVES.md の `__Feature_available__` の一覧に足す。
* **ホストの関数**（JavaScript。新しい vm/wasm/host/canvas.js が、canvas か OffscreenCanvas を受け取り、`hostFunctions` と `hostFeatures` を作る）：
    * すぐ返るもの：`canvas.size`（幅と高さ）、`canvas.clear`、`canvas.fill_style`、`canvas.stroke_style`、`canvas.line_width`、`canvas.font`、`canvas.fill_rect`、`canvas.stroke_rect`、`canvas.begin_path`、`canvas.move_to`、`canvas.line_to`、`canvas.arc`、`canvas.close_path`、`canvas.fill`、`canvas.stroke`、`canvas.fill_text`。
    * まとめて描くもの：`canvas.draw`（上の命令のリスト。`("fill_rect" x y w h)` などを順に。1 回の呼び出しで何百もの命令を描く）。
    * 待つもの：`canvas.frame`（次のアニメーションの 1 コマ、`requestAnimationFrame` を待ち、その時刻を返す）。Node.js には `requestAnimationFrame` がないので、命令を記録する canvas のまね（下のテスト）は、決まった時刻（コマの番号 × 16 ミリ秒）を返す。テストのログには時刻を出さない（M2 の PR のコメントの 3）。
* **ConMa のライブラリ**（新しい include/platform/canvas.se。`(include "platform/canvas.se")`）：上の関数を包む ConMa の関数（`Canvas_fill_rect onError x y w h ,()` など）と `Host_wait`（include/platform/host.se）。包むライブラリが、自分の機能の名前（`"canvas"`）を `__Feature_available__` で確かめる関数（`Canvas_available ,(Boolean)`）も持つ（M2 のレビュー（2026-11-19）の 1：機能の印は、包むライブラリが示す）。
* **見本のページ**：ConMa のプログラムが canvas にアニメーションを描くページ（vm/wasm/web/canvas.html と、その ConMa のソース）。ページの中で VM を動かす（Worker と OffscreenCanvas は、後で）。consh/js/serve.py で見て、ブラウザで動くことを確かめる。
* **測ること**：線を 1 本描く 1 回の時間を、`__Host_call__` 1 回ずつ、`canvas.draw` でまとめたとき、今の `js` コマンド（`__JS_call__`）の 3 つで比べる。
* **テスト**：
    * 待つ呼び出し：test/fast15（`__Host_start__`、`__Host_result__`、`__OS_wait_readable__` と `test.later`、エラー。どの VM でも同じログ。wasm 版の VM では、`CONMA_HOST` の JavaScript の関数が Promise を返す場合も）。
    * canvas のライブラリ：canvas のない VM で、`Canvas_available` が偽で、描く関数が onError に行くこと。wasm 版の VM（Node.js）では、`CONMA_HOST` に、命令を記録するだけの canvas のまね（test の中の JavaScript）を渡し、描いた命令の列を確かめる。
    * 名前の一覧：見本と test が使う `canvas.` の名前が、docs/PRIMITIVES.md の一覧にあること（M2 のレビュー（2026-11-19（2））の 3）。

## 4. 目標 B：ConMa に変換する言語の道具（言語キット）

SJS の作りから、言語によらない部品を取り出し、新しい言語を少ない手間で作れるようにする。

* **字句解析**：sjs_lex から、行と桁を持つ字句、空白とコメント、数と文字列のリテラルの部品を取り出す。
* **構文解析**：sjs_parse の再帰下降と演算子の優先順位の部品（節は `"t"` を持つ Dict）。
* **ConMa のコードを作る**：sjs_gen から、関数、継続、変数、SInfo（元の言語の行と桁）を書く部品。今は ConMa のソースの文字列を作ってからモジュールにするが、構文木から直接モジュールを作ると速いかもしれない（測ってから決める）。
* **実行時の支え**：値の種類、エラーとその位置、例外の型。
* **動かす道具**：sjs/sjs を一般にしたもの（案：`bin/lang 言語 ファイル`）。変換器のモジュールを作ってキャッシュし、変換して、選んだ VM で動かす。
* **テストとベンチマークの型**：本物の処理系がある言語なら、その出力と比べる（sjs/test の compare.sh の形）。ベンチマークは、bench/ と同じ計算で、ほかの言語と比べる。
* **置き場所**：言語ごとに、sjs/ と同じ形のディレクトリ（`言語/include`、`言語/test`、`言語/bench`、`言語/README-ja.md`）を、リポジトリのルートに置く。言語キットの部品は、標準ライブラリ（include/）か、それとは別のディレクトリに置く（6 節で決める）。
* **ConSh から使う**：SJS の `(( ))` と同じように、ConSh の中から別の言語の式や文を書ける形も、言語ごとに選べる。
* **見本の言語**：キットの部品を確かめるために、小さな言語を 1 つ作る（代数的エフェクトとハンドラの小さな言語。6 節の 2）。キットで作った見本と SJS の両方が動けば、部品が言語によらないことを確かめられる。

### 4.1. ConJS（SJS の次の段）

**ConJS** は、SJS（JavaScript の安全なサブセット）に暗黙の await を加えた言語である。待つ呼び出し（fetch、タイマー、ファイル、アニメーションの 1 コマ）まで、`await` を書かずに、ふつうの呼び出しとして書ける（`const text = fetch(url).text()`）。待っている間は、その処理だけが止まり、ほかの処理は動く。用語の「直接スタイル」は、ここでだけ「待つ呼び出しまで直接スタイルで書ける」の意味で使う。

* **ConMa で作る甲斐**：JavaScript では、待つ処理が、呼び出す側に `async` と `await` を広げていく（関数の色）。ConMa は CPS で、どの呼び出しの途中でも継続を持つので、待つ処理を「待つ」ことにするだけで、関数に色が付かない（ConSh のパイプと `$( )` が、もうこの形で動いている）。Go のゴルーチンや Java の仮想スレッドの考え方を、JavaScript の構文で、ConMa の動くどのプラットフォームでも試せる。ふつうの JavaScript ではできず、ほかの言語の上に作るのも難しい。
* **協調的に切り替える**（決めた、2026-11-19。M2 の意見の 3）：JavaScript では、2 つの `await` の間のコードは、割り込まれずに最後まで動き（run-to-completion）、プログラムはそれを頼りにオブジェクトを共有する。ConMa の VProc はステップの数で切り替わるので、処理ごとに VProc を使うと、どこでも入り混じる。そこで、ConJS の処理（並べて動かすものも）は、すべて 1 つの VProc の中で動かし、実行時の支えが継続で切り替える。切り替わるのは、待つ呼び出しの所だけである。待つこと自体は、待つためだけの小さな VProc に任せ、その VProc が結果をチャネルに入れる。実行時の支えは、動かせる処理がなくなったときだけ、そのチャネルを待つ。待つことは並行に進み、ConJS の処理は協調的に切り替わる。VM とスケジューラは変えない。
* **使うもの**：字句解析、構文解析、実行時の支え（数、文字列、配列、オブジェクト、例外）の多くは SJS のもの。待つ呼び出しは、P3 で決めるホストの待つ呼び出し（6 節の 3）。
* **動かす順**（M2 の意見（docs/dev/REVIEW-ja.md の 2026-11-19（4））の 1）：動かせる処理の列は FIFO にする。`spawn` した処理は列の最後に入り、呼んだ側が次に待つまで動き続ける（JavaScript のマイクロタスクと同じ）。待つ呼び出しの結果が届いた処理も、列の最後に入る（いくつか届いていれば、チャネルに届いた順）。これで ConJS のプログラムの動きは、ホストが結果を返す順だけで決まる。決定的でないのは、待つための小さな VProc がチャネルに入れる順だけなので、テストではテストのホストの関数（`test.*`）で返す順を決める。
* **Promise**（決めた、2026-11-19。M2 の意見の 2）：`Promise`、`async`、`.then` は受け付けず、エラーにする。並べて待つには `spawn` と `join` を使う。Promise を受け付けると、`await Promise.all([fetch(a), fetch(b)])` が、エラーにならずに 2 つの fetch を順に待ち、遅くなるだけで気付きにくい。`await` だけは、SJS との行き来のために「その場で待つ」と読んでもよいが、これも P3b で決める。
* **spawn した処理の例外**（M2 の意見の 3）：`join` で、その例外をもう一度投げる。`join` されないまま例外で終わった処理があれば、プログラムの終わりに報告する（JavaScript の unhandled rejection のように）。P3b のテストに、spawn した処理の例外の場合（join する場合としない場合）を入れる。
* **スケジューラを部品にする**（M2 の意見の 4）：ConJS の切り替え（待つ呼び出しで継続を取り、列に入れ、ほかの処理を動かす）は、「待つ」というエフェクトのハンドラそのものである。P3b では ConJS の中に作るが、継続の列とチャネルを待つことを、ConJS の構文によらない部分として分けておき、P4 で言語キットの部品として取り出す。P5 の見本の言語で、このスケジューラをハンドラとして書けることを確かめる。
* **これから決めること**：並べて動かす書き方の細部（`spawn(() => ...)` と終わりを待つ `join`、またはチャネル）。JavaScript との関係（構文は部分集合でも、意味は違う。JavaScript では Promise を返す呼び出しが、ConJS では結果を返すので、同じソースが Node.js で同じに動くとは言えない）。置き場所（conjs/ など）。

## 5. 段階

| 段階 | すること | 終わりとすること |
|---|---|---|
| P0 | この計画書。6 節を決める | 決めたことを、この文書に書く |
| P1（作った、2026-11-19。8 節） | プラットフォームの機能の層：`(Platform "機能")` の付け方、`__Feature_available__`。`__JS_call__` と `__JS_value__` を機能 `"js"` にする | test/ と consh/test がすべての VM で通る。機能の有無で振る舞いを変えるテスト |
| P2（作った、2026-11-19。8.2 節） | `__Host_call__`：wasm 版（JavaScript）、Rust 版と transpiled VM（Rust の表）、Python 版（Python の表）。テストのためのホストの関数 | 同じテストのトレースが Python 版と Rust 版で同じ。wasm 版の Node.js で同じ出力 |
| P3 | 待つホストの呼び出しの形を決めて作る（6 節の 3）。最初のプラットフォームの機能（ブラウザの canvas。描く見本のページ） | 見本が動く。1 回の呼び出しの時間を、今の `js` コマンドと比べて測る。足した機能の名前を docs/PRIMITIVES.md の `__Feature_available__` の一覧に書き、テストで、見本が使う名前が一覧にあることを確かめる（知らない名前は黙って偽になり、書き間違いと区別できないため） |
| P3b | ConJS（4.1 節）：SJS の部品で作る。P3 の待つ呼び出しを使う | ConJS のテストが、すべての VM で通る。協調的に切り替わること（待つ呼び出しの所だけで切り替わる）を確かめるテスト |
| P4 | 言語キット：SJS から部品を取り出し、SJS と ConJS をキットの上に書き直す | sjs/test、consh/test、sjs/bench（遅くならないこと） |
| P5 | 見本の言語（代数的エフェクトとハンドラの小さな言語。6 節の 2）を、キットで作る | その言語のテストが、すべての VM で通る。P3 の機能を使う見本 |
| P6 | プラットフォームを増やす（6 節の順に） | そのプラットフォームでテストが通る |

## 6. 決めること

1. **プラットフォームの順**（決めた、2026-11-19）：P3 は、ブラウザの canvas から始める。手間が小さく、目に見えて確かめやすい。P6 の順は、P3 の後で決める。
2. **見本の言語**（決めた、2026-11-19）：P5 の見本の言語は、**代数的エフェクトとハンドラの小さな言語**（Koka や Eff のような）にする。ハンドラは、捕まえた継続を値として扱うだけ（`__Cont_get__`）で、ConMa で作りやすい。構文を自分で決められるので、SJS と違う構文で、キットが言語によらないこと（4 節）も確かめられる。
    * M1 は、初め「待つ処理を直接スタイルで書ける、JavaScript の部分集合」を見本の言語に勧めた（aae726a）。M2 の意見（docs/dev/REVIEW-ja.md の 2026-11-19（3））を受けて、名前を **ConJS** にし（「直接スタイル」は、ふつうの JavaScript もそうなので区別できない。「await なしの JS」は非同期ができないように聞こえる。ConMa、ConSh と並ぶ名前にした）、見本の言語ではなく、**SJS の次の段**として作る（4.1 節、段階 P3b）。JavaScript の構文なので、キットが構文によらないことの確かめにならないため。
    * 比べた候補：Lua のサブセット（M2 の初めの案。キットの確かめにはなるが、C の処理系がどこでも動くので、ConMa に変換する得は小さい）、小さな Lisp（S 式で、キットの字句解析と構文解析をほとんど確かめない）、Forth のようなスタックの言語、アクターやチャネルの並行言語、論理型言語の小さなもの。
3. **ホストを呼ぶ口の形**：すぐ返る呼び出しは P2 で作った（8.2 節。Dict はまだ渡さない）。**待つ呼び出しの形は、P3 の初めに決める**（決めた、2026-11-19）。canvas のアニメーション（`requestAnimationFrame` を待つ）と ConJS（4.1 節）の両方に要るので、見本の言語より先に決める（M2 の意見の 4）。
4. **言語キットの置き場所**：標準ライブラリ（include/）に入れるか、別のディレクトリにするか。
5. **言語のディレクトリの置き場所**：sjs/ と同じくルートに並べるか、`langs/` のような 1 つのディレクトリの下にまとめるか（まとめるなら sjs/ も移す）。

## 7. 気を付けること

* **トレースとプロファイルの道**：`__Feature_available__ "wasm"` は、wasm 版の VM でも、`--trace` と `CONMA_PROFILE` のときは偽になる（`__Wasm_call__` が WebAssembly を使わないのと揃えた）。これで道を分けるプログラムは、トレースやプロファイルを取ると、ふだんと違う道（WebAssembly を使わない道）を通る。デバッグやプロファイルで見ているのが、ふだんの道でないことがある。
* **トレースの一致**：Python 版と Rust 版の VM はステップごとに同じトレースを出し、テストがそれを比べる。ホストの呼び出しは外の世界に触れるので、テストでは、どの VM にも同じ、決まった値を返すホストの関数を使う。
* **コア ConMa を小さく保つ**：プラットフォームの機能は、コア ConMa にも拡張の機能にも入れない。持たない VM では「使えない」になるだけで、言語の意味は変えない。
* **変換したコード**：transpiled VM は、`__Host_call__` を、VM に任せる呼び出し（変換したコードから VM に戻る所）として扱うところから始める。速さが要れば、後で直接呼ぶ。
* **wasm 版の大きさ**：プラットフォームの機能を足しても、wasm のファイルは大きくしない（ホストの関数は JavaScript の側に置く）。

## 8. 作ったもの

### 8.1. 段階 P1（2026-11-19）

* **`__Feature_available__ name ,(Boolean)`**（コア ConMa のプリミティブ。docs/PRIMITIVES.md の `__JS_value__` の後、docs/dev/ROADMAP.md の Step 49）：拡張 ConMa の機能（`"array"`、`"bitwise32"`、`"boolean"`、`"chan"`、`"control"`）は、どの VM でも使えるので真。`"wasm"` は、WebAssembly が本当に動くとき（wasm 版の VM で、`--trace`、`CONMA_PROFILE`、`CONMA_NO_WASM` のないとき。`__Wasm_call__` と同じ）だけ真。`"js"` は、ホストに JavaScript があるとき（wasm 版の VM）だけ真。ほかの名前は偽。名前が String でなければ、String のプリミティブと同じくエラーで止まる。Python 版（vm/python/vproc_step.py）と Rust 版（vm/src/prim/js.rs。wasm 版と transpiled VM も同じ）。
* **`(Platform "js")`**：`__JS_call__` と `__JS_value__` を、定義のない大域の名前として使うモジュールに、フロントエンド（bin/resolver、include/resolver.se、reference/resolver.se。vm/frontend.module.se を作り直した）が付ける。`(Uses ...)` の後、なければ `(F ...)` の後に置く。この印は、読み込みを止めない。知らせるだけで、JavaScript のない VM でも読み込め、呼んだときに初めて onError に行く（プログラムは `__Feature_available__` で確かめてから使う）。
* **`(Uses ...)` に入れなかった理由**：`(Uses ...)` は「コア ConMa の VM（Python）では、bin/to-core が代わりの定義に置き換える」印である。`"js"` をそこに入れると、Python 版の VM では JavaScript の呼び出しが代わりの定義になり、コア ConMa で作る ConSh の参照版（`__JS_call__` を使う）は、拡張の機能を使わないために代わりの定義を include することになって、wasm 版の VM でも JavaScript を使えなくなる。プラットフォームの機能は、言語の層とは別の軸なので、別の印にし、どの VM もそのプリミティブを持つ（今までどおり、ないプラットフォームでは onError に行く）。bin/to-core は `(Platform ...)` を残し、VM は読み飛ばす。
* **今の `"wasm"`**：WebAssembly も、本来はプラットフォームの機能である。ただ、今は拡張の機能として `(Uses "wasm")` で、Python 版の VM では代わりの定義が onError に行く。P1 では変えず、P2 か P3 で `(Platform ...)` に移すかを決める。
* **テスト**：test/fast13（VM ごとの答え、wasm 版の `CONMA_NO_WASM=1`、String でない名前のエラー、確かめてから `"js"` を使う、フロントエンドの印と bin/to-core が残すこと）。Python 版と Rust 版はエラーのステップの数まで同じログ、wasm 版は `caseN.wasm.log`。

### 8.2. 段階 P2（2026-11-19）

* **`__Host_call__ onError name args ,(result)`**（コア ConMa のプリミティブ。プラットフォームの機能 `"host"`。docs/PRIMITIVES.md、docs/dev/ROADMAP.md の Step 50）：ホストの関数を名前で呼ぶ。渡す値と返る値は、数、文字列、それらの LList で、null は空の LList。Dict と真偽値は、まだ渡さない（M2 のレビュー（docs/dev/REVIEW-ja.md の 2026-11-19）の 3 のとおり。真偽値は、ConMa の真偽値が関数なので、渡し方を決めてから足す）。呼び出しは、すぐに返るものだけで、待つ呼び出し（Stream で結果を返す形）は、要る機能が出てから足す。
* **null と空の LList**：ConMa では null が空の LList なので、ホストには、どちらも `null`（JavaScript の関数には `[]`）として渡り、区別できない。canvas には困らないが、fetch の結果や JSON を渡すようになったら、渡し方を決め直す（M2 の意見の 5）。真偽値は、包むライブラリが 0 と 1 などに直せば、使う側には見えない。
* **どの VM にもあるホストの関数**：機能 `"test"` の `test.add`、`test.echo`、`test.fail`（vm/src/prim/host.rs の `BUILTIN`、vm/python/vproc_step.py の `HOST_FUNCTIONS`）。テストのためのもので、どの VM でも同じに動くので、トレースを比べるテストにも使える（レビューの 2 の「どの VM にも同じホスト」）。`__Feature_available__ "test"` は、どの VM でも真。
* **wasm 版の VM のホスト**：`BUILTIN` にない名前は、ホストの JavaScript の関数（vm/wasm/conma.js の `create` のフック `hostFunctions`。名前から関数へのオブジェクト）を呼ぶ。`hostFeatures`（名前の配列）にある機能は、`__Feature_available__` で真になる。値は JSON で渡す（-0、NaN、無限大は `{"n":"-0"}` などの形）。import は `host_fn_call`（呼ぶ。結果の長さを返す）、`host_fn_result`（結果を写す）、`host_fn_feature`。関数を 1 回だけ呼んでから結果を写すので、大きな結果でも、関数を 2 回呼ばない（今の `host_text` は、バッファが足りないとホストをもう一度呼ぶ）。JavaScript の関数の例外のメッセージは onError に行き、渡せない値（真偽値、オブジェクト）や Promise を返せばエラーになる。Node.js では、vm/wasm/vm-node.js が、環境変数 `CONMA_HOST` のファイル（CommonJS のモジュール）から、この 2 つを読む。
* **ネイティブの VM と Python 版の VM**：`BUILTIN` の関数だけを持つ。ほかの名前は「no host function」で onError に行く。Rust 版の VM に、組み込む人が Rust の関数を足す口は、要る機能が出てから作る。
* **印**：`__Host_call__` を使うモジュールに、フロントエンドが `(Platform "host")` を付ける。どの機能の関数を呼ぶか（`"canvas"` など）は、名前の文字列の中にあるので、フロントエンドには分からない（レビューの 1）。それを包むライブラリが示す形は、P3 で canvas のライブラリと一緒に決める。
* **transpiled VM**：`__Host_call__` は、VM に任せる（レビューの 4）。
* **テスト**：test/fast14（どの VM にもある関数とエラーは、どの VM でも同じログ。wasm 版の VM では、test/fast14/host.js の JavaScript の関数を、-0、NaN、無限大、エスケープのある文字列、例外、返せない値で呼ぶ）。

