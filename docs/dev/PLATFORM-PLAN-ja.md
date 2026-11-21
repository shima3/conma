# いろいろなプラットフォームと、ConMa に変換する言語の計画

ConMa を、いろいろなプラットフォームで、そのプラットフォームに固有の機能（ブラウザの canvas、OS のファイル、モバイルの通知など）も使って動かせるようにする。また、JavaScript の部分集合 ConJS（と SJS）を ConMa に変換して、ConMa が動くどのプラットフォームでも動かせるようにする（2026-11-19。2026-11-20 に、ConMa に変換する言語の目標を ConJS に絞った。6 節の 2）。

**状態**：段階 P1 と P2 を作った（2026-11-19。8 節）。6 節の 1〜3 を決めた（P3 はブラウザの canvas と、待つホストの呼び出しの形から。ConJS を作る。4.1 節）。段階 P3 を作った（8.3 節）。段階 P3b（ConJS）を、4.2 節の設計（PR shima3/conma#3 で M2 がレビューした）で作った（8.4 節）。段階 P4（言語キット）を、4.3 節の設計（PR shima3/conma#5 で M2 がレビューした）で作った（8.5 節）。6 節の 4 と 5 も決めた。段階 P5（ConJS をふだん使えるものにする）を、4.4 節の設計（PR shima3/conma#8 で M2 がレビューした）で作った（8.6 節。6 節の 2 の見本の言語は作らないことにした）。段階 P5b（ConJS が受け付ける JavaScript を広げる）を、4.5 節の設計（PR shima3/conma#10 で M2 がレビューした）で作った（8.7 節）。段階 P5c（ConJS で、ブラウザの入力を待つ）を、4.6 節の設計（PR shima3/conma#12 で M2 がレビューした）で作った（8.8 節）。段階 P6a（端末）を、4.7 節の設計（PR shima3/conma#15 で M2 がレビューした）で作った（8.9 節）。段階 P8（ConJS のファイルを 1 行ずつ読む）を、4.8 節の設計（PR shima3/conma#20 で M2 がレビューした）で作った（8.10 節）。段階 P7（使う人がホストの関数を足す文書と口）を、4.9 節の設計（PR shima3/conma#22 で M2 がレビューした）で作った（8.11 節）。M2 のレビュー（気付いたことと、6 節への案）は、[docs/dev/REVIEW-ja.md](REVIEW-ja.md) の 2026-11-19 の節にある（P1 で答えたことは、その「対応」に書いた）。

## 1. 目的

* **A. プラットフォーム**：同じ ConMa のプログラムが、どの VM とプラットフォームでも同じに動く（今のコア ConMa と拡張 ConMa）うえに、プラットフォームに固有の機能も使える。使えない所では、プログラムがそれを知って、別の道を選べる。
* **B. ConMa に変換する言語**：JavaScript の部分集合（ConJS と SJS）を、変換器とランタイムを ConMa で書いて ConMa に変換し、ConMa のすべてのプラットフォームで動かし、A の固有の機能も使えるようにする。中心は ConJS（4.1 節）で、SJS（sjs/README-ja.md）がその最初の例である。2 つが共有する部品を、言語キットにする（4 節）。

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

SJS の作りから、ConJS と SJS が共有する部品を取り出す（言語キット）。作ろうとしているのは ConJS で、どんな言語にも使えるキットは目指さない（6 節の 2）。

* **字句解析**：sjs_lex から、行と桁を持つ字句、空白とコメント、数と文字列のリテラルの部品を取り出す。
* **構文解析**：sjs_parse の再帰下降と演算子の優先順位の部品（節は `"t"` を持つ Dict）。
* **ConMa のコードを作る**：sjs_gen から、関数、継続、変数、SInfo（元の言語の行と桁）を書く部品。今は ConMa のソースの文字列を作ってからモジュールにするが、構文木から直接モジュールを作ると速いかもしれない（測ってから決める）。
* **ランタイム**：値の種類、エラーとその位置、例外の型。
* **動かす道具**：sjs/sjs を一般にしたもの（案：`bin/lang 言語 ファイル`）。変換器のモジュールを作ってキャッシュし、変換して、選んだ VM で動かす。
* **テストとベンチマークの型**：Node.js と出力を比べる（sjs/test の compare.sh の形）。ConJS でも、`spawn`、`join`、`host` と待つ呼び出しのライブラリを使わないプログラムは比べられる（4.1 節）。ベンチマークは、bench/ と同じ計算で比べる。
* **置き場所**：sjs/ と conjs/ は、同じ形のディレクトリ（`include`、`test`、`bench`、`README-ja.md`）で、リポジトリのルートに置く。言語キットの部品は include/lang/ に置く（6 節の 4 と 5）。
* **ConSh から使う**：今は SJS の `(( ))` がある。ConJS を ConSh から使う形は、まだ決めていない。
* **見本の言語**（作らないことにした、2026-11-20。6 節の 2）：初めは、キットの部品が言語によらないことを確かめるために、小さな言語を 1 つ作るつもりだった。

### 4.1. ConJS（軽量スレッドを持つ JavaScript の部分集合）

**ConJS** は、JavaScript の部分集合で、協調的に切り替わる軽量スレッド（以下「処理」）と、その処理だけを止めるブロッキング呼び出しを持つ。待つ呼び出し（fetch、タイマー、ファイル、アニメーションの 1 コマ）まで、`await` を書かずに、ふつうの呼び出しとして書ける（`const text = fetch(url).text()`）。待っている間は、その処理だけが止まり、ほかの処理は動く。今は SJS（JavaScript の安全なサブセット）の変換器とランタイムを使って作っているが、SJS とは別の言語で、これから受け付ける JavaScript を広げていく。用語の「直接スタイル」は、ここでだけ「待つ呼び出しまで直接スタイルで書ける」の意味で使う。

* **ConMa で作る甲斐**：JavaScript では、待つ処理が、呼び出す側に `async` と `await` を広げていく（関数の色）。ConMa は CPS で、どの呼び出しの途中でも継続を持つので、待つ処理を「待つ」ことにするだけで、関数に色が付かない（ConSh のパイプと `$( )` が、もうこの形で動いている）。Go のゴルーチンや Java の仮想スレッドの考え方を、JavaScript の構文で、ConMa の動くどのプラットフォームでも試せる。ふつうの JavaScript ではできず、ほかの言語の上に作るのも難しい。
* **協調的に切り替える**（決めた、2026-11-19。M2 の意見の 3）：JavaScript では、2 つの `await` の間のコードは、割り込まれずに最後まで動き（run-to-completion）、プログラムはそれを頼りにオブジェクトを共有する。ConMa の VProc はステップの数で切り替わるので、処理ごとに VProc を使うと、どこでも入り混じる。そこで、ConJS の処理（並べて動かすものも）は、すべて 1 つの VProc の中で動かし、ランタイムが継続で切り替える。切り替わるのは、待つ呼び出しの所だけである。待つこと自体は、待つためだけの小さな VProc に任せ、その VProc が結果をチャネルに入れる。ランタイムは、動かせる処理がなくなったときだけ、そのチャネルを待つ。待つことは並行に進み、ConJS の処理は協調的に切り替わる。VM とスケジューラは変えない。
* **使うもの**：字句解析、構文解析、ランタイム（数、文字列、配列、オブジェクト、例外）の多くは SJS のもの。待つ呼び出しは、P3 で決めるホストの待つ呼び出し（6 節の 3）。
* **動かす順**（M2 の意見（docs/dev/REVIEW-ja.md の 2026-11-19（4））の 1）：動かせる処理の列は FIFO にする。`spawn` した処理は列の最後に入り、呼んだ側が次に待つまで動き続ける（JavaScript のマイクロタスクと同じ）。待つ呼び出しの結果が届いた処理も、列の最後に入る（いくつか届いていれば、チャネルに届いた順）。これで ConJS のプログラムの動きは、ホストが結果を返す順だけで決まる。決定的でないのは、待つための小さな VProc がチャネルに入れる順だけなので、テストではテストのホストの関数（`test.*`）で返す順を決める。
* **Promise**（決めた、2026-11-19。M2 の意見の 2）：`Promise` と `async` は受け付けず、エラーにする。並べて待つには `spawn` と `join` を使う。Promise を受け付けると、`await Promise.all([fetch(a), fetch(b)])` が、エラーにならずに 2 つの fetch を順に待ち、遅くなるだけで気付きにくい。`await` だけは、SJS との行き来のために「その場で待つ」と読んでもよいが、これも P3b で決める。P3b の設計で、`await` も受け付けず、`.then` は除かないことにした（4.2 節）。
* **spawn した処理の例外**（M2 の意見の 3）：`join` で、その例外をもう一度投げる。`join` されないまま例外で終わった処理があれば、プログラムの終わりに報告する（JavaScript の unhandled rejection のように）。P3b のテストに、spawn した処理の例外の場合（join する場合としない場合）を入れる。
* **スケジューラを部品にする**（M2 の意見の 4）：ConJS の切り替え（待つ呼び出しで継続を取り、列に入れ、ほかの処理を動かす）は、「待つ」というエフェクトのハンドラそのものである。P3b では ConJS の中に作るが、継続の列とチャネルを待つことを、ConJS の構文によらない部分として分けておき、P4 で言語キットの部品として取り出す。
* **ConJS を広げる道**（決めた、2026-11-20。作者の方針で、M2 が PR shima3/conma#8 のレビューの 3 で伝えた）：新しい JavaScript は、まず ConJS に足す。そのうち安全なものを SJS に移し、2 つの言語で共有する。`class`、`new`、`this` は、SJS が安全のために除いているもの（docs/consh/dev/SAFEJS-PLAN-ja.md の 3.2）なので、まず ConJS だけに足す候補になる。SJS に移すかどうかは、ものごとに決める（SJS は ConSh の `(( ))` の言語でもある）。
    * 変換器とランタイムは、足したものをいつも扱えるようにし、SJS だけが、今の名前の検査（sjs_parse.se）で除く。安全だと決めたら、SJS の検査から外すだけで共有できる。4.2 節の「印を見るのは、受け付けない構文の検査と、作ったソースの初めと終わりだけにする」は保てる。
    * ただし、計算したキーの `constructor`、`prototype` などは、変換するときでなく、ランタイムで止めている（sjs_rt.se）。ConJS で `class` を使えるようにすると、ConJS ではこれらのキーを通し、SJS では止める必要があり、印をランタイムで見ることになる（メンバーを読む、よく通る所に検査が増える）。`class` を足す段階の設計で決め、SJS が遅くならないことを sjs/bench で確かめる。
    * 確かめ方：ConJS だけに足したものは、sjs/test では Node.js と比べられない。`spawn`、`join`、`host` と、待つ呼び出しのライブラリ（4.4 節）を使わない ConJS のプログラムは JavaScript のプログラムでもあるので、sjs/test/compare.sh と同じ形で Node.js と比べる場合を、conjs/test に置く。
    * 初め（3709a40）は、M2 の勧め（docs/dev/REVIEW-ja.md の 2026-11-20 の 2 の a）で「ふつうの JavaScript は SJS にも足す」としていた。作者の方針で順を逆にした（M2 も勧めを取り下げた）。
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

### 4.4. 段階 P5 の設計（決めた、2026-11-20。M2 のレビューは PR shima3/conma#8）

P5 では、ConJS をふだん使えるものにする。することは 2 つある：ランタイムを先にモジュールにしておき、プログラムを動かし始めるまでの時間を短くすること（1）と、待つ呼び出しのライブラリ（タイマー、時刻、ファイル、fetch）を足すこと（2）。6 節の 2 の見本の言語は作らない（6 節の 2 に書いた）。

**1. ランタイムを先にモジュールにしておく**

今は、プログラムを動かすたびに、変換した ConMa のソースと、それが include するランタイム（sjs_rt.se、conjs_rt.se など）を、まとめてフロントエンド（`conma build`）に通している。`console.log("hi")` だけのプログラムで、変換が約 0.02 秒、フロントエンドが約 1.41 秒、動かすのが約 0.01 秒かかる（Rust 版の VM、Apple M1。2026-11-20）。できたモジュール（約 1.7 MB）を読むのは速く、時間のほとんどはフロントエンドがランタイムを通す分である。

* **変換したコードは、ランタイムを include しない**：ConSh の `(( ))`（sjs/include/sjs_gen.se の `Sjs_translate_module`）と同じく、変換したコードは `Conjs_entry` だけを定義する。`Conjs_entry` は、引数の Dict の `"rt"` からランタイムの関数を受け取り、同じ名前の局所変数に束縛してから、今の main と同じことをする。
* **起動部**（conjs/conjs_run.se）：ランタイムと、変換したファイルを `__VProc_spawn__` で動かすことだけを持つ ConMa のプログラム。そのモジュールを 1 度だけ作り、キャッシュに置く（bin/lang が変換器のモジュールを置くのと同じ所）。変換は、今のとおり bin/lang が Rust 版の VM で行い、変換したファイルを起動部に渡す。起動部は、それを新しい VProc として動かす。そのとき、ランタイムの関数の Dict を渡し、VProc が終わるのを待つ。フロントエンドが通すのは、変換したコードだけになる。
    * 変換を起動部に入れない（M2 の PR shima3/conma#8 のレビューの 1）：起動部は選んだ VM で動くので、入れると、変換も選んだ VM で行うことになる。Python 版の VM では、20 行の変換だけで約 5.6 秒かかり（Rust 版は約 0.045 秒）、今より遅くなる。
    * `__VProc_spawn__` は、どの VM でもモジュールを作れる：Rust 版は `conma build` を別のプロセスで動かし、wasm 版（Node.js）は vm/wasm/vm-node.js の `buildModule` で、Python 版は bin/includer と bin/resolver で作る。どの VM も変えない。
    * 小さなファイルを `conma build` に通すのは約 0.01 秒である。Rust 版の VM で、`console.log("hi")` は、今の約 1.45 秒から、約 0.1 秒（変換器のモジュールを読んで変換する約 0.05 秒を含む）になると見込む。ConSh の `(( ))` は、同じ形で 1 回約 0.06 秒（Rust 版）、約 0.08 秒（wasm 版）かかる（M2 が Apple M2 で測った）。
    * Python 版と wasm 版の VM の時間も測り、8.6 節に書く。今は、Rust 版が約 1.45 秒、wasm 版が約 1.55〜2.23 秒、Python 版が約 4.2 秒である（M2 が Apple M2 で測った）。
* **1 つのモジュールにまとめる道も残す**：ランタイムと変換したファイルを include し、main から `Conjs_entry` をランタイムの関数の Dict を付けて呼ぶ、小さな ConMa のファイルで、今と同じく 1 つのモジュールにできる。ブラウザの見本（canvas_demo.con.js）は、この形で作る（vm/wasm/web/build.sh）。どちらの道でも、変換したコードは同じである。
* **ランタイムの関数の名前の一覧**（`Conjs_rt_names`。SJS の `Sjs_rt_names` に、ConJS の名前を足したもの）：変換したコードが一覧にない名前を呼ぶと、動かして初めて分かる（定義のない大域変数）。そこで、テストのすべてのプログラムを変換し、ランタイムなしでモジュールにして、定義のない大域変数がプリミティブと一覧の名前だけであることを確かめる（conjs/test の検査に足す）。あわせて、一覧のすべての名前が、ランタイムの関数の Dict（`Conjs_rt_dict`）のキーにあることも確かめる。一覧にあって Dict にない名前は null に束縛され、その名前を呼ぶまで分からないため（M2 のレビューの 4）。Dict を一覧から作れるなら、そうする。
* **道具**：lang.conf に `runner=conjs_run.se` があれば、bin/lang は、変換器でファイルを変換してから（Rust 版の VM）、起動部のモジュールを作ってキャッシュに置き（Python 版の VM のためには、bin/to-core を通したもの）、`VM --main 起動部のモジュール 変換したファイル` で動かす。`--emit` は、変換したコード（`Conjs_entry` の形）を出す。sjs の lang.conf には `runner` を書かず、今のまま動かす。
* **変わること、変わらないこと**：
    * ConJS が作るコードは変わる（main の代わりに `Conjs_entry`）。SJS が作るコードは、今と 1 文字も違わないことを bin/lang-emit-check で確かめる。
    * 出力、終了ステータス、受け手のない例外の報告、行き詰まりのメッセージは、今と同じ（conjs/test の今の場合のログが変わらない）。
    * ステップ数は、始めに約 100 の名前を Dict から引く分だけ増える。ランタイムの関数を大域変数でなく局所変数から呼ぶので、時間が変わらないかを、sjs/bench の loop、fib、nbody を ConJS として動かして、今の形と比べる。遅くなるなら、この形を考え直す。

**2. 待つ呼び出しのライブラリ**

名前は JavaScript にならう。JavaScript と違うのは、Promise を返さず、結果が届くまで、呼んだ処理だけが待つことである。

