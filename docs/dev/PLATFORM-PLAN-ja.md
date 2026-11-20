# いろいろなプラットフォームと、ConMa に変換する言語の計画

ConMa を、いろいろなプラットフォームで、そのプラットフォームに固有の機能（ブラウザの canvas、OS のファイル、モバイルの通知など）も使って動かせるようにする。また、実験的な言語を ConMa に変換すれば、その言語を、ConMa が動くどのプラットフォームでも試せるようにする（2026-11-19）。

**状態**：段階 P1 と P2 を作った（2026-11-19。8 節）。6 節の 1〜3 を決めた（P3 はブラウザの canvas と、待つホストの呼び出しの形から。P5 の見本の言語は代数的エフェクトとハンドラの小さな言語。ConJS を作る。4.1 節）。段階 P3 を作った（8.3 節）。段階 P3b（ConJS）を、4.2 節の設計（PR shima3/conma#3 で M2 がレビューした）で作った（8.4 節）。段階 P4（言語キット）を、4.3 節の設計（PR shima3/conma#5 で M2 がレビューした）で作った（8.5 節）。6 節の 4 と 5 も決めた。M2 のレビュー（気付いたことと、6 節への案）は、[docs/dev/REVIEW-ja.md](REVIEW-ja.md) の 2026-11-19 の節にある（P1 で答えたことは、その「対応」に書いた）。

## 1. 目的

* **A. プラットフォーム**：同じ ConMa のプログラムが、どの VM とプラットフォームでも同じに動く（今のコア ConMa と拡張 ConMa）うえに、プラットフォームに固有の機能も使える。使えない所では、プログラムがそれを知って、別の道を選べる。
* **B. ConMa に変換する言語**：新しい言語を、変換器とランタイムを ConMa で書いて作れば、それだけで ConMa のすべてのプラットフォームで動き、A の固有の機能も使える。SJS（sjs/README-ja.md）がその最初の例で、その作りを、次の言語を作る道具一式（言語キット）にする。

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

sjs/ に、JavaScript の安全なサブセットを ConMa に変換する変換器（sjs/include の `sjs_lex`、`sjs_parse`、`sjs_gen`、`sjs_wasm`）と、ランタイム（`sjs_rt`、`sjs_bits`、`sjs_inspect`）がある。どれも ConMa で書いてあり、ConSh の `(( ))` も使う。

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

### 3.4. 段階 P3 の設計（決めた、2026-11-19。M2 のレビューは PR shima3/conma#1）

**待つホストの呼び出し**（6 節の 3）

* **プリミティブ**：
    * `__Host_start__ onError name args ,(inputStream)`：ホストの関数 name を args で呼び始め、パイプの読む側を返す。関数が終わると、ホストが結果を VM に渡し、VM がパイプの書く側を閉じる。値の種類と名前は `__Host_call__` と同じ。
    * `__Host_result__ onError inputStream ,(result)`：終わった呼び出しの結果。関数が失敗したら、そのメッセージで onError に行く。まだ終わっていなければ onError に行く。結果を取ると、Stream を閉じる。
    * 待つのは、今ある `__OS_wait_readable__ onError inputStream ,()`（パイプが EOF になると読める）。包むライブラリ（下）が、3 つを 1 つの関数にする：`Host_wait onError name args ,(result)`。
    * `__Host_start_tracked__ onError name args Ref ,(inputStream)`：`__Host_start__` と同じで、同じステップの中で、Stream を Ref の LList の前に入れる（`__OS_open_tracked__` と同じ形）。待っている VProc が殺されても、記録した Stream を閉じられる（M2 の PR のコメントの 3）。閉じた Stream の呼び出しが後で終わったら、VM は結果を捨てる。`Host_wait` は、こちらを使う。
    * プラットフォームの印：`__Host_start__`、`__Host_start_tracked__`、`__Host_result__` も、`__Host_call__` と同じく `(Platform "host")` にする（bin/resolver、include/resolver.se、reference/resolver.se）。transpiled VM は、この 3 つを VM に任せる（`__Host_call__` と同じ）。
* **パイプにする理由**：`__JS_call__` と同じ仕組みなので、スケジューラは「外を待っている」と見なし、wasm 版の VM は Idle で返って、ホストの Promise が解決するのを待てる。待つ VProc のほかは動き続ける。Ref の変化を待つ形（`__Ref_wait_change__`）では、すべての VProc が待つとき、スケジューラが行き詰まり（Deadlock）と報告するので、その扱いを足すことになる。
* **wasm 版のホスト**：`hostFunctions` の関数が Promise を返したら（`__Host_start__` のときだけ受け付ける。`__Host_call__` では今までどおりエラー）、解決したときに、VM の新しい export（`conma_host_done(job, 結果の JSON)`）で結果を渡す。VM は、その job のパイプの書く側を閉じる。ホストは、その後 VM をもう一度動かす（`jsDone` と同じ形のフック）。
* **テストの関数**：どの VM にもあるテストの関数（機能 `"test"`）を、`__Host_start__` でも呼べる。`test.later` のほかは、その場で呼んで結果を取っておき、書く側をすぐ閉じる（読めるようになっている）。待つ関数 `test.later`（引数を、そのまま LList で返す。最初の引数は待つ数）を足す。
* **終わる順を決める**（M2 の PR のコメントの 2 と、その次のコメント）：`test.later` の終わる順は、どの VM でも同じにする。ConJS の動かす順のテスト（4.1 節）のログを、VM によらず同じにするため。どの VM も、同じ仮の時計で決める（言葉の上では 1 つの決め方で、ログが同じになることが作りから言える）。
    * VM（wasm 版の VM では conma.js）が時計（初めは 0）を持つ。`test.later` を待つ数 n で呼んだら、締め切りを「今の時計 + n」とし、すぐには終わらせずに持っておく。
    * 動かせる VProc がなくなったら（スケジューラが外を待つ所。wasm 版の VM では Idle で返ったとき）、締め切りのいちばん早いもの（同じなら呼んだ順）を 1 つだけ終わらせ（書く側を閉じ）、時計をその締め切りまで進める。そのあと、また動かせる VProc がなくなるまで動かす。
    * wasm 版の VM でも、本物のマイクロタスクでは待たない（1 回の実行が何個のマイクロタスクにまたがるかは conma.js の作りで変わるため）。ホストの関数が Promise を返す道は、`CONMA_HOST` の関数で確かめる（下のテスト）。
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
* **ランタイム**：値の種類、エラーとその位置、例外の型。
* **動かす道具**：sjs/sjs を一般にしたもの（案：`bin/lang 言語 ファイル`）。変換器のモジュールを作ってキャッシュし、変換して、選んだ VM で動かす。
* **テストとベンチマークの型**：本物の処理系がある言語なら、その出力と比べる（sjs/test の compare.sh の形）。ベンチマークは、bench/ と同じ計算で、ほかの言語と比べる。
* **置き場所**：言語ごとに、sjs/ と同じ形のディレクトリ（`言語/include`、`言語/test`、`言語/bench`、`言語/README-ja.md`）を、リポジトリのルートに置く。言語キットの部品は、標準ライブラリ（include/）か、それとは別のディレクトリに置く（6 節で決める）。
* **ConSh から使う**：SJS の `(( ))` と同じように、ConSh の中から別の言語の式や文を書ける形も、言語ごとに選べる。
* **見本の言語**：キットの部品を確かめるために、小さな言語を 1 つ作る（代数的エフェクトとハンドラの小さな言語。6 節の 2）。キットで作った見本と SJS の両方が動けば、部品が言語によらないことを確かめられる。