* `sleep(ms)`：ms ミリ秒、呼んだ処理だけが待つ（JavaScript にはない。`spawn` と同じく ConJS の名前）。
* `setTimeout(f, ms)`：`spawn(() => { sleep(ms); f(); })` と同じ。処理を返す（`join` できる。JavaScript は数を返す）。`f` が投げた例外は、JavaScript のようにすぐ止めるのではなく、join されない処理の例外として、プログラムの終わりに報告する。この 2 つを、conjs/README-ja.md の JavaScript との違いに書く。`clearTimeout` は作らない。
* `Date.now()`：今の時刻（1970 年 1 月 1 日からのミリ秒）。
* `fs.readFile(path)`、`fs.writeFile(path, text)`：ファイル全体を文字列で読み書きする（Node.js の名前。`fs` はグローバルに置く）。ふつうのファイルはすぐ読み書きできるので、処理は切り替わらない。読めないときは Error を投げる。ブラウザのページ（ホストのファイルを渡さない wasm 版の VM）では、VM のメモリの中のファイルを読み書きする（ページを閉じると消える）。そう動くことを P5 で確かめ、conjs/README-ja.md に書く。
* `fetch(url, options)`：HTTP の要求。返すのは Response のようなオブジェクト（`status`、`ok`、`text()`、`json()`）で、本体は返す前に全部読んでおく（`text()` は待たない）。options は `method`、`headers`（オブジェクト）、`body`（文字列）。つながらないときは TypeError を投げる（JavaScript と同じ）。
* これらは、SJS が除くホストのもの（docs/consh/dev/SAFEJS-PLAN-ja.md の 3.2）なので、ConJS だけに足し、SJS には移さない（4.1 節の「ConJS を広げる道」）。

作り：

* **ホストの関数**（P2 と P3 の口。名前を docs/PRIMITIVES.md の `__Feature_available__` の一覧に足す）：
    * `time.now`（すぐ返る。機能 `"time"`）：Rust 版と Python 版は VM の表に足す（Rust の表に足すので、transpiled VM も作り直す。P2 と同じ）。wasm 版は vm/wasm/conma.js の既定のホストの関数にする（ページのフックがなくても使える）。
    * `time.sleep ms`（待つ呼び出し。機能 `"time"`）：Rust 版と Python 版は、決まった時間の後に結果をパイプに書く（`test.later` の仮想の時計ではなく、本当の時計）。wasm 版は conma.js の `setTimeout`。
    * `http.fetch url method headers body`（待つ呼び出し。機能 `"fetch"`）：wasm 版だけ（conma.js の既定のホストの関数で、Node.js 18 以降とブラウザの `fetch`）。結果は、状態の番号と本体の文字列のリスト。
* **ConJS のランタイム**（conjs/include/conjs_lib.se）：上の名前を、`host.call` と `host.wait` と同じ道で、ホストの関数につなぐ。`fs` は ConMa の `__OS_open__`、`__OS_read__`、`__OS_write__` で書く（どの VM も変えない）。`fetch` は、ホストに `http.fetch` があればそれを使い、なければ `curl` を子のプロセスとして動かす（`__OS_spawn__`。Rust 版と Python 版）。`curl` も動かせなければ TypeError を投げる。`curl` は、待つための VProc の中で動かし、終わるまで、呼んだ処理だけが待つ。

**テスト**（conjs/test に、step7〜9 を足す。Rust 版、wasm 版（Node.js）、Python 版の VM で同じログ）

* step7（タイマーと時刻）：本当の時計で確かめるのは、負荷で揺れないことだけにする（M2 のレビューの 2。テストはほかの場合と並べて動き、Python 版の VM は Rust 版の 100 倍ほど遅い）：`Date.now()` の差が `sleep` の長さ以上であること、`sleep` の間にほかの処理が動くこと（数を数えて確かめる）、`setTimeout` の関数が呼ばれ、`join` できること。処理の終わる順は、今のとおり `test.later`（仮の時計）で確かめる。
* step8（ファイル）：一時ディレクトリに書いて読むこと、ないファイルを読むときの Error。
* step9（fetch）：テストが 127.0.0.1 に小さな HTTP サーバ（Python で書く。conjs/test/step9/server.py）を立て、GET の本体と状態、404（`ok` が偽で、例外は投げない）、POST の本体とヘッダ、つながらない所の TypeError を確かめる。`curl` の道では、`curl` の終了コードで、つながらない場合と HTTP のエラーの状態を分ける。Rust 版と Python 版は `curl` の道、wasm 版はホストの道を通る。
* `spawn`、`join`、`host` とライブラリを使わない場合は、sjs/test/compare.sh と同じ形で Node.js と比べる（4.1 節）。
* step1〜6 の今のログが変わらないこと（起動部の道）。1 つのモジュールにまとめる道でも、step1〜6 を動かす。ランタイムの関数の名前の一覧の検査。
* 動かし始めるまでの時間を測り、8.6 節に書く（`console.log("hi")`、Rust 版の VM）。

**終わりとすること**（5 節の表）：上のテストが 3 つの VM で通る。`console.log("hi")` を動かす時間が 0.2 秒以下になる（Rust 版の VM、Apple M1）。Python 版と wasm 版の VM でも、今より遅くならない。sjs/bench の loop、fib、nbody を ConJS として動かして、今の形より遅くならない。sjs/test、consh/test が通り、SJS が作るコードが変わらない。

**P5 でしないこと**：受け付ける JavaScript を広げること（`class`、`new`、`this` など。次の段階で、まず ConJS に足す候補。4.1 節の「ConJS を広げる道」）。ブラウザのページの中で ConJS を変換すること。ブラウザのモジュール（約 8 MB）を小さくすること。sjs/sjs を起動部の形にすること（同じ作りでできるので、ConJS で確かめてから決める）。fetch の本体を少しずつ読むこと、`AbortController`、`clearTimeout`。ファイルのストリームとディレクトリ。ConJS から子のプロセスを動かすこと。

**M2 に聞きたいこと**：起動部が、変換したコードを `__VProc_spawn__` で動かす形（ConSh の `(( ))` と同じ）と、名前の一覧の確かめ方。ConJS が作るコードが変わること（bin/lang-emit-check は SJS だけで確かめる）。ライブラリの名前と範囲（`sleep` は JavaScript にない名前、`fs` は Node.js にならう、fetch の本体を先に全部読む）。ネイティブの VM の fetch を `curl` で行うこと。`time.now` と `time.sleep` を、Rust 版と Python 版の VM に足すこと。

### 4.5. 段階 P5b の設計（決めた、2026-11-20。M2 のレビューは PR shima3/conma#10）

P5b では、ConJS が受け付ける JavaScript を広げる（4.1 節の「ConJS を広げる道」。まず ConJS に足し、そのうち安全なものを SJS に移して共有する）。最初に足すのは、ふつうの JavaScript のプログラムでいちばんよく使い、SJS が安全のために除いているもの：`class`、`new`、`this`、`instanceof`、エラーのコンストラクタ。あわせて、SJS にもない、ふつうの JavaScript の関数 `JSON.parse` と配列の `sort` を足す。プラットフォームを増やす段階（5 節の表の P6）は、その後にする。

**足すもの**（ConJS だけ。SJS は、今の名前の検査（sjs_parse.se の `P_check_name_sjs`）で、今までどおり除く）

* **`class`**（宣言と式）：`constructor`、メソッド、`static` のメソッド、インスタンスのフィールド（`x = 1;`。コンストラクタの前に、書いた順に初期化する）、`extends`（クラスか、組み込みのエラーのコンストラクタだけ）、コンストラクタの中の `super(...)`、メソッドの中の `super.m(...)`。`extends` のあるクラスでコンストラクタを書かなければ、`constructor(...args) { super(...args); }` と同じ。
* **`new C(...)`**：C はクラスか、組み込みのエラーのコンストラクタ（`Error`、`TypeError`、`RangeError`、`SyntaxError`、`ReferenceError`）。ふつうの関数を `new` すると TypeError（`... is not a constructor`。JavaScript では関数もコンストラクタになるが、ConJS では、作るのはクラスだけにする）。クラスを `new` なしで呼ぶと TypeError（JavaScript と同じ）。
* **`static` のメソッドの継承**：`class B extends A` の `B.f()` で、A の `static` のメソッド f を呼べ、その `this` は B（JavaScript と同じ）。
* **`this`**：クラスのコンストラクタ、メソッド（`this` はインスタンス）、インスタンスのフィールドの初期化（`x = this.y * 2;`。`this` はインスタンス）、`static` のメソッド（`this` は、メソッドを読んだクラス）の中と、その中のアロー関数の中だけ。ふつうの `function` の中とプログラムの上の階層の `this` は、変換するときのエラー（`conjs: this outside a class`）。`extends` のあるクラスのコンストラクタで、`super(...)` の前に `this` を使うと ReferenceError（JavaScript と同じ）。
* **`instanceof`**：右がクラスか、組み込みのエラーのコンストラクタ（ほかは TypeError）。インスタンスは、クラスと、`extends` でたどる親で真。ランタイムが投げるエラー（`Sjs_error_object` で作るもの）は、`Error` と、名前が同じエラーのコンストラクタ（`TypeError` など）で真。
* **エラーのコンストラクタ**：`new Error(message)` などは、今の SJS のエラーのオブジェクト（`name` と `message` を持ち、`Object.keys` と JSON に出ない）と同じものを作る。`class MyError extends Error` のインスタンスもエラーのオブジェクトで、`throw` でき、受け手のない例外の報告にも `message` が出る。
* **`JSON.parse(text)`**（今は ConJS の Response の `json()` の中だけにある読み手。conjs/include/conjs_lib.se）。キーが `constructor` や `__proto__` のオブジェクトも、JSON のデータとして正しいので、そのまま作る（プロトタイプはないので安全）。そのプロパティを読むと、今の実行時の検査で TypeError（`the property constructor is not in SJS`）になり、`Object.keys` には出る。SJS では、リテラルと計算したキーで、こういうキーのオブジェクトを作る道がなかったので、新しい場合として sjs/test に足す。
* **配列の `sort(compare)`**：安定な並べ替え。`compare` がなければ、JavaScript と同じく文字列にして比べ、比べ方は SJS の文字列の `<` と同じにする（JavaScript は UTF-16 の単位で比べるので、基本多言語面の外の文字だけ順が違うことがある。違えば README に書く）。`undefined` は、`compare` を呼ばずに最後に置く（JavaScript と同じ）。

**足さないもの**（後の候補）：getter と setter、`#` で始まるプライベートなフィールド、`static` のフィールドとブロック、計算した名前のメソッド、`new.target`、ふつうの関数とオブジェクトのリテラルのメソッドの `this`、`Map` と `Set`、`Symbol` と反復のプロトコル、正規表現。プロトタイプには、どの形でも触れられない：`prototype`、`constructor`、`__proto__` の名前のプロパティは、今までどおり、変換するときと（計算したキーは）実行時に止める。

**作り**

* **構文解析と変換**（sjs/include）：印が ConJS のときだけ、`class`、`new`、`this`、`super`、`instanceof` を名前の検査で通し、構文解析とコードの生成が、それらの節を扱う。SJS のプログラムは、今と同じ道だけを通り、作るコードは 1 文字も変わらない（bin/lang-emit-check で確かめる）。作るコードは、ConJS のランタイムの関数（conjs/include/conjs_class.se。`Conjs_class`、`Conjs_new`、`Conjs_super_call`、`Conjs_super_get`、`Conjs_instanceof`）を呼ぶ。
* **値の表し方**：
    * クラスは、関数の値（`"t"` が `"function"`。`typeof` は `"function"`）で、`"cls"` に、メソッド（名前 → `this` と引数の LList を受け取る ConMa の関数）の Dict、`static` のメソッドの Dict（同じ形）、コンストラクタ、フィールドの初期化、親を持つ。`static` のメソッドは、関数のメンバー（`"members"`）には置かない。そのため、`Object.keys(C)` に出ない（JavaScript と同じ）。
    * インスタンスは、今のオブジェクト（`"t"` が `"object"`）に、隠したキー `"cls"`（クラス）を足したもの。`typeof`、`Object.keys`、`JSON.stringify`、スプレッドは、今のオブジェクトと同じく、自分のプロパティだけを見る（JavaScript と同じ）。
    * メソッドを読む（`o.m`）と、自分のプロパティになければ、`"cls"` からクラスと親をたどってメソッドを探し、`this` を o にした関数を返す。そのため、取り出したメソッドを後で呼んでも、`this` は o のまま（JavaScript では undefined になる）で、読むたびに新しい関数を作るので、`o.m === o.m` は偽（JavaScript では真）。この 2 つを README の JavaScript との違いに書く。`in` も、同じくメソッドを見る。
    * `static` のメソッドを読む（`C.f`）と、関数のメンバーになければ、`"cls"` から C と親をたどって探し、`this` を C にした関数を返す。
    * `console.log` は、Node.js に合わせて、インスタンスを `Point { x: 1, y: 2 }`、クラスを `[class Point]`（`extends` があれば `[class B extends A]`）と出す。sjs_inspect.se が、オブジェクトと関数を出す所で隠したキー `"cls"` を見る（SJS の値は持たないので、出すものは変わらない）。エラーは、Node.js がスタックトレースを出すので、テストでは `console.log` に渡さず、`e.message` や `` `${e}` `` を出す。
* **SJS の道への影響**：メソッドを探すのは、メンバーを読んで、自分のプロパティになかったとき（今は undefined を返す所）だけにする。探す所は、sjs_rt.se の `Sjs_get_member` の、オブジェクトの 2 つの道（初めの「キーが文字列」の速い道と、`Sjs_key_string` の後の道。どちらも `Sjs_obj_get` の後）と関数の道（`Sjs_members_find` で見つからなかった後）、`in` の `Sjs_in_op`（`Sjs_obj_has` が偽の後）だけ。`Sjs_obj_get` と `Sjs_obj_has` は、自分のプロパティだけを見るままにする（`Sjs_obj_set` は `Sjs_obj_has` で名前が新しいかを決めるので、メソッドを見ると、`o.m = 1` の m が名前の一覧に入らず、`Object.keys` と JSON に出なくなる。ほかにもランタイムの 17 か所と sjs_inspect.se が使う）。SJS の値は `"cls"` を持たないので、結果は変わらず、ステップ数は、ないメンバーを読む所でだけ 1 ステップほど増える。メンバーがある所（いちばん多い）は変わらない。これを bin/lang-step-check で確かめる。プロトタイプのキーを止める実行時の検査（M2 の PR shima3/conma#8 のレビューの 3）は、プロトタイプを見せないので、印をランタイムで見る必要がなく、今のまま変えない。
* **`JSON.parse` と `sort`**：プロトタイプにもホストにも触れない、ふつうの JavaScript の関数なので、作るときに安全と決め、SJS のランタイム（sjs_rt.se）に置いて、すぐに SJS と共有する（4.1 節の「安全なものを SJS に移す」を、この 2 つは足すときに行う）。SJS では sjs/test で Node.js と比べる。conjs_lib.se の JSON の読み手は、`JSON.parse` に移す。

**テスト**（conjs/test の step11〜13。Rust 版、wasm 版、Python 版の VM で同じログ）

1. step11（クラス）：コンストラクタ、メソッド、フィールドとその初期化の `this`、`static` とその継承、`extends` と `super(...)`、`super.m(...)`、`instanceof`、アロー関数の `this`、`console.log` のインスタンスとクラス、`Object.keys` のインスタンスとクラス。JavaScript との違い（取り出したメソッドの `this`、`o.m === o.m`）は、別の場合にする。
2. step12（エラー）：`new Error(...)` などと `instanceof`、`class MyError extends Error`、受け手のない例外の報告。変換するときのエラー（ふつうの関数の `this`、計算した名前のメソッド、getter）と、実行時のエラー（関数の `new`、クラスの `new` なしの呼び出し、`super(...)` の前の `this`）。
3. step13（`JSON.parse` と `sort`）。sjs/test にも、同じものの場合と、`constructor` のキーの場合を足す。
* **Node.js と比べるもの**：compare.sh は標準出力だけを比べるので、変換するときのエラーの場合と、JavaScript との違いの場合は比べられない。ステップのディレクトリに、Node.js と比べる場合の名前を並べたファイル node.txt を置き、compare.sh は、ファイルを書かなければ、すべての step の node.txt の場合を比べる（step10 にも node.txt を置く）。
* bin/lang-emit-check（SJS が作るコードは同じ）、bin/lang-step-check（SJS のステップ数は、ないメンバーを読む所でだけ増える）。sjs/bench の nbody を、物体をクラスにして ConJS で書いたもの（conjs/bench/nbody_class.con.js）の時間を、今の nbody と比べて、8 節に書く。メソッドを呼ぶたびに `this` を付けた関数を作るので、遅ければ、P5b の中で作り方を変える。そのときの案：ConJS のプログラムだけ、`o.m(...)` を ConJS のランタイムの関数 `Conjs_call_member` で呼ぶ。この関数は、メソッドを見つけたら、関数を作らずに `this` と引数で直接呼ぶ（SJS が作るコードとステップ数は変わらず、ConJS が作るコードだけが変わる）。

**終わりとすること**（5 節の表）：上のテストが 3 つの VM で通り、node.txt に並べた場合が Node.js と同じ出力。SJS が作るコードが変わらず、sjs/test、consh/test、consh/js/test が通る。sjs/bench が遅くならない。

**M2 のレビュー**（PR shima3/conma#10）：分け方、ふつうの関数の `this` をエラーにすること、取り出したメソッドの `this`、メソッドを探す作り、`JSON.parse` と `sort` を足すときに SJS と共有すること（ユーザーにも確かめた）に賛成をもらった。指摘を受けて、メソッドを探す所（`Sjs_get_member` と `Sjs_in_op` だけ）、`console.log` の出し方、`o.m === o.m` の違いと遅いときの案、`static` のメソッドの継承と `Object.keys(C)`、`JSON.parse` の `constructor` のキー、`sort` の比べ方と `undefined`、Node.js と比べる場合の選び方（node.txt）、フィールドの初期化の `this` を書き足した。

### 4.6. 段階 P5c の設計（決めた、2026-11-21。M2 のレビューは PR shima3/conma#12）

P5c では、ConJS のプログラムが、ブラウザのキーとマウスの入力を、待つ呼び出しとして待てるようにする。見本として、入力を待つ対話のゲームを ConJS で書き、ブラウザで動かす。作者の方針（2026-11-21）：「同期処理なら簡単だが、JavaScript では非同期処理が要るので複雑になるプログラム」のうち、入力を待つ対話を選び、それを動かせるように ConJS を広げる。プラットフォームを増やす段階（5 節の表の P6）は、その後にする。

**見本のプログラム**（vm/wasm/web/stars.con.js。オリジナルの小さなゲーム「星ひろい」）

格子の上で、矢印のキーで自分（四角）を 1 マスずつ動かし、星をすべて拾うと次の面に進む。おばけ（2〜4 匹）は、それぞれ一定の間隔で自分に 1 マス近づき、ぶつかると終わる。タイトル、遊ぶ所、結果の 3 つの画面を順に回る。Escape で、どの画面からでもプログラムを終える。

```js
while (true) {
  if (!title()) break;            // 画面を描き、キーかクリックを待つ（Escape なら偽）
  const score = play();           // 遊んで、終わったら点を返す（Escape なら undefined）
  if (score === undefined) break;
  if (!result(score)) break;      // 点を出し、キーを待つ
}

function play() {
  const game = newGame();
  const ghosts = game.ghosts.map((g) => spawn(() => {   // おばけごとに 1 つの処理
    while (!game.over) { sleep(g.interval); moveGhost(game, g); }
  }));
  const painter = spawn(() => {                          // 1 コマごとに描く処理
    while (!game.over) { draw(game); host.wait("canvas.frame"); }
  });
  let quit = false;
  while (!game.over) {                                   // 自分は、キーを待って動く
    const e = nextEvent(100);                            // 100 ミリ秒でキーがなければ undefined
    if (e && e.type === "key" && e.key === "Escape") { quit = true; game.over = true; }
    else if (e && e.type === "key") movePlayer(game, e.key);
  }
  ghosts.forEach(join); join(painter);
  return quit ? undefined : game.score;
}
```

* **JavaScript で同じものを書くと**：画面の移り変わりは「今の画面」の状態の変数と、keydown のイベントの受け手の中の分岐になる。おばけの動きは `setInterval` の受け手に分かれ、遊ぶ所を抜けるときに `clearInterval` で止める。`async` と `await` で書いても、キーを 1 つ待つ Promise を自分で作り（受け手を足して、1 回で外す）、時間切れとの競争（`Promise.race`）も書くことになる。ConJS では、上のように、画面の順と、おばけごとの動きを、ふつうのループと関数で書ける。
* **割り込まれないこと**（4.1 節）：3 種類の処理が `game` を共有するが、切り替わるのは待つ呼び出し（`nextEvent`、`sleep`、`host.wait`）の所だけなので、`moveGhost` や `movePlayer` の途中で、ほかの処理が `game` を見ることはない。
* **待つ間隔**：自分の処理は、`nextEvent(100)` で 100 ミリ秒ごとに起き、おばけの処理が立てた `game.over` を見る。おばけにぶつかってから結果の画面に移るまで、最大 0.1 秒かかる（M2 のレビューの 4。0.5 秒では、遊ぶ人にわかる遅れになる）。
* **残った入力を捨てる**（M2 のレビューの 2）：列は入力を捨てないので、遊ぶ所で押し続けた矢印のキー（繰り返しも 1 つずつ入る）が、次の画面の「キーを待つ」にすぐ渡ってしまう。見本は、画面を描いた後に `while (pollEvent()) {}` で残りを捨ててから待つ。結果の画面は、300 ミリ秒 `sleep` してから捨てる（遊ぶ人の手が離れるまでのキーも捨てる）。ライブラリに足すものは要らない。
* 見本は、`nextEvent` のない所（Rust 版と Python 版の VM、Node.js）では、「this platform has no input」と出して終える（canvas_demo.con.js と同じ形）。

**足すもの**

* **ホストの関数**（プラットフォームの機能 `"input"`。新しい vm/wasm/host/input.js。canvas.js と同じ形で、`ConmaInput(canvas, options)` が `hostFunctions` と `hostFeatures` を返す）：
    * options（どれもなくてよい）：`keyTarget`（keydown を受け取るもの。既定は canvas。ページのどこでキーを押してもよいときは document）、`setTimeout` と `clearTimeout`（時間切れに使う時計。既定は globalThis のもの。テストは手で進める時計を渡す）。canvas は EventTarget であればよい（テストは Node.js の `EventTarget` を渡す）。input.js は、`KeyboardEvent` などの型を見ず、イベントの `key`、`clientX`、`clientY` だけを読む。
    * ホストは、keyTarget の keydown と、canvas の pointerdown を、届いた順に列にためる。待つ人がいないときに来た入力も捨てない。矢印のキー、スペース、Escape は、ページが動かないように `preventDefault` する。
    * `input.next ms`（待つ呼び出し）：列の先頭の入力を返す。列が空なら、次の入力まで待つ。ms が 0 以上の数なら、その時間に入力がなければ null を返す。ms が負の数なら、時間を限らない。
    * `input.poll`（すぐ返る）：列の先頭の入力か、空なら null。
    * 入力は LList：`("key" キーの名前)`（`KeyboardEvent.key`。`"ArrowUp"`、`"a"`、`" "`、`"Escape"` など。キーを押し続けたときの繰り返しも、1 つずつ入る）、`("click" x y)`（いつも canvas の座標。`clientX` と `clientY` から、canvas の `getBoundingClientRect()` の左上を引き、表示の大きさと canvas の幅や高さが違えば、その比で直す。`getBoundingClientRect` のない canvas（テストのまね）では、`clientX` と `clientY` をそのまま使う）。
    * 2 つの処理が同時に `input.next` を待てば、先に待ったほうが先の入力を受け取る。時間切れになった待ちは、列から外す（後の入力を取らない）。
* **ConJS のライブラリ**（conjs/include/conjs_lib.se。ConJS だけ。SJS には移さない）：
    * `nextEvent(ms)`：`input.next` を待ち、入力をオブジェクトにして返す：`{ type: "key", key: "ArrowUp" }`、`{ type: "click", x: 10, y: 20 }`。時間切れなら `undefined`。
    * ms（M2 のレビューの 3）：`undefined`（書かない）と `Infinity` は、時間を限らない（`input.next` に -1 を渡す）。負の数と NaN は 0（すぐに時間切れ。`setTimeout` と同じ）。数でなければ TypeError。`sleep` の `Conjs_ms` は `undefined` と無限を 0 にするので、そのままは使わない。
    * `pollEvent()`：`input.poll`。同じ形のオブジェクトか `undefined`。待たない。
    * 機能 `"input"` がなければ、どちらも TypeError（`nextEvent: this platform has no input`）。プログラムは `host.available("input")` で先に確かめられる。
    * 名前は JavaScript にない（`sleep`、`spawn` と同じく ConJS の名前）。
* **見本のページ**（vm/wasm/web/stars.html）：canvas と、canvas.js、input.js のホストの関数で、stars.con.js を変換したモジュールを動かす。canvas が keydown を受け取れるように `tabindex="0"` を付け、ページを開いたときと、クリックしたときにフォーカスを置く。vm/wasm/web/build.sh が、canvas_demo_conjs と同じ形で stars_conjs.module.se を作る。`.claude/launch.json` の `conma-canvas` で見る。
* canvas に描くのは、今の `host.call("canvas.draw", 命令のリスト)` のまま。JavaScript の canvas の API に似た形（`ctx.fillRect(...)`）は、P5c では作らない。

**テスト**

* **conjs/test/step14**（入力のライブラリ）：
    * case1：`nextEvent` と `pollEvent`。case1.host.js（wasm 版の VM のホストの関数。bin/lang-test の `case*.host.js`）は、本物の input.js に、Node.js の `EventTarget` と、手で進める時計を渡す（test/fast16/host.js が、本物の canvas.js に記録する context を渡すのと同じ形。M2 のレビューの 1）。これで、ConJS の側だけでなく、input.js の列と待ちの決め方も確かめる：待つ人がいないときの入力をためること、2 つの処理が同時に待つとき先に待ったほうが先の入力を受け取ること、時間切れの待ちを列から外して後の入力を取らないこと。ほかに、キー、クリック、列が空のときの `pollEvent`、ms の扱い（`undefined`、`Infinity`、負の数、数でないときの TypeError）。
    * 入力のまねの進め方：決まった手順の表を持ち、`input.next` が呼ばれた数で手順を進める。手順は「n 回目の `input.next` が呼ばれたら、次のタスクで、これらの入力を `dispatchEvent` で入れる」か「時計を ms だけ進める（時間切れの待ちが終わる）」。そのため、ログは本当の時計によらず決まる。手順を使い終わった後に `input.next` が呼ばれたら、Error（`input: the test has no more events`）にする（ずっと返さない Promise にすると、待っている呼び出しがあるので、スケジューラが終わらず、テストが止まる）。
    * Rust 版と Python 版の VM には機能 `"input"` がないので、同じ case1 が、`host.available("input")` の偽と、`nextEvent` の TypeError を出す（case1.log。wasm 版は case1.wasm.log）。
    * 入力のライブラリが使うホストの関数の名前（`input.next`、`input.poll`）と機能の名前（`"input"`）が、docs/PRIMITIVES.md にあることを、conjs/test/run_all.sh の今の検査で確かめる。
* **見本**：conjs/test/step14/case2 で、stars.con.js を、canvas のまね（test/fast16 の命令を記録する canvas と同じ形）と入力のまねで動かす（wasm 版の VM だけ）。決まったキーの列で、タイトルから遊ぶ所に入り、何マスか動き、Escape で終わることと、例外が出ないことを確かめる。描いた命令とおばけの動きは、本当の時計（`sleep`）によるので、ログに出さない。Rust 版と Python 版の VM では、「this platform has no input」を出して終わる。
* **ブラウザ**：アプリの中のブラウザで stars.html を開き、キーで自分が動き、おばけが近づき、ぶつかると結果の画面に移り、キーでタイトルに戻ることと、コンソールにエラーがないことを確かめる。
* conjs/test の今の場合、sjs/test、consh/test、consh/js/test が通り、SJS が作るコードが変わらないこと（bin/lang-emit-check）。

**終わりとすること**（5 節の表）：step14 が 3 つの VM で通る（VM ごとのログ）。見本がブラウザで遊べる。SJS が作るコードが変わらない。

**P5c でしないこと**（後の候補）：ネイティブの VM の端末でキーを待つこと（端末を raw にする OS の機能が要る。Rust 版と Python 版の VM の `input.*`）。キーを押しているかを調べる関数（`isKeyDown`）、keyup、マウスの移動、タッチの複数の指。canvas の API に似た描き方。DOM の要素（ボタンや文字の入力欄）を待つこと。音。

**M2 のレビュー**（PR shima3/conma#12）：見本の選び方と大きさ、`nextEvent` と `pollEvent` の名前と形、入力をためる列と待ちの決め方、P5c でしないことの分け方に、賛成をもらった。指摘を受けて、case1 で本物の input.js を時計のまねで動かすこと（手順を使い終わったときの Error も）、見本が画面を変えるときに残った入力を捨てること、`nextEvent` の ms の扱い、見本の Escape と待つ間隔（100 ミリ秒）、canvas の `tabindex` と、クリックの座標をいつも canvas の座標にすることを書き足した。

### 4.7. 段階 P6a の設計（決めた、2026-11-21。M2 のレビューは PR shima3/conma#15）

P6（プラットフォームを増やす）の最初の段階として、**端末**（macOS と Linux の Rust 版、transpiled VM、Python 版の VM）を足す。P5c で、ConJS の `nextEvent` と `pollEvent` は、ブラウザのページでだけ使えるようにした。P6a では、同じ 2 つを端末のキーでも使えるようにし、P5c の見本「星ひろい」を、同じファイルのまま端末でも遊べるようにする。6 節の 1 の「P6 の順」は、まず端末とし、その後の順（Node.js、WASI、デスクトップの GUI、モバイル）は、P6a の後で決める。

* **選んだ理由**：作ろうとしているのは ConJS で、ConJS のプログラムが、ConMa の動くどの所でも同じに動くことが目標である（1 節の B）。P5c の対話のプログラムは、今はブラウザでしか動かない。端末は、どの開発者の手元にもあり、テストも 3 つの VM で同じログにできる。M2 のレビュー（PR shima3/conma#12）でも、端末でキーを待つことは P6 で考える、とした。
* **VM を変えない**：端末のキーは、ConMa の今のプリミティブ（`__OS_get_stdin__`、`__OS_wait_readable__`、`__OS_read__`、`__OS_spawn__`）だけで読める。P5 の `fs` と `fetch` の curl の道と同じく、ConJS のライブラリ（conjs/include/conjs_lib.se）に書き、どの VM も、conma.js も変えない。

**足すもの**