### 4.1. ConJS（軽量スレッドを持つ JavaScript の部分集合）

**ConJS** は、JavaScript の部分集合で、協調的に切り替わる軽量スレッド（以下「処理」）と、その処理だけを止めるブロッキング呼び出しを持つ。待つ呼び出し（fetch、タイマー、ファイル、アニメーションの 1 コマ）まで、`await` を書かずに、ふつうの呼び出しとして書ける（`const text = fetch(url).text()`）。待っている間は、その処理だけが止まり、ほかの処理は動く。今は SJS（JavaScript の安全なサブセット）の変換器とランタイムを使って作っているが、SJS とは別の言語で、これから受け付ける JavaScript を広げていく。用語の「直接スタイル」は、ここでだけ「待つ呼び出しまで直接スタイルで書ける」の意味で使う。

* **ConMa で作る甲斐**：JavaScript では、待つ処理が、呼び出す側に `async` と `await` を広げていく（関数の色）。ConMa は CPS で、どの呼び出しの途中でも継続を持つので、待つ処理を「待つ」ことにするだけで、関数に色が付かない（ConSh のパイプと `$( )` が、もうこの形で動いている）。Go のゴルーチンや Java の仮想スレッドの考え方を、JavaScript の構文で、ConMa の動くどのプラットフォームでも試せる。ふつうの JavaScript ではできず、ほかの言語の上に作るのも難しい。
* **協調的に切り替える**（決めた、2026-11-19。M2 の意見の 3）：JavaScript では、2 つの `await` の間のコードは、割り込まれずに最後まで動き（run-to-completion）、プログラムはそれを頼りにオブジェクトを共有する。ConMa の VProc はステップの数で切り替わるので、処理ごとに VProc を使うと、どこでも入り混じる。そこで、ConJS の処理（並べて動かすものも）は、すべて 1 つの VProc の中で動かし、ランタイムが継続で切り替える。切り替わるのは、待つ呼び出しの所だけである。待つこと自体は、待つためだけの小さな VProc に任せ、その VProc が結果をチャネルに入れる。ランタイムは、動かせる処理がなくなったときだけ、そのチャネルを待つ。待つことは並行に進み、ConJS の処理は協調的に切り替わる。VM とスケジューラは変えない。
* **使うもの**：字句解析、構文解析、ランタイム（数、文字列、配列、オブジェクト、例外）の多くは SJS のもの。待つ呼び出しは、P3 で決めるホストの待つ呼び出し（6 節の 3）。
* **動かす順**（M2 の意見（docs/dev/REVIEW-ja.md の 2026-11-19（4））の 1）：動かせる処理の列は FIFO にする。`spawn` した処理は列の最後に入り、呼んだ側が次に待つまで動き続ける（JavaScript のマイクロタスクと同じ）。待つ呼び出しの結果が届いた処理も、列の最後に入る（いくつか届いていれば、チャネルに届いた順）。これで ConJS のプログラムの動きは、ホストが結果を返す順だけで決まる。決定的でないのは、待つための小さな VProc がチャネルに入れる順だけなので、テストではテストのホストの関数（`test.*`）で返す順を決める。
* **Promise**（決めた、2026-11-19。M2 の意見の 2）：`Promise` と `async` は受け付けず、エラーにする。並べて待つには `spawn` と `join` を使う。Promise を受け付けると、`await Promise.all([fetch(a), fetch(b)])` が、エラーにならずに 2 つの fetch を順に待ち、遅くなるだけで気付きにくい。`await` だけは、SJS との行き来のために「その場で待つ」と読んでもよいが、これも P3b で決める。P3b の設計で、`await` も受け付けず、`.then` は除かないことにした（4.2 節）。
* **spawn した処理の例外**（M2 の意見の 3）：`join` で、その例外をもう一度投げる。`join` されないまま例外で終わった処理があれば、プログラムの終わりに報告する（JavaScript の unhandled rejection のように）。P3b のテストに、spawn した処理の例外の場合（join する場合としない場合）を入れる。
* **スケジューラを部品にする**（M2 の意見の 4）：ConJS の切り替え（待つ呼び出しで継続を取り、列に入れ、ほかの処理を動かす）は、「待つ」というエフェクトのハンドラそのものである。P3b では ConJS の中に作るが、継続の列とチャネルを待つことを、ConJS の構文によらない部分として分けておき、P4 で言語キットの部品として取り出す。P5 の見本の言語で、このスケジューラをハンドラとして書けることを確かめる。
* **ConJS を広げる道**（決めた、2026-11-20。M2 のレビュー（docs/dev/REVIEW-ja.md の 2026-11-20）の 2 の a）：足すものが、ふつうの JavaScript の構文と意味（`class`、`new`、`this` など）なら、SJS にも足し、sjs/test で Node.js と出力を比べて確かめる。ConJS は、印で除くもの（`async`、`await`、`Promise`）と、JavaScript と意味が違うもの（`spawn`、`join`、`host` と、待つ呼び出し）のほかは、SJS と同じ言語のまま広がる。こうすれば、2 つの言語の違いは印の所に集まったままになり、4.2 節の「印を見るのは、受け付けない構文の検査と、作ったソースの初めと終わりだけにする」も保てる。SJS に入れたくないものが出てきたら、そのとき、ConJS にだけ足して印を見る所を増やすか、ConJS の変換器をキットの部品で作り直すかを決める。
* **これから決めること**：並べて動かす書き方の細部、JavaScript との関係、置き場所。案は 4.2 節。

### 4.2. 段階 P3b の設計（決めた、2026-11-19。M2 のレビューは PR shima3/conma#3）

4.1 節で決めたこと（1 つの VProc の中で協調的に切り替える、FIFO、Promise を受け付けない、spawn した処理の例外、スケジューラを部品にする）の上に、4.1 節の「これから決めること」と、作り方を決める。

**言語**

* **ファイル**：ConJS のファイルの拡張子は `.con.js` にする（SJS の `.safe.js` と同じ形。エディタと GitHub は JavaScript として扱う）。ファイルは、SJS と同じく文の並びである。
* **SJS から除くもの**（変換するときに `not in ConJS`、終了ステータス 2）：`async`（関数、アロー関数、メソッド、async ジェネレータ）、`await`（`for await` も）、名前 `Promise`。
    * **`then` は除かない**（M2 の PR shima3/conma#3 のレビューの 1）：メンバーの名前 `then` を除くと、`obj.then = 1` や JSON の `"then"` のキーのような、ふつうのデータまで書けなくなる。計算したキーを実行時に確かめると、メンバーを読むよく通る所に検査が入り、SJS のプログラムも遅くなるおそれがある。ConJS には Promise がないので、`then` を持つ値はプログラムが自分で作ったものだけで、持ってきた `x.then(...)` は、実行時に「関数でない」TypeError になって気付ける。
    * **`await` も受け付けない**（4.1 節で「P3b で決める」としたもの）。ConJS では、待つ呼び出しはその場で待つので、`await x` は `x` と同じになり、書く意味がない。受け付けると、JavaScript から持ってきた `await Promise.all([...])` で、`await` は通って `Promise` だけがエラーになり、何が違うのかが分かりにくい。SJS とのソースの行き来に要ることが分かったら、後で足せる（足すのは易しく、除くのは、使っているプログラムがあると難しい）。
    * ジェネレータ（`function*`）は残す。
* **足すもの**（大域の名前）：
    * `spawn(f)`：引数のない関数 f を、新しい処理として列の最後に入れ、処理のオブジェクト（task）を返す。呼んだ側は、次に待つまで動き続ける（4.1 節の動かす順）。f が関数でなければ TypeError。
    * `join(t)`：処理 t が終わるまで待ち、f の戻り値を返す。t が例外で終わっていれば、その例外を投げ直す（位置は、t の中で投げた所のまま）。何度でも、いくつの処理からでも join でき、どれも同じ値か、同じ例外になる。終わっている t の join は、待たずに（切り替わらずに）返る。自分自身を join すると TypeError（必ず行き詰まるため）。
    * `host.call(name, ...args)`：すぐ返るホストの関数（`__Host_call__`）。`host.wait(name, ...args)`：待つホストの関数（`__Host_start_tracked__` で始め、結果が届くまで、この処理だけが待つ）。`host.available(feature)`：`__Feature_available__`。値は、数、文字列、それらの配列（LList にして渡し、返った LList は配列にする）。真偽値とオブジェクトは渡さない（3.2 節。渡すと TypeError）。ホストにその関数がないときと、関数が失敗したときは、ホストのメッセージを message にした `Error` を投げる。
    * 待つ呼び出しは、`host.wait` と `join` の 2 つだけにする。`fetch` や `sleep` のような関数は、ConJS の中で `host.wait` を包んで書ける。ライブラリにするのは、使う所ができてから。
    * 表示：task は `console.log` で `[Task]`、`typeof` は `"object"`。
* **JavaScript との関係**：ConJS の構文は JavaScript の部分集合だが、ConJS のプログラムは JavaScript のプログラムではない（`spawn`、`join`、`host` は JavaScript にない。JavaScript では、待つ呼び出しは Promise を返す）。そのため、sjs/test のように Node.js と出力を比べることはしない。ConJS と SJS の両方が受け付ける、待つ呼び出しを使わないプログラムは、同じ変換器とランタイムで動くので、どちらでも同じ結果になる（初めは「待つ呼び出しを使わない ConJS のプログラムは SJS のプログラムでもある」と書いていたが、ConJS を広げると成り立たないので書き直した。M2 のレビュー、docs/dev/REVIEW-ja.md の 2026-11-20 の 1）。
* **関数の色がない**：どの関数からでも待つ呼び出しを呼べる。JavaScript の async 関数のように、呼ぶ側を書き換えて回る必要がない。配列の `map` に渡した関数や、ジェネレータの本体の中でも待てる（継続で切り替えるので、どこで待ってもよい）。
* **割り込まれないこと**：2 つの待つ呼び出しの間のコードは、割り込まれずに動く（4.1 節）。待たない長いループは、その間ほかの処理を止める（JavaScript と同じ）。

**終わりと例外**

* main（ファイルの文の並び）も 1 つの処理である。main が受け手のない例外で終われば、SJS と同じく、すぐにメッセージを出し、終了ステータス 2 で終える。ほかの処理は待たない（Node.js の受け手のない例外と同じ）。
* main が終わっても、動かせる処理か、待っている呼び出しが残っていれば、動かし続ける（Node.js が、イベントループが空になるまで終わらないのと同じ）。
* すべての処理が終わったら、join されないまま例外で終わった処理を、終わった順に `ファイル:行:桁: Uncaught (in task) TypeError: ...` の形で標準エラーに出す。1 つでもあれば、終了ステータス 2。
* **行き詰まり**：動かせる処理も待っている呼び出しもなく、join で待っている処理だけが残ったら（2 つの処理が互いを join するなど）、`Deadlock: N tasks are waiting in join` を出し、終了ステータス 2 で終える。

**スケジューラ**（conjs/include/conjs_sched.se。ConJS の構文にも、SJS の名前にもよらない部分。4.1 節の「部品にする」）

* **処理の記録**：Dict。状態（動ける、待っている、終わった、例外で終わった）、結果、join で待っている処理の LList、その処理だけの状態（下）を持つ。
* **列**：動かせる処理（継続と、それに渡す値）の FIFO。SJS のマイクロタスクの列（`Sjs_jobs`）と同じく、2 つの LList で作る。
* **待つ**：`Sched_wait onError name args ,(result)` は、`__Cont_get__` で今の処理の継続を取り、その処理だけの状態を保存し、待つための小さな VProc を `__VProc_start__` で作ってから、列の次の処理を動かす。小さな VProc は `Host_wait_tracked`（include/platform/host.se）で待ち、（処理、成否、値）を 1 つのチャネルに入れる。
* **動かせる処理がなくなったら**：待っている呼び出しがあれば、チャネルから 1 つ取り（`__Chan_take__`。ここで、この VProc が待つ）、その処理を列の最後に入れる。待っている呼び出しも join で待つ処理もなければ終わり、join で待つ処理だけなら行き詰まり。
    * `test.later` の仮の時計は、動かせる VProc がなくなるまで呼び出しを終わらせないので、ConJS の VProc が動いている間は、結果が届かない。結果は、ConJS の処理がすべて待ったときに、締め切りの順に 1 つずつ届く。テストのログは、どの VM でも同じになる。
    * そのため、`spawn` し続けて、動かせる処理がなくならないプログラムには、ホストの結果がずっと届かない（M2 のレビューの 4）。Node.js でも、マイクロタスクを積み続けると同じことになるので、そのままにする。