* **端末の入力の道**（conjs/include/conjs_lib.se）：`nextEvent` と `pollEvent` は、ホストに機能 `"input"` があれば今のとおりそれを使い（ブラウザ）、なければ、標準入力から読む（端末）。
    * 入力の列と待ちの列：P5c の input.js と同じく、読んだキーを届いた順に列にため、待ちの列は待ち始めた順に入力を渡し、時間切れの待ちは列から外す。どちらも、ConMa の値（Ref と LList）で持つ。
    * 列を受け持つ VProc（M2 のレビューの 1 と、その後の確認）：ConJS のスケジューラ（include/lang/sched.se）は、待っている処理を、自分のチャネルに `Kit_sched_put` で届いた結果でだけ起こす。また、ConMa のスケジューラは、どの 2 つのステップの間でも VProc を切り替え（docs/SPEC.md）、Ref を読んで書き換えるのをまとめて行うプリミティブはない。いくつもの VProc が同じ列を読み書きすると、時間切れとキーが同じ待ちを終えて結果が 2 つ届いたり、足した待ちが古い列の書き戻しで消えたりする。そこで、input.js（1 つのスレッドなので `put` と時間切れが混ざらない）と同じ決め方を守るために、列は 1 つの VProc だけが持つ。
        * 入力を受け持つ VProc を 1 つ置く。入力の列、待ちの列、ESC の持ち越しは、その VProc だけが読み書きする。ほかの VProc は、ConMa のチャネル（書き手はいくつあってもよい）で、その VProc に知らせを送る。
        * `nextEvent`：（"wait"、処理、チャネルの書き手、ms、待ちの番号）を送って止まる（`Kit_sched_suspend` と、`"pending"` に 1 を足す所は、`Kit_sched_wait_work` と同じ）。受け持つ VProc は、列にキーがあればすぐに `Kit_sched_put` で渡し、なければ待ちの列に入れる。ms が 0 以上なら、`time.sleep` を待つ時間切れの VProc を始め、その VProc は、時間が来たら（"timeout"、待ちの番号）を送るだけにする。
        * `pollEvent`：（"poll"、チャネルの書き手）を送り、答え（キーか null）を受け取る。ConJS の処理から見れば、`Kit_sched_wait_work` の待ちと同じ。
        * 読み手：`__OS_wait_readable__` で待ち、読めた文字列をそのまま（"data"、文字列）で送る。標準入力が終わったら（"end"）を送る。読み手は、キーに分けない。
        * 受け持つ VProc は、届いた文字列をキーに分け（1 回の読みから、いくつものキーを取り出す。キーを押し続けると `ESC [ A ESC [ A ...` が並ぶ）、キーごとに、待ちの列の先頭があれば `Kit_sched_put` で渡し、なければ列にためる。待ちを終えたら、その時間切れの VProc を `__VProc_kill__` で止める。（"timeout"、番号）が来たとき、その待ちがもう終わっていれば、何もしない。こうすれば、1 つの待ちに結果が 2 つ届くことはない。
        * 受け持つ VProc と読み手は、`"pending"` に数えない（数えると、`nextEvent` を一度呼んだプログラムが終わらなくなる）。どちらも、初めて `nextEvent` か `pollEvent` を呼んだときに始める（ConJS の処理の外の VProc）。
    * キーの名前：ブラウザの `KeyboardEvent.key` に合わせる。ふつうの文字はその文字、`\r` と `\n` は `"Enter"`、`\t` は `"Tab"`、127 と 8 は `"Backspace"`、ESC `[` `A`〜`D`（と ESC `O` `A`〜`D`）は `"ArrowUp"`、`"ArrowDown"`、`"ArrowRight"`、`"ArrowLeft"`、ESC だけなら `"Escape"`。ほかの ESC の並びと、名前を決めていない制御文字（Ctrl-A など。`-icanon` では Ctrl-D も 1 文字として届く）は `"Unidentified"`（捨てずに渡す。プログラムは無視できる）。クリックは、端末では来ない（`type` は、いつも `"key"`）。
    * ESC の並びが分かれて届くとき（M2 のレビューの 3）：ssh ごしなどでは、`ESC [ A` が `ESC` と `[A` に分かれて届くことがある。読んだ分の終わりが、ESC か、終わっていない並び（`ESC [`、`ESC O`）なら、次の読みに持ち越す。40 ミリ秒、続きが来なければ、`"Escape"` か `"Unidentified"` にする（受け持つ VProc が、持ち越したときに時間切れの VProc を始め、その VProc が（"esc-timeout"、番号）を送る。その前に続きが届いていれば、何もしない）。標準入力が終わったら、すぐにそうする（テストのファイルでは、時計によらない）。
    * 標準入力の終わり：`pollEvent` は `undefined`。`nextEvent` は、列が空で、もう来ないので、Error（`nextEvent: the input has ended`）を投げる（ずっと待つと、プログラムが終わらない）。時間を限った `nextEvent` も、時間を待たずに同じ Error にする。
    * 端末のモード：標準入力が端末なら、行ごとでなく 1 文字ずつ読み、打った文字を出さないように、初めて読むときに `stty -icanon -echo`（`__OS_spawn__` で、標準入力を受け継ぐ）にし、前の設定（`stty -g` の出力）を覚えておく。端末でなければ（テストのファイルやパイプ）、`stty` は失敗するので、何もせずに読む。
    * 端末を戻し、VProc を止める（M2 のレビューの 2）：Rust 版の VM は、すべての VProc が終わるまで動き、OS を待つ VProc は行き詰まりとしても報告されない。そこで、ライブラリが始めた VProc（受け持つ VProc、読み手、時間切れの VProc、SIGINT を受け取るために `__OS_trap_interrupt__` の Ref を `__Ref_wait_change__` で待つ VProc）を覚えておき、3 つの終わり方のどれでも、端末を戻してから、それらを `__VProc_kill__` で止める。
        * ふつうに終わったとき：`Conjs_run` の終わり。
        * 受け手のない例外で終わるとき：sjs_rt.se の `Sjs_uncaught` が、PDict の `"sjs_cleanup"` にある関数を、出す前に呼ぶ。`"sjs_cleanup"` は main の VProc の PDict に置き、ConJS の処理ごとに入れ替える PDict の名前（`"sjs_handlers"`、`"sjs_gen"`、`"sjs_pos"`）には入れない（どの処理で例外が起きても読める）。SJS と ConSh は置かないので変わらない。
        * Ctrl-C：`-icanon` だけなので（`isig` は残す）、Ctrl-C は SIGINT のまま届く。`__OS_trap_interrupt__` で受け取り、戻して止めてから、終了ステータス 130 で終える。
        * 3 つの道を通らない終わり方（ConMa の `__print_exit_error__` の道や、VM の止まるエラー）では、main の VProc だけが終わって読み手が残り、VM は終わらず、端末も戻らない（`__OS_trap_interrupt__` の後なので、Ctrl-C でも止まらないことがある）。まれな場合なので防がない。conjs/README-ja.md に、端末が戻らなかったときは別の端末から VM を止め、`stty sane`（か `reset`）を打つ、と書く。
    * キーを待てるかを知る方法（M2 のレビューの 4）：`nextEvent` と `pollEvent` は、どこでも使える。端末でも、標準入力がファイルやパイプでも読み、入力がなければ（`/dev/null` など）、`nextEvent` は終わりの Error になる。`host.available("input")` は、今のとおりブラウザのホストの機能（input.js）を表し、端末では偽のままにする。ブラウザで input.js を渡さないページだけは、標準入力もないので、今のとおり TypeError（`nextEvent: this platform has no input`）。
* **見本**（vm/wasm/web/stars.con.js）：機能 `"canvas"` があれば今のとおり canvas に描き、なければ端末に描く（今の「`"canvas"` か `"input"` がなければ終える」判定は、なくす）。`nextEvent` の終わりの Error（標準入力が `/dev/null` やパイプで、尽きたとき）は、見本が受け止め、「入力が終わった」と出して、ふつうに終える（端末も戻る）。端末では、ANSI のエスケープの並び（画面を消して左上に戻る `\u001b[2J\u001b[H`、色）を `console.log` で出し、自分を `@`、星を `*`、おばけを `G` で描く。描く処理は、`host.wait("canvas.frame")` の代わりに `sleep(100)` で待ち、変わったときだけ描く。クリックは来ないので、タイトルと結果の画面の説明を、端末では「キーで始める」にする。ゲームの決まり（画面の順、おばけごとの `spawn`、`nextEvent(100)`）は、2 つの描き方で同じにする。
* **道具**：bin/lang-test は、今は標準入力を `/dev/null` にして動かす。ステップのディレクトリに `case*.in` があれば、それを標準入力にする（ConJS と SJS のほかのテストは変わらない）。

**テスト**（conjs/test の step15。Rust 版、wasm 版（Node.js）、Python 版の VM で同じログ）

* case1：`case1.in`（ふつうの文字、Enter、Tab、Backspace、矢印のキーの ESC の並び、続けて並んだ矢印のキー、Ctrl-A、ESC だけ、のバイトの列）を標準入力にして、`nextEvent` と `pollEvent` が返すキーの名前、1 回の読みからいくつものキーを取り出すこと、列にためること、時間を限った `nextEvent` で列に入力があればすぐ返すこと、標準入力の終わりの Error を確かめる。最後の ESC は、ファイルの終わりで続きが来ないので、時計を待たずに `"Escape"` になる。本当の時計を使う時間切れは、ログに入れない。
    * ESC の並びが分かれて届く場合は、ファイルでは作れないので、テストのための小さな ConMa の関数（キーに分ける所だけ。読んだ文字列の並びを渡して、取り出したキーを返す）を、case3 で直に呼んで確かめる。
* case2：見本を端末の道で動かし、`case2.in` の Escape で、タイトルから終わることを確かめる（遊ぶ所は、おばけが本当の時計で動き、描く回数が揺れるので、ログで比べない）。
* step14 のログが変わる：Rust 版と Python 版の VM の case1 と case2 は、今は「入力がない」を確かめているが、端末の道ができるので、標準入力が `/dev/null`（すぐに終わり）のときの振る舞いに変わる。case1 は `nextEvent` の終わりの Error、case2 は、見本がそれを受け止めて終える（受け手のない例外にならない）ログになる。wasm 版の VM のログ（case*.wasm.log。ホストの input.js を使う）は変わらない。
* 端末で遊ぶこと：Rust 版の VM で、本当の端末で見本を動かし、キーで動いておばけにぶつかり、Escape で終わったとき、Ctrl-C で止めたとき、例外で止めたときに、端末が元に戻る（打った文字が見え、行ごとに読む）ことを確かめる。これはテストにできないので、M1 と M2 が手で確かめ、8 節に書く。
* conjs/test の今の場合、sjs/test、consh/test が通り、SJS が作るコードが変わらないこと（bin/lang-emit-check）。

**終わりとすること**（5 節の表）：step15 が 3 つの VM で同じログで通る。見本が、Rust 版の VM の端末で遊べ、どの終わり方でも端末が元に戻る。SJS が作るコードが変わらない。

**P6a でしないこと**（後の候補）：Windows の端末（`stty` がない。コンソールの API が要る）、マウス、端末の大きさ（`stty size`）と、大きさが変わったとき、色を使えない端末、キーを離したこと。ブラウザと端末の描き方を 1 つの API にすること（見本は、2 つの描き方を自分で持つ）。

**M2 のレビュー**（PR shima3/conma#15）：P6 を端末から始めること、VM を変えずに ConJS のライブラリで標準入力を読む作り、キーの名前をブラウザに合わせること、標準入力の終わりを Error にすること、端末を戻す 3 つの道、step14 のログが変わることに、賛成をもらった。指摘を受けて、列を 1 つの VProc だけが持ち、ほかの VProc はチャネルで知らせを送る形（M2 の確認で、いくつもの VProc が同じ列を読み書きすると混ざることが分かったため。待ちの列、`Kit_sched_put`、時間切れの VProc は、その VProc が扱う）と、受け持つ VProc と読み手を `"pending"` に数えないこと、終わりに止める VProc と、3 つの道を通らない終わり方（README の `stty sane`）、ESC の並びの持ち越しと 40 ミリ秒、1 回の読みからいくつものキーを取り出すこと、キーを待てるかを知る方法と見本の終わり方、名前のない制御文字、`"sjs_cleanup"` を置く PDict を書き足した。

### 4.8. 段階 P8 の設計（決めた、2026-11-21。M2 のレビューは PR shima3/conma#20）

P8 では、ConJS のファイルの入出力を広げ、ファイルを 1 行ずつ読む見本を書く（5 節の表の P8 の行。ユーザーが 2026-11-21 に頼み、M2 が PR shima3/conma#17 で足した）。P5c と同じく、「同期処理なら簡単だが、JavaScript では非同期処理が要るので複雑になるプログラム」の見本にする。P6（プラットフォームを増やす）の続きと P7（ホストの関数を足す文書と口）より先にするのは、作ろうとしているのが ConJS で、P6a で作った標準入力を読む部品も使えるためである（P8 は「後でよい」として足した行なので、M2 がユーザーに確かめ、「P8 を先に進めて良い」との答えをもらった。2026-11-21）。

* **今あるもの**：ファイル全体を文字列で読み書きする `fs.readFile(path)` と `fs.writeFile(path, text)` だけ（P5、4.4 節）。大きなファイルは、全部を 1 つの文字列にしないと読めない。
* **JavaScript（Node.js）で 1 行ずつ読むと**：`fs.createReadStream` と `readline.createInterface` を作り、`for await (const line of rl)` で回し、`try`／`finally` で閉じる。関数は `async` になり、呼ぶ側にも `await` が広がる（関数の色）。2 つのファイルを並べて読む（下の見本）には、`for await` は使えず、`rl[Symbol.asyncIterator]()` の `next()` を、それぞれ `await` で呼ぶことになる。ConJS では、結果が届くまで呼んだ処理だけが待つので、ふつうのループで書ける。

**足すもの**（ConJS だけ。ホストのものなので、SJS には移さない。4.1 節の「ConJS を広げる道」）

* **`fs.open(path, mode)`**：ファイルを開き、ファイルのオブジェクトを返す（Node.js の `fs.promises.open` の FileHandle に似せる。Node.js の `fs.open` はファイル記述子をコールバックに渡す別のものだが、ConJS には要らないので、短い名前を使う）。mode は `"r"`（読む。既定）、`"w"`（書く。なければ作り、あれば空にする）、`"a"`（後ろに足す）。開けなければ Error（今の `fs.readFile` と同じメッセージの形）。ConMa の `__OS_open__` で開く（P5 の `fs` と同じ。VM は変えない）。
* **ファイルのオブジェクト**（`console.log` では `File { path: 'a.txt' }`。ほかのプロパティは見せない）：
    * `f.readLine()`：次の 1 行を、行の終わり（`\n` か `\r\n`）を除いて返す。ファイルの終わりなら `undefined`。最後の行が `\n` で終わっていなくても、その行を返す。空の行は `""`。
    * `f.readLines()`：残りの行を 1 行ずつ返すジェネレータ（`for (const line of f.readLines())`）。名前は Node.js の FileHandle の `readLines()`（Node.js 18.11 から）に合わせる。Node.js のものは `for await` で回すが、ConJS では `for` で回す（README の JavaScript との違いに書く）。ランタイムの `Sjs_generator` で作り、本体の `finally` で、ジェネレータが終わったときと、`for` を `break`、例外、関数の `return` で抜けたときに、ファイルを閉じる（M2 が、今の ConJS のジェネレータで、この 4 つの抜け方で `finally` が動き、Node.js と同じ出力になることを確かめた）。
    * `f.read()`：残りを全部、1 つの文字列で返す（終わりなら `""`）。
    * `f.write(text)`：文字列を書く（`"w"` と `"a"` のとき。ほかは Error）。
    * `f.close()`：閉じる。何度呼んでもよい。閉じた後の `readLine`、`read`、`write` は Error（`File a.txt is closed` の形）。
* **`fs.readLines(path)`**：`fs.open(path).readLines()` と同じ（Node.js にはない近道）。いちばんよく使う形：`for (const line of fs.readLines("log.txt")) { ... }`。
* **標準入力と標準出力**：`fs.stdin`（読むだけのファイルのオブジェクト。`fs.stdin.readLine()`、`fs.stdin.readLines()`）と `fs.stdout`（`write` だけ。`console.log` と違い、改行を足さない）。Node.js では `process.stdin` と `process.stdout` だが、ConJS には `process` がないので `fs` に置く（README の JavaScript との違いに書く）。標準入力を、`nextEvent` のキー（P6a）と行の両方で読むことはできない。先に使ったほうに決まり、もう一方は Error（`standard input is read as keys` か `standard input is read as lines`）にする。端末のモードは変えない（行ごとに読む、ふつうのモード）。
* **読み方**（conjs/include/conjs_lib.se に足す、新しい conjs/include/conjs_file.se）：
    * ファイルのオブジェクトは、隠したキーに、Stream、読んだ文字列のバッファとその中の位置、終わりに着いたか、閉じたかを持つ。`readLine` は、バッファの位置から `\n` を探し（`__String_find__`。ASCII でない文字列の位置は、PR shima3/conma#18 で、前に求めた位置からたどるので速い）、なければ次を読み足す。バッファを読み足すときは、使い終わった前の部分を捨てる（1 行ごとに文字列を作り直さない。行の数に比例する時間で読む）。
    * 読むのは `__OS_read__`。ふつうのファイルは、すぐに読める（処理は切り替わらない）。パイプ、FIFO、標準入力のように、すぐには読めないかもしれないものは、`__OS_wait_readable__` を、P5 の curl と同じ `Kit_sched_wait_work` の VProc で待つ（待つ間、ほかの処理は動く）。すぐ読めるかは、先に `__OS_read__` を試し、`""` が返ったときだけ待つ、で決める。`""` は「パイプに、まだ何も届いていない」のほかに、「届いたのが ASCII でない文字の途中までのバイトだけ」のときにも返る（下の持ち越しのため。M2 のレビューの 2）。どちらも、待てば続きのバイトが届いてから起きるので、同じ扱いでよい。
    * `__OS_read__` の区切りは、ASCII でない文字の途中に来ない：どの VM も 1 回に 4,096 バイトまで読み、文字の途中で切れたバイトを Stream に持ち越して、次の読みでつなぐ（Rust 版と wasm 版の VM は vm/src/value.rs の `decode`、Python 版の VM は `codecs` の incremental decoder。M2 がコードを読んで確かめた）。
* **見本**（conjs/examples/merge.con.js。新しいディレクトリ）：並べた（ソートした）2 つのファイルを 1 行ずつ読み、片方だけにある行と、両方にある行を出す（Unix の `comm` と同じ）。どちらのファイルも全部は読まず、2 つの `readLine` を、比べた結果で交互に進める。`-` は標準入力（`sort a.txt | conjs/conjs conjs/examples/merge.con.js - b.txt`）。大きなファイルでも、使うメモリは 1 行の長さほどで済む。
    * README（conjs/README-ja.md）に、同じことを Node.js（`readline` と、2 つの非同期のイテレータの `next()` を `await` で呼ぶ形）で書いたものと並べて載せる。Node.js のほうは、git に入れる別のファイル（conjs/examples/merge.node.js）にし、同じ出力になることを conjs/test/compare.sh で確かめる（下のテスト）。
    * もう 1 つ、ログの集計（conjs/examples/wordcount.con.js。`fs.readLines` で 1 行ずつ読み、語の数の上位を出す）を、いちばん短い使い方の例として置く。

**テスト**（conjs/test の step16。Rust 版、wasm 版（Node.js）、Python 版の VM で同じログ）