* **処理だけの状態**：スケジューラは、切り替えるときに呼ぶ 2 つの関数（`save ,(state)` と `restore state ,()`）を受け取る。ConJS は、PDict の `"sjs_handlers"`（try の受け手のスタック）、`"sjs_gen"`（動いているジェネレータ）、`"sjs_pos"`（投げた所の位置）を保存して、戻す。これで、try の受け手が処理ごとに分かれる（ある処理が try の中で待っている間に、別の処理が投げた例外は、その try に捕まらない）。
    * `"sjs_pos"`（M2 のレビューの 2）：sjs_rt は、投げた所の位置を `"sjs_pos"` に置き、catch で消す。処理 A が投げた例外が通る finally の中で A が待ち、その間に処理 B が throw と catch をすると、`"sjs_pos"` が書き換わり（または消え）、A の受け手のない例外のメッセージや、join で投げ直したときの位置が B のもの（またはなし）になる。そのため、これも処理ごとに保存する。処理が例外で終わったときは、その位置を例外と一緒に記録し、join は、その位置を戻してから投げ直す。
    * 保存する PDict の名前は、ConJS のつなぎ（conjs_rt.se）の 1 か所にまとめ、sjs_rt に名前を足したときに忘れないようにする。
* **プログラムが途中で終わるとき**（main の受け手のない例外）：待つための VProc の Stream を閉じ（track の Ref）、その VProc を殺してから終える。
* P5 の見本の言語で、このスケジューラを「待つ」エフェクトのハンドラとして書けることを確かめる（4.1 節）。P4 では、このファイルを言語キットに移す。

**変換器と道具**

* **変換器は SJS のものを共有する**：sjs_lex、sjs_parse、sjs_gen に、言語の印（`"sjs"` か `"conjs"`）を足して使い、写しは作らない。印を見るのは、受け付けない構文の検査と、作ったソースの初めと終わり（main を処理として動かし、スケジューラを最後まで回す）だけにする。ほかは同じコードを作るので、SJS のプログラムは遅くならない。P4 で部品を取り出すとき、2 つの言語の違いが印の所に集まっていると分けやすい。
* **ランタイム**：conjs/include/conjs_rt.se に、`spawn`、`join`、`host` と、SJS のランタイムとのつなぎを置く。`Promise` は名前なので、変換するときだけ確かめ、sjs_rt に実行時の検査は足さない。
* **動かす道具**：conjs/conjs（sjs/sjs と同じ形。`--emit`、`CONMA_VM=wasm`、`CONMA_VM=python`）。変換器のモジュールは、bin/cache-dir の場所に conjsc.module.se として作っておく。
* **置き場所**：conjs/（conjs/include、conjs/test、conjs/README-ja.md）をリポジトリのルートに置く（sjs/ と同じ。6 節の 5 で `langs/` にまとめると決めたら、sjs/ と一緒に移す）。
* **ConSh**：ConSh の `(( ))` から ConJS を使うことは、P3b ではしない。

**テスト**（conjs/test/stepN。`.con.js` と、期待するログ。変換は Rust 版の VM で行い、変換したプログラムを Rust 版、wasm 版（Node.js）、Python 版の VM で動かして、同じログになることを確かめる）

1. 動かす順：`spawn` の FIFO。`test.later` で返す順を決めた待つ呼び出し（締め切りが違うもの、同じもの）。待っている間に、ほかの処理が動くこと。
2. 協調的に切り替わること：2 つの処理が、共有のオブジェクトを待つ呼び出しの間で読み書きしても入り混じらないこと。VProc が切り替わるより長いループを待たずに回しても、ほかの処理が割り込まないこと。
3. 例外：`join` が投げ直す（位置も）。join されない例外が終わりに報告される。main の例外は、ほかの処理を待たずに終える。try の受け手が処理ごとに分かれる（上の例）。投げた例外が通る finally の中で待ち、その間にほかの処理が throw と catch をしても、受け手のない例外のメッセージと join で投げ直した位置が、最初に投げた所のままであること。
4. 待つ所：ジェネレータの本体、配列の `map` に渡した関数、try と finally の中。ある処理がジェネレータの本体の中で待っている間に、ほかの処理がそのジェネレータの `next()` を呼ぶと、TypeError（`Generator is already running`）になること。
5. エラー：行き詰まり、自分自身の join、関数でないものの spawn、`host.call` と `host.wait` のホストのエラー（ない関数、失敗）、渡せない値。wasm 版の VM では、`CONMA_HOST` の関数が Promise を返す場合も。
6. 受け付けない構文：`async`、`await`、`for await`、`Promise`。`then` はふつうの名前として使えること。
* 変換器を共有するので、sjs/test がすべて通ること、sjs/bench が遅くならないことも確かめる。

**見本**：vm/wasm/web の canvas の見本（跳ねるボール）を ConJS で書いたもの（canvas_demo.con.js）。`host.wait("canvas.frame")` を、`await` のない while ループで呼ぶ。canvas.html で、ConMa 版と選べるようにする。

**P3b でしないこと**：ConSh の `(( ))` から使うこと。チャネル（`spawn` と `join` で書きにくい例が出てから決める）。`fetch` などのライブラリ。Worker。

**M2 のレビュー**（PR shima3/conma#3）：聞いた 4 つ（`.con.js` の拡張子、`await` を受け付けないこと、main の後も処理が残っていれば動かし続けること、変換器を写さず印で共有すること）は、どれも賛成だった。指摘の 1〜4（`then` を除かない、`"sjs_pos"` も保存する、待っている途中のジェネレータのテスト、結果が届かなくなる場合）を、上に入れた。

### 4.3. 段階 P4 の設計（決めた、2026-11-20。M2 のレビューは PR shima3/conma#5）

4 節の言語キットを、SJS と ConJS から取り出す。6 節の 4 と 5 も、ここで案を出す。

**終わりとすること**（5 節の表に足す）：キットの部品で SJS と ConJS を書き直しても、SJS と ConJS が作る ConMa のコードが 1 文字も変わらないこと、実行時の部品を移しても、変換したプログラムのステップ数が変わらないこと（下の「確かめ方」）。sjs/test、conjs/test（3 つの VM）、consh/test、consh/js/test が通ること。変換の時間が遅くならないこと。キットだけで作った小さな言語のテストが、すべての VM で通ること。

**置き場所**（6 節の 4 と 5）

* **キットは include/lang/ に置く**（6 節の 4 の案）：`(include "lang/lex.se")` のように読む（include/platform/ と同じ形）。標準ライブラリの探し場所に入っているので、ConSh（consh/bin/consh、consh/js/build.sh）、wasm 版のページ（vm/wasm/web/build.sh）、sjs/sjs、sjs/bench/run.sh、conjs/conjs の include の並びを変えずに済む。別のディレクトリにすると、そのすべてに足すことになり、足し忘れると、その道だけで「include が見つからない」になる。キットの部品は、ふつうのプログラムからも使える（文字列を字句にする、構文木の節を作る）ので、標準ライブラリに置いてもおかしくない。
* **言語のディレクトリは、ルートに並べたままにする**（6 節の 5 の案）：sjs/ を `langs/` の下に移すと、ConSh、bench、文書の多くのパスと、「移動」の記録（docs/dev/CONTRIBUTING.md の Files that moved）が要る。言語が 3〜4 個のうちは、ルートに並べても見通しは悪くならない。P5 の見本の言語も、ルートに置く。言語が増えて見通しが悪くなったら、そのときにまとめて移す。
* **名前**：キットの関数は `Kit_` と部品の名前で始める（`Kit_lex_`、`Kit_parse_`、`Kit_code_`、`Kit_err_`、`Kit_sched_`）。`Parse_`（include/parser.se）と `Sched_`（conjs）は使われているため。キットが新しく使う PDict の名前は `"kit_"` で始めるが、**今ある名前（`"sjs_pos"`、`"sjs_handlers"`、`"sjs_gen"` など）は変えない**（M2 の PR shima3/conma#5 のレビューの 2）。consh/include/consh_jsstmt.se が `"sjs_pos"` と `"sjs_handlers"` を直接読み書きし、conjs_rt.se の `Conjs_local_keys` も `"sjs_pos"` を使うため。キットの部品で、そうした名前を使うものは、名前を引数で受け取る（エラーの部品は、位置を置く名前を受け取り、SJS は `"sjs_pos"` を渡す）。

**部品**（左が取り出すもの、右が今あるところ）

* **字句**（include/lang/lex.se）：行と桁を持つ字句（今の `Sjs_tok`、`Tok_kind`、`Tok_text`、`Tok_line`、`Tok_col`、`Tok_nl`）、行の始まりの表と桁（`Sjs_lex_starts`、`Sjs_lex_col`）、文字を見る関数（`Sjs_peek_char`、`Sjs_is_digit`、`Sjs_is_ident_start`、`Sjs_is_ident_char`、`Sjs_code`）、空白と `//`、`/* */` のコメント、10 進と 16 進の数、`"` と `'` の文字列と逃がし（`Sjs_lex_number`、`Sjs_lex_string`、`Sjs_escape`）、句読点のいちばん長い一致（`Sjs_lex_punct`。句読点の一覧を引数で受け取る）、語の一覧（`Sjs_words`、`Sjs_in_words`、`Sjs_starts`）。SJS に残すもの：テンプレート文字列、正規表現の検査、キーワードの一覧、字句の段階の範囲の検査（`Sjs_check_tokens`）。
* **構文**（include/lang/parse.se）：字句の列を読む状態と、`P_peek`、`P_next`、`P_eat`、`P_expect`、`P_fail`（「expected X」のエラーと、その字句の位置）、節（`Sjs_node`、`Sjs_get`、`P_at` の行と桁）、演算子の優先順位で 2 項演算を読む部品（今の `P_binary` と `Sjs_prec`。優先順位の表と、被演算子を読む関数を引数で受け取る）。SJS に残すもの：文、式のほかの部分、パターン、関数。
* **コード**（include/lang/code.se）：ConMa のソースを作る部品。文字列のリテラル（`Gen_quote`）、つなぐ（`Cat3` など）、一時変数（`Gen_temp`）、名前の有効範囲（`Gen_lookup`、`Gen_declare`。名前から ConMa の変数名への Dict の LList）、SInfo の印（`Gen_sinfo_fill`、`Gen_replace_marks`。言語の名前を引数で受け取る。SJS は今と同じ `"sjs"`）。SJS に残すもの：文と式の変換、数の関数の wasm 版。
* **エラー**（include/lang/error.se）：投げた所の位置（今の `Sjs_raise_pos`。CChain の CFrame から、SInfo の最後が言語の名前のいちばん上のもの）と、受け手のない例外のメッセージ（`ファイル:行:桁: メッセージ`、位置がなければ `言語: メッセージ`、終了ステータス 2）。例外の受け手のスタック（`Sjs_try`、`Sjs_raise`）と値の種類は、SJS に残す。P5 の見本の言語は、例外をハンドラで作るので、要るものが違うかもしれない。P5 で要ると分かったものを、そのときキットに移す。
* **スケジューラ**（include/lang/sched.se）：conjs/include/conjs_sched.se を移し、`Sched_` を `Kit_sched_` にする。ConJS はそれを include する。
* **動かす道具**（bin/lang）：sjs/sjs と conjs/conjs の共通の部分。`bin/lang [--emit] 言語のディレクトリ ファイル`。変換器のモジュールをキャッシュし（bin/cache-dir）、変換して、`CONMA_VM` の VM で動かす。言語のディレクトリの `lang.conf`（`translator=sjsc.se`、`include=sjs/include` のような `名前=値` の行。シェルとして読まず、bin/lang が行ごとに読む）に、変換器と include の並びを書く。sjs/sjs と conjs/conjs は、今の使い方のまま、bin/lang を呼ぶだけにする（文書とテストがこの名前を使っているため）。
* **テストの型**（bin/lang-test）：conjs/test/run_case.sh を一般にしたもの。ステップのディレクトリの `caseN.拡張子` を bin/lang で動かし、出力（標準エラーは後ろに続ける）と終了ステータスを `caseN.log`（VM ごとの `caseN.$CONMA_VM.log`）と比べる。ConJS は、これを使う。SJS は、Node.js と比べる今の形（sjs/test の test.sh と compare.sh）のままにする。

**確かめ方**

* **作るコードが変わらないこと**（変換器の部品：字句、構文、コード）：部品を 1 つ移すごとに、sjs/test、sjs/bench、sjs/bench/consh（`(( ))` の中）、conjs/test、vm/wasm/web の見本のすべてのソースを `--emit` で変換し、移す前と 1 文字も違わないことを確かめる（その道具を P4 の初めに作る。`bin/lang-emit-check 前のコミット`。前のコミットのワークツリーで変換したものと比べる）。作るコードが同じで、実行時の部品も変えていなければ、変換したプログラムの速さと動きは変わらないので、sjs/bench の実行の時間は測り直さない。
* **ステップ数が変わらないこと**（実行時の部品：エラー、スケジューラ。M2 のレビューの 1）：これらは変換したプログラムが include する部品なので、移しても `--emit` は変わらない。移す前と後で、sjs/bench（nbody、fib、loop の小さい大きさ）と conjs/test のすべての場合のステップ数を、Rust 版の VM で `CONMA_PROFILE=file` と vm/tools/vm_profile.py で数え、同じであることを確かめる（ステップ数は機械によらず、毎回同じ）。ステップ数が変わったら、どこで変わったかを調べ、増えたなら直す。関数の名前が変わるだけなら、ステップ数は変わらない。
* **キットだけの言語**：test/fast17 に、キットの部品だけで作った小さな電卓の言語（数、`+ - * /`、括弧、`let`、`print`、エラーの位置）の変換器と、そのプログラムを置き、Python 版、Rust 版、wasm 版の VM で同じログになることを確かめる。キットの部品が SJS の名前を使っていないことの確かめにもなる（SJS のファイルを include しないで作れる）。
* **変換の速さ**：キットの部品が引数で一覧や表を受け取るようにすると、変換が遅くなるおそれがある。変換のステップ数を、移す前と後で比べる（M2 のレビューの 3。nbody の変換は約 0.15 秒で、その 5% は時間のばらつきに埋もれるため）。入力は、sjs/test、sjs/bench、conjs/test のすべてのソースを変換器に順に渡すもの。ステップ数が 5% より増えたら、そこを直してから進める。時間は、最後に、sjs/bench/consh の nbody.sh（時間の大半が ConSh の `(( ))` の変換）で、移す前と後を交互に測る（m1/bench-<7 桁>。docs/dev/CONTRIBUTING.md の Benchmarks）。
* consh/include は変えない（ConSh は、sjs/include を include するだけで、PDict の名前も変えない）が、ConSh のモジュールは変わるので、consh/test と consh/js/test を流し、transpiled VM は作り直して流してもらう。