* case1（読む）：`\n` と `\r\n` の行、空の行、最後の行に `\n` がないファイル、空のファイル、`read()` で残りを読むこと、`readLine` の後の `readLines()`。ASCII でない文字の行。テストのファイルは、ステップのディレクトリに置く（一時ディレクトリに写す copy.txt を使う）。
    * 読みの区切り（4,096 バイト）をまたぐ場合（M2 のレビューの 1。バッファを読み足す所は、4,096 バイトを超えるファイルでしか通らない）：1 つの行が区切りをまたぐファイル、`\r\n` の `\r` と `\n` が区切りの両側に分かれるファイル、ASCII でない文字が区切りをまたぐファイル、4,096 バイトより長い 1 行。ファイルは、テストの中で `fs.writeFile` で作る。
* case2（書く）：`"w"` で書いて閉じて読み直す、`"a"` で足す、閉じた後の Error、開けないファイルの Error、読むだけのファイルへの `write` の Error。
* case3（`readLines()` と閉じること）：`for` を回し切ったとき、`break` で抜けたとき、本体で例外を投げたとき、ループのある関数を `return` で抜けたときに、ファイルが閉じられること（閉じたかは、ファイルのオブジェクトの `closed` で見る。JavaScript の FileHandle にはないが、テストと使う人のために置く）。
* case4（標準入力）：`case4.in` を `fs.stdin.readLines()` で読む。キーと行の両方で読もうとしたときの Error。
* case5（見本）：conjs/examples/merge.con.js を、ステップに置いた 2 つのファイルで動かす（3 つの VM で同じログ）。
* Node.js 版との比べ方（M2 のレビューの 4）：conjs/test/compare.sh に、ConJS のプログラムと Node.js のプログラムの組を並べるファイル（conjs/test/node-pairs.txt。1 行に 2 つのファイルと、渡す引数）を足し、compare.sh が、merge.con.js と merge.node.js を同じ引数で動かして、出力を比べる。compare.sh は今も `node` があることを前提にしているので（node.txt の場合）、Node.js のない機械では、compare.sh ごと流さない（conjs/test/run_all.sh と step16 は `node` を使わない）。
* 大きなファイル（M2 のレビューの 1）：10 万行と 100 万行のファイルを `fs.readLines` で数える時間は、step16 に入れず、手で測って 8 節に書く（Python 版の VM では、100 万行だけで何分もかかるおそれがある）。比べる相手として、同じファイルを `fs.readFile` と `split("\n")` で数える時間も測る。行の数に比例することを確かめる。
* conjs/test の今の場合、sjs/test、consh/test が通り、SJS が作るコードが変わらないこと（bin/lang-emit-check）。

**終わりとすること**（5 節の表）：step16 が 3 つの VM で同じログで通る。見本が 3 つの VM で動き、Node.js で同じことを書いたものと同じ出力になる（compare.sh。README に並べて載せる）。大きなファイルを読む時間が、行の数に比例する（手で測る）。SJS が作るコードが変わらない。

**P8 でしないこと**（後の候補）：バイトのデータ（`Buffer`）、UTF-8 でない文字コード、`seek` と位置、ディレクトリ（今は `__OS_listdir__` がある）、ファイルの情報（大きさ、時刻）、消すことと名前を変えること、`fs.watch`、子のプロセスの入出力。`for ... of` を、ジェネレータでない反復のプロトコル（`Symbol.iterator`）で回すこと（P5b で足さないものにした。ファイルのオブジェクトそのものは回せず、`readLines()` を呼ぶ）。

**M2 のレビュー**（PR shima3/conma#20）：向き、`readLine()`、`read()`、`write()`、`close()`、mode の名前、`closed` を置くこと、ジェネレータで作り `finally` で閉じること、すぐに読めないものだけを待つこと、標準入力をキーと行の両方で読めないこと、見本の選び方に、賛成をもらった。M2 が、ジェネレータの `finally` が 4 つの抜け方で動くことと、`__OS_read__` が文字の途中で区切らないことを確かめた。指摘を受けて、読みの区切りをまたぐ場合を case1 に足し、大きなファイルの時間を手で測ることにし、`""` のもう 1 つの意味を書き、`lines()` を Node.js に合わせて `readLines()` に（`fs.lines` も `fs.readLines` に）し、`fs.stdin` と `fs.stdout` を README の違いに書くことにし、Node.js 版との比べ方を compare.sh に決めた。

### 4.9. 段階 P7 の設計（決めた、2026-11-21。M2 のレビューは PR shima3/conma#22）

P7 では、使う人がホストの関数を足すための文書をまとめ、Rust 版の VM（と transpiled VM）に、組み込む人がホストの関数と機能の名前を足す口（Rust の API）を作る（5 節の表の P7 の行）。

* **今、先にする理由**（ユーザーの判断、2026-11-21）：P7 は「P5 の続きと P6 の後でよい」として足した行だが、P9、P10 と進めて大きくなってからでは、変える量が大きくなる。特に、P6 の続き（Node.js、WASI、デスクトップの GUI）は、どれもホストの関数を足す段階で、口がないまま進むと、そのたびに VM の決まった表（vm/src/prim/host.rs）を書き換え、後で全部を API に移すことになる。ConJS の `fetch` が、Rust 版と Python 版の VM で curl を動かしているのも、口がないための回り道である。値の形（下の「値」）も、ホストの関数が増える前に決めるほうが、変える所が少ない。M1 が書いた P9（ConJS のチャネル）の案は、ホストの関数を足さないので P7 の量を増やさないが、P7 の後に回す（push していないブランチ m1/platform-p9-design に置いてある）。
* **今あるもの**：
    * ConMa：`__Host_call__`（すぐ返る）、`__Host_start__` と `__Host_result__`（待つ。結果は JSON の文字列でパイプに書く。3.4 節）、`__Feature_available__`。ConJS：`host.call`、`host.wait`、`host.available`。
    * wasm 版の VM：ホスト（vm/wasm/conma.js）の `hostFunctions`（名前から関数。Promise を返せば待つ呼び出し）と `hostFeatures`。Node.js では `CONMA_HOST` の CommonJS のモジュール（vm/wasm/vm-node.js）。ブラウザのものは vm/wasm/host/canvas.js と input.js。
    * Rust 版と transpiled VM：決まった表だけ（`test.add`、`test.echo`、`test.fail`、`time.now`、と `__Host_start__` の中で特別に扱う `test.later` と `time.sleep`）。VM は bin のクレート（vm/src/main.rs）で、ほかの Rust のプログラムからは使えない。
    * Python 版の VM：`test.*` と `time.*` の表（vm/python/vproc_step.py）。
    * 文書：conjs/README-ja.md、vm/wasm/conma.js の先頭のコメント、vm/wasm/host/canvas.js、vm/wasm/vm-node.js、docs/PRIMITIVES.md に分かれている。

**1. 文書**（新しい docs/HOST-ja.md。使う人のための 1 か所）

* ホストの関数とは何か、すぐ返る関数と待つ関数、機能の名前、渡せる値、エラー（ホストのメッセージが onError や ConJS の Error になる）。
* 呼ぶ側：ConJS の `host.call`、`host.wait`、`host.available` と、ConMa の `__Host_call__`、`Host_wait`（include/platform/host.se）、`__Feature_available__`。
* 足す側を、プラットフォームごとに：ブラウザ（`ConMaVM.create` の `hostFunctions` と `hostFeatures`）、Node.js（`CONMA_HOST`）、Rust 版の VM（下の 2）。それぞれに、同じ小さな例を置く：すぐ返る `demo.add`（数を足す）と、待つ `demo.delay`（ミリ秒の後に文字列を返す）と、それを呼ぶ 1 つの ConJS のプログラム（docs/examples ではなく、テストに使う conjs/examples/host_demo.con.js）。
* 今の 5 か所は、詳しいことはそのまま残し（conma.js のコメントは、そのファイルを読む人のため）、docs/HOST-ja.md への参照を足す。ConJS の README の `host.call` の項は、短くして参照にする。

**2. Rust の API**

* **クレートを lib と bin に分ける**：vm/src に lib.rs を足し、今の main.rs の `mod` の並びを lib.rs に移す。main.rs は、lib の `conma::run_main(args)`（今の main の中身）を呼ぶだけにする。wasm 版のクレート（vm/wasm/src/lib.rs が vm/src のファイルを `#[path]` で使う）と transpiled VM（`CONMA_TRANSPILED` で、build.rs が作ったコードを入れる）は、今の作り方のまま動くようにする。
* **登録**：組み込む人は、自分の main で表を作り、VM を動かす：

```rust
let mut host = conma::Host::new();          // test.* と time.* は、はじめから入っている
host.function("demo.add", |args| { ... Ok(HostValue::Num(sum)) });
host.waiting("demo.delay", |args, done| {   // done は、後で 1 回だけ呼ぶ（別のスレッドからでもよい）
    let ms = ...;
    std::thread::spawn(move || { sleep(ms); done.finish(Ok(HostValue::Str("late".into()))) });
});
host.feature("demo");
std::process::exit(conma::run_main_with(std::env::args().collect(), host));
```

* **すぐ返る関数**（`function`）：`Fn(&[HostValue]) -> Result<HostValue, String>`。`__Host_call__` からも `__Host_start__` からも呼べる（後者では、すぐ結果を書く。今の BUILTIN と同じ）。
* **待つ関数**（`waiting`）：`Fn(&[HostValue], Done)`。`__Host_start__` からだけ呼べる（`__Host_call__` では「待つ関数は `__Host_call__` で呼べない」のエラー。wasm 版で Promise を返したときと同じ扱い）。`Done` は `Send` で、`finish(result)` は、結果をスケジューラの列（`Mutex` の中）に入れ、起こすためのパイプに 1 バイト書くだけにする。スケジューラは、ステップの間でその列を見て、その呼び出しを終える（下の「結果の置き場所」。今の `Timers` が、時刻の来た `time.sleep` を、ステップの間で終えるのと同じ。別のスレッドが直接パイプに書くと、書いた所と閉じた所の間を VProc が見て、`__Host_result__` が「終わっていない」になるため）。どの VProc も動けないときは、起こすためのパイプも `poll` で待つ。`finish` を呼ばずに `Done` を捨てたら、エラーの結果（`demo.delay: the host dropped the call`）にする（`Drop` で）。
* **結果の置き場所**（M2 のレビューの 1）：今は、結果の JSON の文字列を、その呼び出しのパイプに書いてから閉じる。スケジューラや呼んだステップの中で書く結果（BUILTIN と `test.later`）は、読み手がまだいないので、OS のパイプに入り切る 8,192 バイト（`PIPE_MAX`）を超えるとエラーにしている（`capped`）。同じ形で待つ関数の結果を書くと、組み込む人の関数の結果（ファイルの中身、HTTP の本体）も 8 KB までになり、限りのない wasm 版（メモリの上のパイプ）と、渡せる大きさが違ってしまう。そこで、Rust 版と transpiled VM では、結果の文字列を `Machine` の表（呼び出しのパイプの読む側の fd から結果へ）に置き、パイプには何も書かずに閉じるだけにする（パイプは、終わったことを知らせて `__OS_wait_readable__` を起こすためだけに使う）。`__Host_result__` は、パイプの終わりを見てから、表の文字列を取り出す。すぐ返る関数を `__Host_start__` から呼んだときも、`test.later` も、同じ道にし、`capped` と `PIPE_MAX` はなくす。呼んだ VProc が殺されて Stream が閉じられたら（`__Host_start_tracked__`）、Stream を閉じる所で、その fd の結果を表から除く（残さない）。wasm 版の VM（パイプがメモリの上で、限りがない）は、今のまま。Python 版の VM も今のまま（関数はテストのものだけで、結果は小さいので、8 KB の限り `HOST_PIPE_MAX` を残す。テストの結果が 8 KB を超えないかぎり、トレースは Rust 版と同じ）。
* **VM が終わった後の `finish`**（M2 のレビューの 2）：組み込む人のスレッドが、VM が終わった後（`run_main_with` が返った後）に `finish` を呼ぶことがある。起こすためのパイプを VM が閉じていると、その fd の番号が、組み込む人の開いた別のファイルに使い回され、そこに 1 バイト書いてしまうおそれがある。そこで、結果の列と起こすためのパイプは、`Done` と共有する `Arc` が持ち、VM と最後の `Done` の両方がなくなったときに閉じる。VM が終わった後の `finish` は、結果を捨てるだけにする。
* **表を Machine に持たせる**：今の `BUILTIN` と `NATIVE` の決まった表と、`feature()` の決まった名前を、`Machine` が持つ `Host` の表に替える。`test.*` と `time.now` は、`Host::new()` が登録する（同じ API を使う。`time.sleep` も、`Timers` を使う待つ関数として登録できるかを作るときに確かめ、できなければ今のまま特別に扱う）。`test.later`（仮の時計）は、スケジューラと結び付いているので、今のまま特別に扱う。
* **transpiled VM**：`__Host_call__` と `__Host_start__` は、変換したコードから VM に任せる所（7 節）なので、表が `Machine` にあれば、そのまま使える。組み込む人が transpiled VM を使うときは、自分のクレートで vm を依存にして、`CONMA_TRANSPILED` を付けて作る（vm/conma2rust/build.sh と同じ）。docs/HOST-ja.md に書く。
* **wasm 版の VM**：今のまま（ホストの関数は JavaScript の側。7 節の「wasm 版の大きさ」）。Rust の API は、wasm 版のクレートでは使わない。
* **Python 版の VM**：今のまま（5 節の表の P7 の行のとおり。テストと `time.*` だけ）。

**3. 値**（M2 に聞きたいことの 2）

* 今、ホストと受け渡せるのは、数、文字列、それらの LList だけで（null は空の LList）、真偽と Dict は渡せない。ConJS では、ホストが返した空の配列も `undefined` になる。使う人が関数を足し始める前に決めたい。
* M1 の案：真偽（`true`、`false`）と Dict（文字列の鍵。JavaScript のオブジェクト、Rust の `HostValue::Dict(Vec<(String, HostValue)>)`）を足す。wasm 版では、今の JSON の形のまま、真偽は JSON の真偽、Dict は JSON のオブジェクトにする（`{"n":...}` の特別な形は、数だけに使っているので、鍵が 1 つで名前が `n` の Dict と区別するため、Dict は `{"d":{...}}` で包む）。ConJS では、真偽はそのまま、Dict は SJS のオブジェクトにする。Python 版の VM の表（`test.echo`）も、同じに受け渡せるようにする（トレースを比べるため）。
* null と空の配列の区別は、ConMa の値の形（null が空の LList）によるので、P7 でも変えない。docs/HOST-ja.md に書く。
* ConJS からホストに渡すとき（M2 のレビューの 4）：ConJS のオブジェクトは、隠したキー（クラスの記録の `"cls"`、P8 のファイルの `"fs_stream"` など）も持つ Dict なので、`Conjs_to_host` は、自分のプロパティだけ（`Object.keys` と同じ）を Dict にして渡す。値が関数のものは、今と同じく TypeError にする。たとえば、P8 のファイルのオブジェクトを渡すと、`{ path: "a.txt" }` だけが渡る。

**4. Node.js で決まらない Promise**（7 節）

* vm-node.js で、VM が IDLE（どの VProc も外を待つ）で、待っているホストの呼び出しがあるのに、Node.js のイベントループに、ほかにすることがなくなったら（`process.on("beforeExit")`）、`vm-node: every VProc waits for host calls that never finish: N`（N は待っているホストの呼び出しの数）を標準エラーに出し、終了ステータス 2 で終える。今の `vm-node: every VProc waits, and nothing can wake them`（終了ステータス 2）にそろえ、2 つの違いが分かるようにする（M2 のレビューの 3）。M2 が確かめたとおり、今は、`jsPending` がその呼び出しを数え続けるので、その行にも来ず、`main()` の Promise が決まらないまま Node.js が終わり、何も出さずに終了ステータス 0 になる。
* Rust 版の VM でも、待つ関数が `Done` を持ったまま何もしなければ、同じことが起きる（スケジューラが、起こすためのパイプを待ち続ける）。こちらは、`Done` を持つものがいなくなれば `Drop` でエラーになるので、持ち続ける場合だけである。見つける方法がないので、docs/HOST-ja.md に書くだけにする。

**テスト**