**P4 で決めないこと、しないこと**

* **構文木から直接モジュールを作ること**（4 節）：P4 では、変換の時間の内訳（字句、構文、コード、`conma build`）を測るだけにする。作り変えるかは、その数を見て、後の段階で決める。
* ConSh から別の言語を使う形（`(( ))` のような）、値の種類と例外の受け手のキット、`langs/` への移動。

**M2 のレビュー**（PR shima3/conma#5）：聞いた 4 つ（キットを include/lang/ に置くこと、言語のディレクトリをルートに並べたままにすること、移すものと残すものの分け方、作るコードが 1 文字も変わらないことを終わりの条件にすること）は、どれも賛成だった。指摘の 1〜3（実行時の部品はステップ数で確かめる、今ある PDict の名前を変えない、変換の速さはステップ数で比べる）を、上に入れた。

## 5. 段階

| 段階 | すること | 終わりとすること |
|---|---|---|
| P0 | この計画書。6 節を決める | 決めたことを、この文書に書く |
| P1（作った、2026-11-19。8 節） | プラットフォームの機能の層：`(Platform "機能")` の付け方、`__Feature_available__`。`__JS_call__` と `__JS_value__` を機能 `"js"` にする | test/ と consh/test がすべての VM で通る。機能の有無で振る舞いを変えるテスト |
| P2（作った、2026-11-19。8.2 節） | `__Host_call__`：wasm 版（JavaScript）、Rust 版と transpiled VM（Rust の表）、Python 版（Python の表）。テストのためのホストの関数 | 同じテストのトレースが Python 版と Rust 版で同じ。wasm 版の Node.js で同じ出力 |
| P3（作った、2026-11-19。8.3 節） | 待つホストの呼び出しの形を決めて作る（6 節の 3）。最初のプラットフォームの機能（ブラウザの canvas。描く見本のページ） | 見本が動く。1 回の呼び出しの時間を、今の `js` コマンドと比べて測る。足した機能の名前を docs/PRIMITIVES.md の `__Feature_available__` の一覧に書き、テストで、見本が使う名前が一覧にあることを確かめる（知らない名前は黙って偽になり、書き間違いと区別できないため） |
| P3b（作った、2026-11-19。8.4 節） | ConJS（4.1 節）：SJS の部品で作る。P3 の待つ呼び出しを使う | ConJS のテストが、すべての VM で通る。協調的に切り替わること（待つ呼び出しの所だけで切り替わる）を確かめるテスト |
| P4（作った、2026-11-20。8.5 節） | 言語キット：SJS から部品を取り出し、SJS と ConJS をキットの上に書き直す | sjs/test、consh/test、sjs/bench（遅くならないこと） |
| P5 | 見本の言語（代数的エフェクトとハンドラの小さな言語。6 節の 2）を、キットで作る | その言語のテストが、すべての VM で通る。P3 の機能を使う見本 |
| P6 | プラットフォームを増やす（6 節の順に） | そのプラットフォームでテストが通る |

## 6. 決めること

1. **プラットフォームの順**（決めた、2026-11-19）：P3 は、ブラウザの canvas から始める。手間が小さく、目に見えて確かめやすい。P6 の順は、P3 の後で決める。
2. **見本の言語**（決めた、2026-11-19）：P5 の見本の言語は、**代数的エフェクトとハンドラの小さな言語**（Koka や Eff のような）にする。ハンドラは、捕まえた継続を値として扱うだけ（`__Cont_get__`）で、ConMa で作りやすい。構文を自分で決められるので、SJS と違う構文で、キットが言語によらないこと（4 節）も確かめられる。
    * M1 は、初め「待つ処理を直接スタイルで書ける、JavaScript の部分集合」を見本の言語に勧めた（aae726a）。M2 の意見（docs/dev/REVIEW-ja.md の 2026-11-19（3））を受けて、名前を **ConJS** にし（「直接スタイル」は、ふつうの JavaScript もそうなので区別できない。「await なしの JS」は非同期ができないように聞こえる。ConMa、ConSh と並ぶ名前にした）、見本の言語ではなく、**SJS の次の段**として作る（4.1 節、段階 P3b）。JavaScript の構文なので、キットが構文によらないことの確かめにならないため。
    * 比べた候補：Lua のサブセット（M2 の初めの案。キットの確かめにはなるが、C の処理系がどこでも動くので、ConMa に変換する得は小さい）、小さな Lisp（S 式で、キットの字句解析と構文解析をほとんど確かめない）、Forth のようなスタックの言語、アクターやチャネルの並行言語、論理型言語の小さなもの。
3. **ホストを呼ぶ口の形**：すぐ返る呼び出しは P2 で作った（8.2 節。Dict はまだ渡さない）。**待つ呼び出しの形は、P3 の初めに決める**（決めた、2026-11-19）。canvas のアニメーション（`requestAnimationFrame` を待つ）と ConJS（4.1 節）の両方に要るので、見本の言語より先に決める（M2 の意見の 4）。
4. **言語キットの置き場所**：標準ライブラリ（include/）に入れるか、別のディレクトリにするか。決めた（2026-11-20）：include/lang/（4.3 節）。
5. **言語のディレクトリの置き場所**：sjs/ と同じくルートに並べるか、`langs/` のような 1 つのディレクトリの下にまとめるか（まとめるなら sjs/ も移す）。決めた（2026-11-20）：ルートに並べたまま（4.3 節）。

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

### 8.3. 段階 P3（2026-11-19）

3.4 節の設計（PR shima3/conma#1 で M2 がレビューした）で作った。作るときに設計と変えたところは、下の「設計と変えたところ」に書く。