* **Rust の API**：vm/examples/host_demo.rs（`cargo run --example host_demo -- ...`。上の例の `demo.add`、`demo.delay`、機能 `"demo"` を登録して、今の main と同じに VM を動かす）を作り、conjs/examples/host_demo.con.js を動かして、決めたログと比べる。テストの台本は test/host-api.sh（新しい）。台本は、`cargo build --release --example host_demo` を自分で呼ぶ（今の Full Test の手順では example が作られないため。M2 のレビューの 5。作るときの申し送りにも書く）。待つ関数は別のスレッドから `finish` するので、スレッドが結果を返す順を、決まるように作る（`demo.delay` の時間を十分に離す。ログには順だけを出す）。ほかに、`finish` を呼ばずに捨てたときのエラー、待つ関数を `__Host_call__` で呼んだときのエラー、登録していない名前のエラー、`host.available("demo")` が真で、登録しない main では偽になること。
* **wasm 版**：同じ host_demo.con.js を、同じ関数を JavaScript で書いた conjs/examples/host_demo.host.js（`CONMA_HOST`）で動かし、同じログになること（docs/HOST-ja.md の例と同じファイル）。
* **値**：真偽と Dict を、`test.echo` で 3 つの VM に通す（test/ のホストの場合 fast14、fast15 に足すか、新しい場合。ConJS では conjs/test の新しい step）。
* **決まらない Promise**：Promise を返して決めないホストの関数で、vm-node.js が Deadlock を出して 2 で終わること（conjs/test の新しい step の wasm 版のログ）。
* test/、consh/test、sjs/test、conjs/test が 3 つの VM と transpiled VM で通る（クレートを分けたため）。wasm 版の VM と transpiled VM を作り直す。sjs/bench で遅くならないこと（表を `Machine` に持たせて、名前で引く所が変わるため）。

**終わりとすること**（5 節の表）：docs/HOST-ja.md の例（ホストの関数と、それを呼ぶ ConJS のプログラム）が、wasm 版の VM で動く。Rust の API で足した関数を、同じ ConJS のプログラムから呼べ、`host.available` が真になる（test/host-api.sh）。値に真偽と Dict を足すなら、3 つの VM で受け渡せる。今のテストがすべて通る。

**P7 でしないこと**（後の候補）：Python 版の VM に使う人が関数を足す口（Python のテストの VM なので要らない）。ホストの関数から ConMa の関数を呼び返すこと（コールバック）。ホストの値をつかんだまま渡すこと（JavaScript のオブジェクトを `__JS_value__` のように持つこと）。ホストの関数を、Rust のクレートを作らずに、動的なライブラリ（`.so`、`.dylib`）から読むこと。ConJS の `fetch` を、Rust 版の VM でホストの関数に替えること（curl をやめる。P7 の口を使って、後の段階で）。

**M2 のレビュー**（PR shima3/conma#22）：向き、値に真偽と Dict を足すこと（`{"d":{...}}` の形）、`Done` の作り、決まらない Promise を `beforeExit` で見つけること、テストの置き場所に、賛成をもらった。指摘を受けて、結果を `Machine` の表に置いて 8 KB の限りをなくすこと（1）、VM が終わった後の `finish`（2）、vm-node.js の出す行（3）、ConJS から渡すのは自分のプロパティだけにすること（4）、test/host-api.sh が example を作ること（5）を書き足した。

**M2 に聞いたこと**

1. **向き**：lib と bin に分け、`Host` の表を `Machine` に持たせ、組み込む人が自分の main を書く形でよいか。今の `conma` のコマンドに、組み込みでない方法（設定のファイルなど）で関数を足す道は作らない。
2. **値**：真偽と Dict を P7 で足すか。足すなら、wasm 版の JSON の `{"d":{...}}` の形でよいか。
3. **待つ関数の `Done`**：別のスレッドから呼べるようにし、スケジューラがステップの間で書く形でよいか。`Drop` でエラーにすることも。
4. **決まらない Promise**：vm-node.js の `beforeExit` で Deadlock にすることでよいか（ブラウザでは、ページは終わらないので見つけない）。
5. **テストの置き場所**：Rust の API のテストを、cargo の example と test/host-api.sh にすることでよいか（今の bin/vm と bin/lang-test は、決まった VM の実行ファイルを使うため）。

## 5. 段階

| 段階 | すること | 終わりとすること |
|---|---|---|
| P0 | この計画書。6 節を決める | 決めたことを、この文書に書く |
| P1（作った、2026-11-19。8 節） | プラットフォームの機能の層：`(Platform "機能")` の付け方、`__Feature_available__`。`__JS_call__` と `__JS_value__` を機能 `"js"` にする | test/ と consh/test がすべての VM で通る。機能の有無で振る舞いを変えるテスト |
| P2（作った、2026-11-19。8.2 節） | `__Host_call__`：wasm 版（JavaScript）、Rust 版と transpiled VM（Rust の表）、Python 版（Python の表）。テストのためのホストの関数 | 同じテストのトレースが Python 版と Rust 版で同じ。wasm 版の Node.js で同じ出力 |
| P3（作った、2026-11-19。8.3 節） | 待つホストの呼び出しの形を決めて作る（6 節の 3）。最初のプラットフォームの機能（ブラウザの canvas。描く見本のページ） | 見本が動く。1 回の呼び出しの時間を、今の `js` コマンドと比べて測る。足した機能の名前を docs/PRIMITIVES.md の `__Feature_available__` の一覧に書き、テストで、見本が使う名前が一覧にあることを確かめる（知らない名前は黙って偽になり、書き間違いと区別できないため） |
| P3b（作った、2026-11-19。8.4 節） | ConJS（4.1 節）：SJS の部品で作る。P3 の待つ呼び出しを使う | ConJS のテストが、すべての VM で通る。協調的に切り替わること（待つ呼び出しの所だけで切り替わる）を確かめるテスト |
| P4（作った、2026-11-20。8.5 節） | 言語キット：SJS から部品を取り出し、SJS と ConJS をキットの上に書き直す | sjs/test、consh/test、sjs/bench（遅くならないこと） |
| P5（作った、2026-11-20。8.6 節） | ConJS をふだん使えるものにする：ランタイムを先にモジュールにしておく。待つ呼び出しのライブラリ（タイマー、時刻、ファイル、fetch） | conjs/test が 3 つの VM で通る。`console.log("hi")` を動かす時間が 0.2 秒以下。sjs/bench の計算を ConJS で動かして遅くならない |
| P5b（作った、2026-11-20。8.7 節） | ConJS が受け付ける JavaScript を広げる：`class`、`new`、`this`、`instanceof`、エラーのコンストラクタ。`JSON.parse` と配列の `sort`（SJS と共有） | conjs/test が 3 つの VM で通り、足したものの場合が Node.js と同じ出力。SJS が作るコードが変わらず、sjs/bench が遅くならない |
| P5c（作った、2026-11-21。8.8 節） | ConJS で、ブラウザのキーとマウスの入力を待つ：ホストの関数 `input.next`、`input.poll`（機能 `"input"`）と、ConJS の `nextEvent`、`pollEvent`。見本の対話のゲーム（stars.con.js） | conjs/test の step14 が 3 つの VM で通る（VM ごとのログ）。見本がブラウザで遊べる。SJS が作るコードが変わらない |
| P6a（作った、2026-11-21。8.9 節） | P6 の最初の段階：端末（macOS と Linux）。ConJS の `nextEvent` と `pollEvent` を、標準入力のキーでも使えるようにする（ConJS のライブラリで書き、VM は変えない）。見本「星ひろい」を端末でも遊べるようにする | conjs/test の step15 が 3 つの VM で同じログで通る。見本が Rust 版の VM の端末で遊べ、どの終わり方でも端末が元に戻る。SJS が作るコードが変わらない |
| P6 | プラットフォームを増やす（6 節の順に） | そのプラットフォームでテストが通る |
| P7（作った、2026-11-22。8.11 節。4.9 節。忘れないための行として足したが、大きくなる前にするよう、ユーザーが 2026-11-21 に先にした） | 使う人がホストの関数を足すための文書：ブラウザと Node.js（`CONMA_HOST`）で `hostFunctions` と `hostFeatures` を渡し、ConJS から `host.call`、`host.wait`、`host.available` で呼ぶ方法を、1 か所にまとめる。今は conjs/README-ja.md、vm/wasm/conma.js の先頭のコメント、vm/wasm/host/canvas.js、vm/wasm/vm-node.js、docs/PRIMITIVES.md に分かれている。また、Rust 版の VM（と transpiled VM）に、組み込む人がホストの関数と機能の名前を足す口（Rust の API）を作る（3.2 節の「組み込む人が足すもの」。今の vm/src/prim/host.rs は決まった表で、足すには VM のソースを変える）。Python 版の VM は、今のまま（テストと `time.*` だけ） | 文書の例（小さなホストの関数と、それを呼ぶ ConJS のプログラム）が、wasm 版の VM で動く。Rust の API で足した関数を、同じ ConJS のプログラムから呼べ、`host.available` が真になる（Rust 版の VM のテスト） |
| P8（作った、2026-11-21。8.10 節。4.8 節。M2 が忘れないための行として足し、ユーザーが 2026-11-21 に頼んだ） | ConJS のファイルの入出力を広げ、見本を書く：ファイルを 1 行ずつ読むこと（Python の `for line in open(f)`）と、開いたファイルを持つ API。JavaScript（Node.js）では、ストリームと readline、`for await`、`try`／`finally` で閉じることが要り、非同期で複雑になるが、ConJS では、結果が届くまで呼んだ処理だけが待つので、ふつうのループで書ける。P5c の入力の見本と同じく、「同期処理なら簡単だが、JavaScript では非同期処理が要るので複雑になるプログラム」の見本にする。今の ConJS には、ファイル全体を読み書きする `fs.readFile` と `fs.writeFile` だけがある（P5、4.4 節） | 見本が 3 つの VM で動き、Node.js で同じことを書いた場合と比べて、README に載せる |

## 6. 決めること

1. **プラットフォームの順**（決めた、2026-11-19）：P3 は、ブラウザの canvas から始める。手間が小さく、目に見えて確かめやすい。P6 の順は、P3 の後で決める。P6 は、まず端末から始める（2026-11-21。4.7 節、段階 P6a）。その後の順は、P6a の後で決める。
2. **見本の言語**（決めた、2026-11-19）：P5 の見本の言語は、**代数的エフェクトとハンドラの小さな言語**（Koka や Eff のような）にする。ハンドラは、捕まえた継続を値として扱うだけ（`__Cont_get__`）で、ConMa で作りやすい。構文を自分で決められるので、SJS と違う構文で、キットが言語によらないこと（4 節）も確かめられる。
    * M1 は、初め「待つ処理を直接スタイルで書ける、JavaScript の部分集合」を見本の言語に勧めた（aae726a）。M2 の意見（docs/dev/REVIEW-ja.md の 2026-11-19（3））を受けて、名前を **ConJS** にし（「直接スタイル」は、ふつうの JavaScript もそうなので区別できない。「await なしの JS」は非同期ができないように聞こえる。ConMa、ConSh と並ぶ名前にした）、見本の言語ではなく、**SJS の次の段**として作る（4.1 節、段階 P3b）。JavaScript の構文なので、キットが構文によらないことの確かめにならないため。
    * 比べた候補：Lua のサブセット（M2 の初めの案。キットの確かめにはなるが、C の処理系がどこでも動くので、ConMa に変換する得は小さい）、小さな Lisp（S 式で、キットの字句解析と構文解析をほとんど確かめない）、Forth のようなスタックの言語、アクターやチャネルの並行言語、論理型言語の小さなもの。
    * 変えた（2026-11-20。作者の判断）：P5 では見本の言語を作らない。作ろうとしているのは ConJS で、キットが言語によらないことを確かめることではないため。言語は数限りなくあるので、どんな言語にも使えるキットを目指しても切りがない。キットは ConJS と SJS のための部品とする。M1 が書いた代数的エフェクトの言語の案（ConEff）は、計画に入れなかった。P5 は、ConJS をふだん使えるものにする段階にした（4.4 節）。
3. **ホストを呼ぶ口の形**：すぐ返る呼び出しは P2 で作った（8.2 節。Dict はまだ渡さない）。**待つ呼び出しの形は、P3 の初めに決める**（決めた、2026-11-19）。canvas のアニメーション（`requestAnimationFrame` を待つ）と ConJS（4.1 節）の両方に要るので、見本の言語より先に決める（M2 の意見の 4）。
4. **言語キットの置き場所**：標準ライブラリ（include/）に入れるか、別のディレクトリにするか。決めた（2026-11-20）：include/lang/（4.3 節）。
5. **言語のディレクトリの置き場所**：sjs/ と同じくルートに並べるか、`langs/` のような 1 つのディレクトリの下にまとめるか（まとめるなら sjs/ も移す）。決めた（2026-11-20）：ルートに並べたまま（4.3 節）。

## 7. 気を付けること

* **トレースとプロファイルの道**：`__Feature_available__ "wasm"` は、wasm 版の VM でも、`--trace` と `CONMA_PROFILE` のときは偽になる（`__Wasm_call__` が WebAssembly を使わないのと揃えた）。これで道を分けるプログラムは、トレースやプロファイルを取ると、ふだんと違う道（WebAssembly を使わない道）を通る。デバッグやプロファイルで見ているのが、ふだんの道でないことがある。
* **トレースの一致**：Python 版と Rust 版の VM はステップごとに同じトレースを出し、テストがそれを比べる。ホストの呼び出しは外の世界に触れるので、テストでは、どの VM にも同じ、決まった値を返すホストの関数を使う。
* **コア ConMa を小さく保つ**：プラットフォームの機能は、コア ConMa にも拡張の機能にも入れない。持たない VM では「使えない」になるだけで、言語の意味は変えない。
* **変換したコード**：transpiled VM は、`__Host_call__` を、VM に任せる呼び出し（変換したコードから VM に戻る所）として扱うところから始める。速さが要れば、後で直接呼ぶ。
* **wasm 版の大きさ**：プラットフォームの機能を足しても、wasm のファイルは大きくしない（ホストの関数は JavaScript の側に置く）。
* **Node.js で、決まらない Promise を待つとき**（2026-11-21。8.8 節。M2 の PR shima3/conma#14 のレビューの 2）：wasm 版の VM を Node.js で動かすと（vm/wasm/vm-node.js）、ホストの関数が返した Promise がいつまでも決まらず、ほかに動かせる処理もないとき、プロセスが何も出さずに終了ステータス 0 で終わる。テストはログと比べるので見逃さないが、手で動かすと、正しく終わったように見える。直すなら、vm-node.js が「待っている呼び出しがあるのに、起こすものがない」ことを見つけて、行き詰まりとして知らせる（P7 でホストの関数を足す口を作るときに考える）。 → 段階 P7 で直した（2026-11-22。8.11 節）：vm-node.js が、`process.on("beforeExit")` で見つけ、`vm-node: every VProc waits for host calls that never finish: N` を出して、終了ステータス 2 で終える。ブラウザのページは終わらないので、見つけない。Rust の API の待つ関数で、`Done` を持ったまま何もしないときも、見つけない（docs/HOST-ja.md に書いた）。

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

### 8.6. 段階 P5（2026-11-20）

4.4 節の設計（PR shima3/conma#8 で M2 がレビューした）で作った。使い方は conjs/README-ja.md。

* **ランタイムを先にモジュールにしておく**（bin/lang）：変換したソースの include の行（ランタイム）だけのファイルのモジュール（ランタイムのモジュール）を、bin/cache-dir の場所に「name」-rt-ハッシュ.module.se として 1 度だけ作り（変換器のモジュールと同じく、ソースか VM が新しくなったときだけ作り直す）、変換したソースは include の行を除いてモジュールにしてから、新しい `conma link`（vm/src/link.rs）で、大域変数の名前で合わせる。できるモジュールは、include の行ごとフロントエンドに通したものと、大域変数の番号とファイルの順のほかは同じで、同じステップ数で動く。変換器は変えていない。bin/lang を使う SJS（sjs/sjs）も、同じく速くなった。bin/lang は、VM を `exec` で動かすのをやめた（一時ディレクトリが消えずに残っていた）。
* **待つ呼び出しのライブラリ**（conjs/include/conjs_lib.se）：`sleep`、`setTimeout`、`Date.now`、`fs.readFile`、`fs.writeFile`、`fetch`（Response のようなオブジェクト。`json()` のための JSON の読み手もここにある）。ホストの関数 `time.now` と `time.sleep`（機能 `"time"`）を、Rust 版（transpiled VM も）と Python 版の VM の表と、vm/wasm/conma.js の既定のホストの関数に、`http.fetch`（機能 `"fetch"`）を conma.js に足した（ホストに `fetch` があるとき）。Rust 版と Python 版の VM の fetch は curl を使い、スケジューラに、待つための VProc でホストの呼び出しでない仕事をする `Kit_sched_wait_work` を足した（include/lang/sched.se。今の `Kit_sched_wait` は変えていない）。
* **テスト**：conjs/test の step7（タイマーと時刻）、step8（ファイル）、step9（fetch。step9/server.py の HTTP のサーバ）、step10（ConJS だけの名前を使わないプログラム）。step8 と step9 は、run_case.sh が一時ディレクトリに写してから動かす。conjs/test/compare.sh が step10 を Node.js と比べる。conjs/test/run_all.sh の最後に、ライブラリが使うホストの関数と機能の名前が docs/PRIMITIVES.md にあることの検査。`conma link` には Rust のテスト（`cargo test link`）。
* **確かめたこと**（Apple M1、Rust 版の VM で変換）：
    * bin/lang-emit-check（alpha の b181b90 と比べる）：作るコードは、すべてのソースで 1 文字も違わない。
    * bin/lang-step-check（同じ）：94 のプログラムで出力は同じ。SJS のプログラムのステップ数は同じ。ConJS の今までの 17 のプログラムは、どれも 407 ステップ多い（始めにライブラリの組み込みを置く分）。新しい step7〜10 は比べていない（BASE にない）。
    * `console.log("hi")` を conjs/conjs で動かす時間（実時間、3 回）：Rust 版の VM で 1.53〜1.57 秒から 0.09 秒、wasm 版の VM（Node.js）で 1.64〜2.33 秒から 0.18〜0.19 秒、Python 版の VM で 5.39〜7.10 秒から 3.75〜3.81 秒。どれも 4.4 節の終わりの条件（Rust 版で 0.2 秒以下、ほかの VM で遅くならない）を満たす。Python 版の残りは、ランタイムを含むモジュールを Python 版の VM が読む時間と、bin/vm が拡張 ConMa をコア ConMa に直す時間。
    * sjs/bench の nbody（大きさ 5000）を ConJS として conjs/conjs で動かす時間（実時間、3 回）：4.56〜4.74 秒から 3.06 秒。違いは、動かし始めるまでの時間で、動く速さは変わらない（合わせたモジュールは、前のモジュールと大域変数の番号のほかは同じ）。
* **設計と変えたところ**：
    * ランタイムと合わせるのを、`__VProc_spawn__` で動かす形（4.4 節。変換したコードは `Conjs_entry` だけを定義し、ランタイムの関数を Dict で受け取る）ではなく、モジュールを大域変数の名前で合わせる形（`conma link`）にした。前の形で書いた nbody（大きさ 10000）が、Rust 版の VM で 5.21 秒から 5.76 秒（3 回とも同じ）と約 10% 遅かったため（4.4 節の「遅くなるなら、この形を考え直す」）。ランタイムの関数を、大域変数でなく局所変数から読むので、入れ子の関数から、フレームの鎖をたどって読むことになる。合わせる形では、大域変数のままで、同じ nbody が 5.18〜5.28 秒（前は 5.20〜5.28 秒）。そのため、起動部（conjs/conjs_run.se）、`Conjs_entry`、ランタイムの関数の名前の一覧と Dict、その両方向の検査（M2 のレビューの 4）、1 つのモジュールにまとめる道は、どれも要らなくなった。変換器は変えず、ConJS が作るコードも変わらない（bin/lang-emit-check で確かめられる）。lang.conf に `runner` は足さず、bin/lang が変換したソースの include の行からランタイムのモジュールを決めるので、SJS も同じく速くなった。VM には、`conma link`（Rust 版の VM のサブコマンド。bin/lang だけが使う）を足した。
    * `time.sleep` は、Rust 版と Python 版の VM で、時間が来たら結果をパイプに書くスレッドで作ったが、待つための VProc が、結果を書いた後、パイプを閉じる前に起きることがあり、`__Host_result__` が「the call has not finished」になった（Python 版の VM の step7）。そこで、スケジューラが、ステップとステップの間に、時間の来た呼び出しを終える（書いて閉じる）形にした（`test.later` と同じ所。何も動けないときは、次の時間まで OS で眠る）。
    * Response の `json()` のために、ConJS の中に JSON の読み手を書いた（SJS には `JSON.parse` がない）。`JSON.parse` そのものは足していない（受け付ける JavaScript を広げる段階で決める）。
    * `Date` は、`now` だけを持つオブジェクトにした（`typeof Date` は JavaScript の `"function"` でなく `"object"`。`new` は SJS の範囲の外）。
    * step10 を書くときに、SJS に配列の `sort` がないことが分かった（ConJS を広げるときの候補）。
    * M2 の PR shima3/conma#9 のレビューを受けて直した：bin/lang は、ランタイムのソース（キャッシュの「name」-rt-ハッシュ.se）も、一時ファイルに書いてから mv で置く（同時に動く bin/lang が、書きかけのソースからモジュールを作らないように）。curl の道の fetch は、HEAD を `--head` で送る（`-X HEAD` の curl は、来ない本体を待って終了コード 18 で終わり、TypeError になっていた。step9 に HEAD の場合を足した）。bin/lang-step-check は、copy.txt のあるステップ（step8 と step9。ファイルを書くか、サーバが要る）を除き、BASE にないプログラムを動かさない（step8 のファイルを作業ツリーに残し、cd のエラーを出していた）。
    * ブラウザのページでの `fs`（VM のメモリの中のファイル）と、canvas の見本は、確かめていない（見本は変換器もランタイムも、作るコードも変わっていないので、vm/wasm/web/build.sh で作り直せば今と同じ）。

### 8.7. 段階 P5b（2026-11-20）

4.5 節の設計（PR shima3/conma#10 で M2 がレビューした）で作った。使い方は conjs/README-ja.md の「言語」。

* **構文解析と変換**（sjs/include/sjs_parse.se、sjs_gen.se）：ConJS のときだけ、名前の検査で `this`、`new`、`class`、`super` を通し、`instanceof` を受け付ける。class 宣言は、`let 名前 = class 名前 { ... }` の宣言の節にする（宣言より前に使えないのも let と同じ）。変換は、クラスの中のスコープの Dict に、this を読む変数（メソッドとフィールドでは、インスタンスの変数そのもの。`extends` のあるクラスのコンストラクタでは、`super(...)` が入れる Ref）と、今のクラスの記録の変数を置く。ふつうの `function` は、その Dict に「読めない」の印を置くので、中の `this` と `super` は変換するときのエラー（`conjs: this outside a class`、`'super' keyword unexpected here`）。アロー関数は、外のものを読む。SJS は、字句の名前の検査で先に止まるので、SJS が作るコードは変わらない。
* **ランタイム**（conjs/include/conjs_class.se）：`Conjs_class`、`Conjs_method`、`Conjs_static`、`Conjs_ctor`、`Conjs_fields`、`Conjs_new`、`Conjs_super_call`、`Conjs_super_get`、`Conjs_this`、`Conjs_instanceof` と、組み込みのエラーのコンストラクタ。クラスの値とインスタンスは、隠したキー `"cls"` にクラスの記録を持ち、記録の `"find"`（`Conjs_find_member`）がメソッドを探す。sjs/include/sjs_rt.se は、`Sjs_get_member` のオブジェクトの 2 つの道と関数の道、`Sjs_in_op` で、メンバーが自分のプロパティになかったときだけ `Sjs_class_member` を呼ぶ（新しい `Sjs_obj_member`、`Sjs_function_member`、`Sjs_obj_in`。メンバーがあるときの手順の数は、前の `Sjs_obj_get`、`Sjs_members_find`、`Sjs_obj_has` と同じ）。`Sjs_obj_get` と `Sjs_obj_has` は変えていない。sjs_inspect.se は、インスタンスを `Point { x: 1 }`、クラスを `[class Point]` と出す。
* **`JSON.parse` と `sort`**（sjs_rt.se）：conjs_lib.se の JSON の読み手を `JSON.parse` に移し、配列の `sort`（安定なマージソート）を足した。SJS と ConJS で共有する（sjs/README-ja.md の「JSON.parse と配列の sort」）。
* **テスト**：conjs/test の step11（クラス）、step12（エラーのコンストラクタと、エラーの場合）、step13（`JSON.parse` と `sort`）、sjs/test の step12。Node.js と比べる場合を、各ステップの node.txt に並べ、conjs/test/compare.sh は、ファイルを書かなければ、それらを比べる（step10 にも node.txt を置いた）。conjs/test/step6 の case6（上の階層の `this`）は、メッセージが `conjs: not in ConJS: this` から `conjs: this outside a class` になった。
* **確かめたこと**（Apple M1、Rust 版の VM で変換）：
    * conjs/test/compare.sh：node.txt の 6 つの場合（step10 の 2 つ、step11 の 2 つ、step12 と step13 の 1 つずつ）が、Node.js と同じ標準出力。sjs/test/step12 の case1 も、Node.js と、ConSh の段階 A（`sjs/test/compare.sh`）と同じ。
    * bin/lang-emit-check（alpha の 117e4c3 と比べる）：作るコードは、conjs/test/step6/case6（上のメッセージ）のほかは、すべてのソースで 1 文字も違わない。
    * bin/lang-step-check（同じ）：89 のプログラムで、出力は case6 のほかは同じ。SJS のプログラムは、始めに `JSON.parse` を置く分の 69 ステップのほかに、ないメンバーを読む所、`in`、`console.log` に渡すオブジェクトの数によって −4〜+64 ステップ違う（81 のうち 25 は 69 ステップちょうど。いちばん多い sjs/test/step8 の case12 は、さらに 784）。ConJS の今までのプログラムは 502〜574 ステップ多い（始めにエラーのコンストラクタを置く分）。sjs/bench の loop、fib、nbody は 69 ステップだけ多い（計算の中のステップは同じ）。
    * nbody（大きさ 2000）を ConJS で動かす時間（user + sys、3 回の最良）：物体を配列にした今の nbody（sjs/bench/nbody.safe.js）が 1.46 秒、物体をクラス Body にした conjs/bench/nbody_class.con.js が 2.33 秒、そのうえで、ループの中のメソッドの呼び出し（`b1.pull(b2, dt)`、`b.move(dt)`）をふつうの関数の呼び出しにしたものが 2.21 秒。メソッドを読むたびに関数を作る分は約 5% で、残りの違いは、フィールドを名前で読む（配列は、数の添字の速い道 `Sjs_get_index` を通る）ことから来る。そのため、4.5 節の遅いときの案（`Conjs_call_member`）は作らなかった。
* **設計と変えたところ**：
    * ふつうの関数の `new` のメッセージは、JavaScript の `F is not a constructor` でなく、`a function is not a constructor`（SJS の `Sjs_describe`。`... is not a function` と同じ書き方）。
    * 足さないもの（getter、計算した名前、static のフィールドとブロック）のメッセージは、ほかの範囲の外のものと同じく `conjs: not in ConJS: a getter or a setter` などにした。
    * メッセージが空のエラーの `String(e)` を、名前だけにした（`new Error()` を `Error` と書く。JavaScript と同じ）。
    * 配列の `sort` に、関数でない compare を渡したときのメッセージは、Node.js と同じく値を付ける（`...: 1`）。
* **M2 の PR のレビューで直したこと**（PR shima3/conma#11）：
    * コンストラクタが返すオブジェクトを、`new` の値にする（JavaScript と同じ）。親のコンストラクタが返したものは、子の `this` にしない（conjs/README-ja.md の違い）。
    * エラーの `name` と `toString` を、組み込みのエラーのコンストラクタの記録の、メソッドの表に置く（JavaScript のプロトタイプと同じ所）。`this.name = "MyErr"` が `Object.keys` と JSON に出て、`new Error().toString()` が動く。ConJS では、ランタイムが投げるエラーも、同じ名前のエラーのコンストラクタの記録を持つ（sjs_rt.se の `Sjs_error_object`。エラーを作るたびに 24 ステップ多い）。
    * `JSON.parse` は、JSON の数の形（`+1`、`01`、`1.`、`.5` は SyntaxError）と、文字列の中の生の制御文字を確かめる。大きな JSON の parse は、1 割ほど遅くなった。

### 8.8. 段階 P5c（2026-11-21）

4.6 節の設計（PR shima3/conma#12 で M2 がレビューした）で作った。使い方は conjs/README-ja.md の「入力を待つ」。

* **ホストの関数**（vm/wasm/host/input.js。機能 `"input"`。docs/PRIMITIVES.md）：`ConmaInput(canvas, options)` が、`input.next ms`（待つ）と `input.poll`（すぐ返る）を返す。keydown（options の `keyTarget`。既定は canvas）と、canvas の pointerdown を列にためる。待ちの列は、待ち始めた順に入力を渡し、時間切れの待ちは列から外す。時計（`setTimeout`、`clearTimeout`）は、options で替えられる。
* **ConJS のライブラリ**（conjs/include/conjs_lib.se）：`nextEvent(ms)`（`Conjs_next_event`。ms は `Conjs_event_ms` で、`undefined` と `Infinity` を -1 にする）と `pollEvent()`（`Conjs_poll_event`）。入力の LList を、`Conjs_event_object` がオブジェクトにする。機能がなければ TypeError（`Conjs_input_check`）。
* **見本**：vm/wasm/web/stars.con.js（「星ひろい」）と stars.html。vm/wasm/web/build.sh が、input.js と、変換したモジュール（stars_conjs.module.se。git には入れない）を置く。アプリの中のブラウザで、タイトルからクリックで始め、矢印のキーで動いて星を拾い、おばけにぶつかって結果の画面に移り、キーでタイトルに戻り、Escape で終わる（`stars: bye`）ことと、コンソールにエラーがないことを確かめた。面をクリアする道は、星を 1 つにした写しを、入力のまねで動かして確かめた（git には入れていない）。
* **テスト**：conjs/test/step14。case1（`nextEvent` と `pollEvent`）と case2（見本。conjs/test/step14/case2.con.js は vm/wasm/web/stars.con.js へのシンボリックリンク）。wasm 版の VM では、case*.host.js が、本物の input.js を、入力のまね（conjs/test/step14/input_mock.js。`input.next` が呼ばれた数で手順を進め、手で進める時計を渡す）で動かす。case1 は、待つ人がいないときの入力をためること、2 つの処理が同時に待つときの順、時間切れの待ちが後の入力を取らないこと、ms の扱い、手順を使い終わった後の Error を確かめる。Rust 版と Python 版の VM では、機能がないことを確かめる（case*.log。wasm 版は case*.wasm.log）。
* **確かめたこと**（Apple M1）：conjs/test が Rust 版（15 秒）、wasm 版（19 秒）、Python 版（3 分 30 秒）で通る。conjs/test/compare.sh が通る。bin/lang-emit-check（alpha と比べる）に違いがない（SJS と、今の ConJS のプログラムが作るコードは変わらない）。
* **見つけたこと**（P5c では直さない）：
    * フロントエンド（`conma build`）の時間が、1 つの CPS の鎖の長さ（続けて束縛する数）に対して、2 乗のように増える。ConJS のプログラムは、ファイルの文の並び全体が 1 つの鎖になるので、文が多いと遅い。`const x = [...]` を並べただけのプログラムで、100 文 0.46 秒、200 文 0.91 秒、400 文 2.23 秒、800 文 6.48 秒（Rust 版の VM。変換を含む）。stars.con.js（217 行。変換したものは約 88 KB で、束縛は約 1770）は、フロントエンドに約 6.6 秒かかる（canvas_demo.con.js は、束縛約 450 で約 0.1 秒）。そのため、conjs/test の step14 の case2 は、どの VM でも 7 秒ほどかかる。ページのモジュールは build.sh が先に作るので、遊ぶときには関係ない。
    * wasm 版の VM を Node.js で動かすとき（vm/wasm/vm-node.js）、ホストの関数が返した Promise がいつまでも決まらず、ほかに動かせる処理もないと、Node.js のイベントループに何も残らないので、プロセスが何も出さずに終了ステータス 0 で終わる（vm-node.js は、`jsPending()` が 0 でないので、待つ Promise を作って待つが、それを起こすものがない）。ネイティブの VM なら待ち続ける所である。ページでは待ち続ける（正しい）。入力のまねの手順を間違えたときに見つけ、`host.wait` で決まらない Promise を返すホストの関数で確かめた。手順を使い終わった後の `input.next` を Error にしたので、テストでは起きない。

### 8.9. 段階 P6a（2026-11-21）

4.7 節の設計（PR shima3/conma#15 で M2 がレビューした）で作った。使い方は conjs/README-ja.md の「入力を待つ」。