* **待つホストの呼び出し**（コア ConMa のプリミティブ。プラットフォームの機能 `"host"`。docs/PRIMITIVES.md、docs/dev/ROADMAP.md の Step 51）：`__Host_start__ onError name args ,(inputStream)`、`__Host_start_tracked__ onError name args Ref ,(inputStream)`、`__Host_result__ onError inputStream ,(result)`。結果は、パイプを通る結果の JSON（`{"r":値}` か `{"e":"メッセージ"}`）で、どの VM でも同じ形。`__Host_result__` は、パイプが終わりでなければ「the call has not finished」で onError に行く。
* **test.later と仮の時計**：Rust 版の VM の `Later`（vm/src/prim/host.rs）を、スケジューラ（vm/src/sched.rs）が、動かせる VProc がないときに 1 つ終わらせる。wasm 版の VM も同じスケジューラなので、conma.js には何も要らない。Python 版の VM は、そのスケジューラの `finish_later`。呼び出しを終わらせた後は、OS を待たずに確かめる（終わらせた呼び出しの読み手が、もういないことがあるため。test/fast15 の case3 で見つかった）。
* **wasm 版の VM のホスト**：`BUILTIN` にない名前は、import の `host_fn_has` で確かめ、`host_fn_start` で呼ぶ。conma.js は、関数を後のタスクで呼び（`conma_run` の中で VM に入り直さないため）、値か Promise の結果を、パイプに `conma_write` で書いて `conma_close` で閉じ、`jsDone` で VM をもう一度動かす。その間は `jsPending` に数える。
* **ネイティブの VM と Python 版の VM**：`BUILTIN` の関数は、呼んだステップで結果をパイプに書いて閉じる。OS のパイプに読み手なしで書けるのは限りがあるので、結果が 8192 バイトを超えたら、それをエラーの結果にする（`PIPE_MAX`、`HOST_PIPE_MAX`）。テストの関数の結果は小さい。
* **canvas**：vm/wasm/host/canvas.js（3.4 節の関数。2D の context と、コマを呼ぶ関数を、オプションで受け取れる）、include/platform/host.se（`Host_wait`、`Host_wait_tracked`）、include/platform/canvas.se（`Canvas_available` と `Canvas_fill_rect` など）。
* **見本のページ**：vm/wasm/web/canvas.html と canvas_demo.se（跳ねる玉。1 コマを 1 回の `Canvas_draw` で描き、`Canvas_frame` で待つ）。vm/wasm/web/build.sh が VM とモジュールを置き（git には入れない）、`.claude/launch.json` の `conma-canvas`（または `python3 -m http.server -d vm/wasm/web`）で見る。アプリの中のブラウザで、玉が動き、コンソールにエラーがないことを確かめた。ページの中で VM を動かす（Worker と OffscreenCanvas は、まだ）。
* **測ったこと**（vm/wasm/web/measure.sh。wasm 版の VM、Node.js、何もしない canvas。引数を作るだけの時間を引いた、線 1 本あたり。5 万本、3 回の中央値）：`__Host_call__` で 1 本ずつ 2.3 µs、`canvas.draw` で 1000 本ずつまとめて 0.7 µs、今の `js` コマンドと同じ `__JS_call__` で 35 µs。`__JS_call__` は、20 万本では 118 µs になり、呼ぶほど 1 回が遅くなる（ほかの 2 つは本数に比例する）。原因は、まだ調べていない。
* **テスト**：test/fast15（3.4 節の待つ呼び出し：結果、エラー、終わる前の `__Host_result__`、`__Host_start_tracked__`、`test.later` の終わる順、殺した VProc の Stream、wasm 版の VM で Promise を返す関数。Python 版、Rust 版、wasm 版の VM で同じログ）、test/fast16（canvas のない VM での canvas のライブラリ、wasm 版の VM で命令を記録する canvas のまね、ライブラリが使う `canvas.` の名前が docs/PRIMITIVES.md にあること）。
* **設計と変えたところ**：
    * wasm 版の VM に、新しい export の `conma_host_done` は作らなかった。結果は、今ある `conma_write` と `conma_close` でパイプに書く（`__JS_call__` と同じ）。
    * 仮の時計は、conma.js ではなく、Rust のスケジューラに置いた（wasm 版の VM も同じスケジューラなので、決め方を 1 か所に書ける）。
    * `Host_wait` は `__Host_start__` を使う。`__Host_start_tracked__` の Ref は、それを閉じる側が持つものなので、Ref を受け取る `Host_wait_tracked` を別に置いた（`__OS_open_tracked__` を使う ConSh と同じ形）。

### 8.4. 段階 P3b（2026-11-19）

4.2 節の設計（PR shima3/conma#3 で M2 がレビューした）で作った。使い方と言語の説明は conjs/README-ja.md。