* **端末の入力**（新しい conjs/include/conjs_term.se。conjs_rt.se が include する）：`Conjs_term` が、初めて呼ばれたときに、`stty -g` と `stty -icanon -echo`（`Conjs_term_raw`）、列を受け持つ VProc（`Conjs_term_owner`）、読み手（`Conjs_term_reader`）、端末なら Ctrl-C を待つ VProc を始める。知らせ（"wait"、"poll"、"data"、"end"、"timeout"、"esc"）と、キーに分ける所（`Conjs_term_keys`）は 4.7 節のとおり。
* **ConJS のライブラリ**（conjs_lib.se）：`nextEvent` と `pollEvent` は、機能 `"input"` がなければ `Conjs_term_next` と `Conjs_term_poll` を使う。`Conjs_input_check`（機能がなければ TypeError）はなくした。
* **片付け**：`Conjs_run` の終わりと、sjs_rt.se の `Sjs_uncaught`（PDict の `"sjs_cleanup"`）が `Conjs_term_cleanup` を呼ぶ。端末のモードを戻してから、受け持つ VProc、時間切れの VProc、読み手、Ctrl-C を待つ VProc を止める。Ctrl-C は `Conjs_term_interrupted` が、モードを戻して終了ステータス 130 で終える。
* **見本**：vm/wasm/web/stars.con.js は、canvas がなければ端末に描く（`paintTerm`。自分は `@`、星は `*`、おばけは `G`。変わったときだけ描く）。入力の終わりの Error は受け止めて、`stars: the input has ended` と出して終える。遊ぶ所で入力が尽きても、おばけと描く処理を終えてから抜ける（`try`／`finally`）。
* **道具**：bin/lang-test は、`case*.in` があれば、それを標準入力にする。
* **テスト**：conjs/test/step15（case1 は、キーの名前、1 回の読みからいくつものキー、列、入力の終わりの Error。case2 は、見本を Escape で終える）。test/fast18（ESC の並びが 2 回の読みに分かれる場合。4.7 節では step15 の case3 にするとしたが、ConJS のプログラムからは ConMa の関数を呼べないので、ConMa のテストにした）。conjs/test/step14 の Rust 版と Python 版のログを、端末の道（標準入力は /dev/null）のものに変えた。
* **端末で確かめたこと**（Apple M1。Python の pty で、プログラムを本当の端末で動かし、キーを送って、端末のモードを調べた）：
    * Rust 版の VM：読んでいる間は `icanon` と `echo` が切れ、キーが Enter なしで届く。見本をタイトルで Escape、遊んでから Escape、Ctrl-C（終了ステータス 130）、受け手のない例外（終了ステータス 2）の 4 つの終わり方で、モードが元に戻る。
    * Python 版の VM：Escape と Ctrl-C で、同じく戻る。
    * wasm 版の VM（Node.js）：モードは、どの終わり方でも戻る。ただし、見本は、`stars: bye` を出した後もプロセスが終わらない（Ctrl-C でも）。`pollEvent` を 1 回呼ぶだけのプログラムは終わる。原因は、まだ分からない（4.7 節の終わりとすることは、Rust 版の VM の端末）。
* **設計と変えたところ、見つけたこと**：
    * wasm 版の VM（Node.js）では、設定を戻す `stty` を始めるだけで、終わるのを待たない（`Conjs_term_restore`）。Node.js が標準入力の読みをスレッドで待ったままのとき、設定を戻した `stty` は、設定を変えた後も終わらず、待つと VM が終わらなかった。
    * 片付けで、読み手を止める前に標準入力を閉じる（`__OS_close__`。止めるだけでは、wasm 版の VM が待っているものとして数えるかもしれないため）。上の wasm 版の VM の残りは、これでも直らなかった。
    * `stty` を動かす `__OS_spawn__` には、コマンドを LList でなく MSeq で渡す（初め LList で渡して失敗し、端末のモードが変わらなかった）。
    * 4.7 節の「ブラウザで input.js を渡さないページは、今のとおり TypeError」は、ページでも標準入力を読む道に入り、入力の終わりの Error になる（機能で、ページかどうかを見分けないことにした）。
* **M2 の PR のレビューで直したこと**（PR shima3/conma#19）：
    * キーの列を、スケジューラの "in" と "out" と同じく、2 つの LList（"kin" と "kout"）にした。前は、1 つ足すたびに列全体を作り直していたので、1 回の読みで多くのキーが届くと、時間がキーの数の 2 乗で増えた（`a` を 8,000 個読むのに、M2 が測って 24.8 秒）。直した後は、1,000、2,000、4,000、8,000 個で 0.16、0.22、0.35、0.70 秒（Rust 版の VM。動かし始める時間を含む）。
    * 止めた時間切れの VProc の書き手を、受け持つ VProc が閉じる（`Conjs_term_remove_timer`）。閉じないと、読み手が閉じていない鎖の読み位置を持ち続け、見本で動くキーを押すたびに 1 つずつ増えた。

### 8.10. 段階 P8（2026-11-21）

4.8 節の設計（PR shima3/conma#20 で M2 がレビューした）で作った。使い方は conjs/README-ja.md の「ファイルを 1 行ずつ読む」。

* **ファイル**（新しい conjs/include/conjs_file.se。conjs_rt.se が include し、conjs_lib.se の `Conjs_lib_builtins` が `Conjs_file_builtins` を呼ぶ）：`fs.open`、`fs.readLines`、`fs.stdin`、`fs.stdout`。ファイルのオブジェクトは、ConJS のクラスの記録 "File"（`Conjs_record`）を持つ SJS のオブジェクトで、自分のプロパティは `path` だけ。メソッドと `closed` は、記録のメソッドの表から見つける（`closed` を作る Closure は、真偽を返す。エラーの `name` と同じ）。隠したキーに、Stream、読んだ文字列のバッファとその中の位置、終わりに着いたか、閉じたかを持つ。
* **読み方**：`Conjs_file_next_line` が、バッファの位置から `\n` を探し、なければ `Conjs_file_fill` が `__OS_read__` で読み足す（まだ返していない所の後ろに付け、返した前の部分は捨てる）。`""` のときは、`__OS_wait_readable__` を `Kit_sched_wait_work` の VProc で待つ。`readLines()` は `Sjs_generator` で作り、本体を `Sjs_try` で包んで、どう抜けても閉じる。
* **標準入力**：PDict の `"conjs_stdin"` に、先に使った読み方（"keys" か "lines"）を置き（`Conjs_stdin_use`）、もう一方は Error。`readLines()` が終われば `fs.stdin` も閉じたことになる（本当の標準入力は閉じない）。
* **`argv`**（設計に足したもの）：見本の merge.con.js にファイルの名前を渡すため、プログラムの引数を `argv` に置いた。変換した main に `Conjs_argv G args` の 1 行を足し（sjs/include/sjs_gen.se の `Gen_conjs_program`。SJS の main は変えない）、main が VM から受け取る引数の Seq を配列にする。conjs/conjs と bin/lang は、ファイルの後の引数を VM に渡し、bin/lang-test は `case*.args` を引数にする。
* **見本**：conjs/examples/merge.con.js と merge.node.js（`comm` と同じ）、wordcount.con.js（語を数える。ConJS には正規表現がないので、1 文字ずつ見て語に分ける）。conjs/test/compare.sh は、ファイルを書かなければ、conjs/test/node-pairs.txt の組（merge の 2 つ。a と b を入れ替えたものも）も比べる。
* **テスト**：conjs/test/step16（case1〜5。4.8 節のとおり。case5 は merge.con.js へのシンボリックリンク）。
* **確かめたこと**（Apple M1）：conjs/test が Rust 版、wasm 版、Python 版で通る。conjs/test/compare.sh（node.txt の場合と、node-pairs.txt の組）が通る。sjs/test、consh/js/test が通る。bin/lang-emit-check（alpha と比べる）：SJS のプログラムが作るコードは変わらない。ConJS のプログラム（42）は、main の `Conjs_argv` の 1 行だけ違う。
* **大きなファイル**（手で測った。Rust 版の VM、Apple M1。「n words on the line」の形の行、動かし始める約 0.09 秒を含む）：

| 行の数（大きさ） | `fs.readLines` で数える | `fs.readFile` と `split("\n")` で数える |
|---|---|---|
| 10 万（2.9 MB） | 2.30 秒 | 0.46 秒 |
| 100 万（29.9 MB） | 22.1 秒 | 21.2 秒 |

`fs.readLines` は行の数に比例する（1 行およそ 22 マイクロ秒）。`fs.readFile` と `split` は、10 万行では速いが、100 万行では 46 倍になり、行の数に比例しない（大きな文字列を作って分けるため）。

* **M2 の PR のレビューで直したこと**（PR shima3/conma#21）：
    * とても長い 1 行は、前は長さの 2 乗に比例した（4,096 バイトごとに、まだ返していない所の全体をつなぎ直し、`\n` も初めから探し直していたため。M2 が測って、100 万文字の行が ASCII で 0.19 秒、ひらがなで 1.93 秒、200 万文字で 0.43 秒と 7.55 秒）。読んだかたまりを列にため、`\n` を含むかたまりが来てから 2 つずつつないで 1 回でまとめ（`Conjs_file_gather`）、`\n` は、もう探した所の後ろから探す（"fs_scan"）ようにした。直した後は、ASCII で 100 万、200 万、400 万文字が 0.11、0.11、0.12 秒、ひらがなで 100 万、200 万文字が 0.12、0.14 秒（動かし始める時間を含む）。ふつうの行は、少し遅くなった（10 万行が 2.30 秒から 2.49 秒、100 万行が 22.1 秒から 24.0 秒）。
    * 単独の `\r` は行の終わりにしない（Node.js の readline と違う）ことを、README の JavaScript との違いに書いた。
    * 上の 1 つ目の直しで、読んだかたまりを処理ごとの列にためたため、2 つ以上の処理が同じパイプから読むと、待つ所をまたいで、ほかの処理が読んだものと順が入れ替わり、行が壊れるようになった（M2 が確かめで見つけた。前の版では起きなかった）。かたまりは、ファイルのオブジェクトの隠したキー "fs_chunks" にためるようにし（`Conjs_file_read_more`）、待った後は、行を探すところからやり直す。テストに、少しずつ届く標準入力を 3 つの処理で取り合う case6 を足した（bin/lang-test は、`case*.feed.py` の出力をパイプで標準入力に渡す）。case6 の入力は、M2 の確かめと同じ形（乱数の種を決めた、行の途中で分けた送り方）で、前の版では 3 回のうち 2 回、行が壊れた。

### 8.11. 段階 P7（2026-11-22）

4.9 節の設計（PR shima3/conma#22 で M2 がレビューした）で作った。使い方は、新しい docs/HOST-ja.md（使う人のための 1 か所）。

* **文書**：docs/HOST-ja.md に、呼ぶ側（ConJS と ConMa）、渡せる値、足す側（ブラウザ、Node.js、Rust 版の VM と transpiled VM）、どの VM にもある関数、テストをまとめた。今までの 5 か所（conjs/README-ja.md、vm/wasm/conma.js の先頭のコメント、vm/wasm/host/canvas.js、vm/wasm/vm-node.js、docs/PRIMITIVES.md）は、詳しいことを残し、docs/HOST-ja.md への参照を足した。vm/README.md にも、ライブラリとして使えることを書いた。
* **クレートを lib と bin に分けた**：vm/src/lib.rs（今までの main.rs の中身と、`run_main`、`run_main_with`、`Host`、`HostValue`、`Done`、`Pool` の re-export）と、`conma::run_main` を呼ぶだけの vm/src/main.rs。VM のアロケータ（`Pool`）は、bin の側の `#[global_allocator]` に置き、組み込むプログラムにも同じ 1 行を勧める（docs/HOST-ja.md）。wasm 版のクレート（vm/src のファイルを `#[path]` で使う）と transpiled VM は、作り方を変えずに作れた。
* **ホストの関数の表**（vm/src/prim/host.rs）：決まった表（`BUILTIN`、`NATIVE`）と `feature()` を、`Machine` が持つ `Host` に替えた。`Host::new()` が `test.add`、`test.echo`、`test.fail` と、Rust 版と transpiled VM では `time.now` と機能 `"time"` を登録する（同じ API）。`test.later` と `time.sleep` は、今までどおり `__Host_start__` の中で扱う。`run_main_with` は、VM のスレッドに `Host` を渡す（`set_host` と、`Machine::new` の `take_host`）。
* **待つ関数**：`Host::waiting` の関数は、`__Host_start__` が `Done` を付けて呼ぶ。`Done::finish` は、結果を `Mutex` の列に入れ、起こすためのパイプ（ブロックしない。`sys::make_wake_pipe`）に 1 バイト書く。スケジューラは、ステップの間で列を見て（`any_finished` の 1 回の読みだけ）、呼び出しを終える。どの VProc も動けないときは、起こすためのパイプも `poll` で待つ。列とパイプは、`Done` と共有する `Arc` が持ち、VM と最後の `Done` がなくなったときに閉じる。`finish` を呼ばずに捨てた `Done` は、`名前: the host dropped the call` のエラーで終える（`Drop`）。`__Host_call__` で待つ関数を呼ぶと、`__Host_call__: 名前 waits: call it with __Host_start__`。
* **結果の置き場所**（M2 のレビューの 1）：結果を `Machine` の表（`HostResults`。呼び出しの Stream の serial から、結果へ）に置き、パイプは閉じるだけにした（`__Host_result__` は、パイプの終わりを見てから、表の結果を取る。表にないときは、今までどおり、wasm 版のホストが書いた JSON を読む）。`capped` と `PIPE_MAX` はなくなった。Stream が閉じられた呼び出しの結果は捨て、次に結果を置くときに、閉じた Stream の分を表から除く。
* **設計と違うこと**：Python 版の VM も、8 KB の限り（`HOST_PIPE_MAX`）を残さず、結果を Stream に置いてパイプを閉じるだけにした（`_host_done`）。テストのログ（test/fast15 と conjs/test/step5 の 128 KB の結果）を、すべての VM で同じにするため。Python 版の VM の JSON の道（`_host_json` など）は、使わなくなったので除いた。
* **値**：真偽と、キーが文字列の Dict を、すべての VM で受け渡せるようにした（Rust の `HostValue::Bool` と `Dict`。真偽は `boolean_choice` で見分け、返すときは `true_closure` と `false_closure`）。wasm 版のホストとは、真偽は JSON の真偽、Dict は `{"d":{...}}`（vm/wasm/conma.js の `hostDecode` と `hostEncode`。JavaScript の側は、ふつうのオブジェクトだけを返せ、`Date` などは関数のエラー）。Python 版の VM にも同じものを足した（`_HostDict`、`_host_boolean`。モジュールの関数の本体には SInfo があるので、それを飛ばして見分ける）。ConJS は、オブジェクトの自分のプロパティだけを Dict にして渡し（M2 のレビューの 4）、ホストの Dict をオブジェクトにする（conjs/include/conjs_rt.se の `Conjs_to_host`、`Conjs_from_host`）。
* **決まらない Promise**（7 節）：vm/wasm/conma.js が、待つホストの呼び出しの数を、ほかの JavaScript の呼び出しと分けて数え（`vm.hostPending()`）、vm-node.js が `beforeExit` で、`vm-node: every VProc waits for host calls that never finish: N` を出して、終了ステータス 2 で終える（M2 のレビューの 3）。
* **見本とテスト**：vm/examples/host_demo.rs（`demo.add`、Dict と真偽を返す `demo.info`、スレッドで終える待つ `demo.delay`、`Done` を捨てる `demo.drop`、機能 `"demo"`）と、それを呼ぶ conjs/examples/host_demo.con.js、同じ関数を JavaScript で書いた conjs/examples/host_demo.host.js。test/host-api.sh（新しい）が、`cargo build --release --example host_demo` を自分で呼び（M2 のレビューの 5）、Rust 版と wasm 版で同じログ（test/host-api/demo.log）になること、Rust の API だけのこと（test/host-api/rust.con.js：待つ関数を `host.call` で呼んだときのエラー、5 つの処理から同時に待つこと、`host.available`）、関数を足さない VM で `host.available("demo")` が偽になることを確かめる。bin/lang は、`CONMA_RUNNER` があれば、その実行ファイルでモジュールを動かす。test/fast14 と fast15（Dict と真偽、Number のキーの Dict と真偽でない Closure のエラー、128 KB の結果、wasm 版のホストが返す真偽とオブジェクトと `Date`）、conjs/test/step5（オブジェクト、クラスのインスタンス、関数と `null` を渡すエラー、128 KB の結果）を書き換え、conjs/test/step17（決まらない Promise）を足した。
* **確かめたこと**（Apple M1）：test/ が Rust 版、Python 版、wasm 版（違いは既知の step40 だけ）、transpiled VM で通る。consh/test が Rust 版と transpiled VM で、consh/js/test、sjs/test が通る。conjs/test が Rust 版、wasm 版、Python 版で、conjs/test/compare.sh が通る。test/host-api.sh が Rust 版と wasm 版で通る。vm の `cargo test --release` が通る。bin/lang-emit-check（alpha と比べる）：SJS のプログラムが作るコードは変わらない。ConJS は、書き換えた conjs/test/step5 の場合だけ違う。
* **M2 のレビューで直したこと**（PR shima3/conma#26）：自分を含む値（`__Dict_put__ d "self" d` の Dict、ConJS の `o.self = o`）を渡すと、Rust 版の VM は `to_host` の再帰でスタックがあふれて落ち、ConJS は `Conjs_to_host` が止まらずにメモリーを使い続けた。今たどっている Dict（Rust 版はポインター、Python 版は `id`）と、ConJS のオブジェクトと配列（`"id"`）を覚え、また出会ったら、ConMa では `onError`（`got a Dict that contains itself`）、ConJS では `TypeError`（`host.call: cannot pass an object that contains itself to the host`）にした。同じ値を 2 か所に持つだけなら、今までどおり渡せる。test/fast14 と conjs/test/step17 の case2 に足した。値が `undefined` のプロパティは、今の形（`TypeError`）のままにし、docs/HOST-ja.md に書いた。