* **conjs/**：conjs/conjs（変換して動かす道具。`--emit`、`CONMA_VM` に wasm と python）、conjs/conjsc.se（変換器の入口）、conjs/include/conjs_sched.se（スケジューラ）、conjs/include/conjs_rt.se（SJS のランタイムとのつなぎ、`spawn`、`join`、`host.call`、`host.wait`、`host.available`）、conjs/test（6 つのステップ、24 の場合）、conjs/README-ja.md。
* **SJS の変換器とランタイムに足したもの**：sjs_parse.se の名前の検査（印が ConJS なら `async`、`await`、`Promise` を除き、「not in SJS」を「not in ConJS」と書く）、sjs_gen.se の `Gen_conjs_program`（文の並びを `Conjs_run` の中に置く）、sjs_rt.se の `Sjs_is_conjs`（受け手のない例外の前置きを `conjs:` にする）と、値の種類 `"task"`（`typeof`、`String`、`JSON.stringify`、`in`、エラーの説明）、sjs_inspect.se の `[Task]`。どれも、SJS のプログラムが通る所では、今までと同じコードを作り、同じ道を通る（`"task"` の検査は、ほかの種類の検査の後に置いた）。
* **見本**：vm/wasm/web/canvas_demo.con.js（canvas_demo.se と同じ跳ねる玉。もう 1 つの処理が、30 コマごとに見出しを変える）。vm/wasm/web/build.sh が変換してモジュールにし（canvas_demo_conjs.module.se。git には入れない）、`canvas.html?conjs` で見る。アプリの中のブラウザで、毎秒約 60 コマで動き、見出しが変わり、コンソールにエラーがないことを確かめた。モジュールは SJS のランタイムを含むので、約 8 MB ある（ConMa 版は約 0.2 MB）。
* **テスト**：conjs/test の 24 の場合が、Rust 版、wasm 版（Node.js）、Python 版の VM で同じログになる（wasm 版の VM だけにある Promise を返すホストの関数の場合は、case4.wasm.log）。
* **設計と変えたところ**：
    * チャネルは、読み手の VProc（ConJS の VProc）が、書かない書き手を 1 つ閉じずに持つ。チャネルの書き手を足せるのは、書き手を持つ VProc か、読み始める前に作った VProc だけなので（docs/PRIMITIVES.md の Channels）、こうして、読み始めた後でも、待つための VProc ごとに書き手を作れるようにした。チャネルが終わる（すべての書き手が閉じる）こともない。
    * 待つための VProc は `Host_wait_tracked` を使わず、ConJS の VProc が `__Host_start__` で呼び始めてから、その Stream を待つ VProc に渡す。呼び始めるときのエラー（ない関数など）が、待たずにその場で投げられ、`test.later` の締め切りも、呼んだ処理の順で決まる。プログラムが途中で終わるのは `__OS_exit__` のときだけで、そのとき待つための VProc も一緒に終わるので、Stream を記録して閉じることはしなかった。
    * 4.2 節は「除くもの」を名前としか書いていなかったが、オブジェクトのキーとメンバーの名前には、3 つの名前を使えるようにした（M2 の PR shima3/conma#4 のレビューの 1。`then` を除かないのと同じく、JSON のようなデータを書けるように）。字句の検査で、`{` か `,` の後で `:` の前にある名前を、キーと見る。
    * 行き詰まりのメッセージは `conjs: Deadlock: tasks waiting in join: N` にした（4.2 節の案は `Deadlock: N tasks are waiting in join`。数で始めず、1 つのときも文が崩れないように）。

### 8.5. 段階 P4（2026-11-20）

4.3 節の設計（PR shima3/conma#5 で M2 がレビューした）で作った。部品を 1 つ移すごとに、bin/lang-emit-check（作るコード）と bin/lang-step-check（実行時のステップ数）で確かめ、1 つずつコミットした。

* **include/lang/**（言語キット）：
    * lex.se：字句と、その手伝いの関数（`Kit_lex_tok` と `Kit_lex_tok_kind` などの読み出し、`Kit_lex_line_starts` と `Kit_lex_col`、`Kit_lex_peek` などの文字の関数、コメント、`Kit_lex_number`、`Kit_lex_string` と `Kit_lex_escape`、`Kit_lex_ident`、`Kit_lex_punct`、`Kit_lex_words`、`Kit_lex_in_words`）。ほかの言語のために `Kit_lex_start`、`Kit_lex_chars`。エラーは `Kit_lex_error`（PDict の `"kit_error"` の関数があればそれを呼び、なければメッセージを出して終了ステータス 2）。
    * parse.se：読む状態（`Kit_parse_new`、`Kit_parse_peek`、`Kit_parse_next`、`Kit_parse_eat`、`Kit_parse_expect`、`Kit_parse_fail`）、節（`Kit_parse_node`、`Kit_parse_get`、`Kit_parse_at`）、優先順位で 2 項演算を読む `Kit_parse_binary`（被演算子を読む関数と、優先順位の関数を受け取る）。
    * code.se：ConMa のソースを作る部品（`Kit_code_quote`、`Kit_code_cat3`〜`5`、`Kit_code_temp`、`Kit_code_lookup`、SInfo の印の `Kit_code_sinfo_fill` と `Kit_code_replace_marks`）。SInfo の言語の名前は、状態 g の `"lang"`（`Kit_code_state`、`Kit_code_set_lang`）。
    * error.se：投げた所の位置 `Kit_err_pos`（SInfo の印を引数で受け取る。SJS は `":sjs"`）と、受け手のない例外の `Kit_err_report`。
    * sched.se：conjs/include/conjs_sched.se を移し、`Sched_` を `Kit_sched_` にした。
* **道具**：bin/lang（言語のディレクトリの lang.conf を読んで、変換して動かす。sjs/sjs と conjs/conjs は、これを呼ぶだけにした。sjs/sjs も `CONMA_VM=python` で動く）、bin/lang-test（conjs/test/run_case.sh を一般にしたもの。ConJS が使う）、bin/lang-emit-check、bin/lang-step-check。
* **test/fast17**：キットの部品だけで作った電卓の言語（calc.se と calc_rt.se。SJS のファイルを include しない）。Python 版、Rust 版、wasm 版の VM で同じログ。
* **確かめたこと**：alpha（68795c9）と比べて、sjs/test、sjs/bench、conjs/test、見本の 97 のソースで、作るコードが 1 文字も違わない。変換のステップ数は 76,336,089 から 76,373,138（+0.05%。SInfo ごとに、状態から言語の名前の部分を引く 1 ステップ）。sjs/test（wasm 版だけの step9 と step10 を除く）、conjs/test、sjs/bench の小さい大きさの 83 のプログラムで、出力は同じ。ステップ数は、sjs/bench と、例外を投げないプログラムで同じ。例外を投げる 40 のプログラム（sjs/test の 32、conjs/test の 8）は 1〜11 多い（合計 +136。投げるたびに印 `":sjs"` を `Kit_err_pos` に渡す 1 ステップと、受け手のない例外の報告 `Kit_err_report` を呼ぶ 1 ステップ）。4.3 節は「ステップ数が同じであること」としていたが、例外の道だけの、この増え方は受け入れた（M2 の PR shima3/conma#6 のレビューの 1）。そのため、alpha（68795c9）と比べた bin/lang-step-check は、終了ステータス 1 で終わる。この後に実行時の部品を変えるときは、この段階を入れた後のコミットと比べる（そうすれば、違いがなければ 0 で終わる）。
* **変換の時間の内訳**（4.3 節の「決めないこと」。Rust 版の VM、Apple M1。nbody の大きさ 2000）：変換が約 0.15 秒（ステップの 29% が字句、17% が構文、48% がコードと数の関数の wasm 版）。変換したソースを `conma build` でモジュールにするのに約 1.65 秒かかり、そのうち約 1.42 秒は、空のプログラムでもかかる（ランタイム sjs_rt.se などを、毎回フロントエンドに通している）。構文木から直接モジュールを作っても、減らせるのは変換したコードの分（約 0.23 秒）だけである。先に作っておいたランタイムのモジュールを使い回す方が、ずっと効く。これを、後の段階の候補にする（ConSh の `(( ))` は、ランタイムを ConSh のモジュールに持っているので、この時間はかからない）。
* **設計と変えたところ**：
    * SJS の字句を読むループ（`Sjs_lex_loop`。テンプレート文字列を扱う）と、2 項演算を読むループ（`P_binary_rest`。`==`、`instanceof`、for の `in` の検査が途中にある）は、SJS に残し、キットの手伝いの関数を呼ぶ形にした。キットの関数から SJS を呼び返すと、ステップが増えるため。`Kit_parse_binary` は、test/fast17 で確かめる。
    * consh/include/consh_jsstmt.se の、字句の部品の名前を呼ぶ所（`Tok_text`、`Tok_kind`、`Sjs_starts`、`Sjs_words`、`Cat3`）を、キットの名前に変えた（4.3 節は「consh/include は変えない」としていたが、変えたのは名前だけで、PDict の名前は変えていない）。
    * 字句の部品だけが使う PDict の名前（`"sjs_lex_n"`、`"sjs_lex_starts"`）は、`"kit_lex_n"`、`"kit_lex_starts"` にした（ほかからは読まない）。`"sjs_pos"`、`"sjs_handlers"`、`"sjs_gen"`、`"gen_quote_specials"`、`"conjs_sched"` は変えていない。
    * bin/lang-step-check は、wasm 版の VM だけで流す sjs/test の step9 と step10 を除く（Rust 版の VM では、燃料のテストの大きなループを ConMa のまま回すので、数えると何分もかかる）。
    * 変換の時間を ConSh の `(( ))`（sjs/bench/consh の nbody.sh）で前と後を交互に測ることは、まだしていない。変換のステップ数が +0.05% なので、時間の違いは測れないほど小さいと見込む。

