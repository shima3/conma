# ビット演算のプリミティブの計画

ConMa にビット演算のプリミティブを足す計画である（2026-11-12）。今の ConMa には、ビット演算がない。bench/ の richards と、SJS のビット演算子（consh/sjs）は、算術でビットを 1 つずつ求めていて、遅い。

## 1. 目的と、測った効果

* **bench/ の richards**：アイドルのタスクが `control ^ 0xD008` を、2 で割った余りを 1 ビットずつ比べる ConMa の関数（`xor_bits`）で求めている。`richards 2` のステップの 29.9% がこの関数だった（CONMA_PROFILE）。
* **試した**：Rust VM だけに `__Number_xor__`（0 以上の 2 つの整数の XOR）を足し、richards の `xor w half mask` の 1 行をそれに替えて、`richards 10` を同じ VM で測った（5 回の最良。出力は同じで、CPython とも同じ）。

| richards 10（Rust VM） | 時間 | VM のステップ |
|---|---|---|
| 今（ConMa の `xor_bits`） | 3.527 秒 | 145,505,547 |
| `__Number_xor__` を使う | 2.465 秒（−30%） | 101,449,407（−30%） |
| CPython（参考） | 0.294 秒 | |

* **SJS**：`&`、`|`、`^` は、32 ビットを 1 つずつ求める `Sjs_bits`（consh/include/sjs_rt.se）で動く。ループの 1 回あたりのステップは、`x = x + i` が 45、`x = x ^ i` が 4,573、`x = x & 1023` が 4,505、`x = x << 1` が 359 だった（Rust VM、CONMA_PROFILE）。ビット演算は、足し算の約 100 倍かかる。
* ほかのベンチマーク（bench/ の nbody、fib、loop、consh/bench、consh/sjs/bench）はビット演算を使わないので、変わらない。

## 2. 決めたこと（2026-11-12、ユーザーと）

* **高速版 ConMa に足す**（docs/FAST-PLAN-ja.md）。簡潔版 ConMa（docs/SPEC.md、docs/PRIMITIVES.md の言語）は変えない。高速版の VM（Rust、wasm、変換版）はプリミティブとして動かし、簡潔版の VM（Python）は、変換器 bin/concise で簡潔版に直したモジュールを動かす。高速版のプリミティブを足すのは、これが初めてになる（F2〜F5 は、言語を変えない VM の動きだった）。
* **被演算子は JavaScript と同じ 32 ビット**：被演算子を ToInt32 で 32 ビットの整数にしてから求め、結果は符号付きの 32 ビットの整数（`>>>` だけ符号なし）。整数でない数は 0 の方へ切り捨て、`NaN` と無限大は 0 になる。エラーはないので onError は要らない。richards の値は 2^16 より小さいので、結果は今と同じ。
* **JavaScript の 7 つの演算**：and、or、xor、not、左シフト、符号付きの右シフト、符号なしの右シフト（`& | ^ ~ << >> >>>`）。シフトの量は、右の被演算子の ToUint32 の下 5 ビット。
* **数の関数（`exp`、`log`、`sin` など）は別にする**：JavaScript（V8 は fdlibm を移したもの）と Rust や Python（その環境の libm）で、最後のビットが違うことがあり、どこまで合わせるかを別に決める。ビット演算は、どの VM でも結果がちょうど決まる。

## 3. 形

* **名前**：`__Number_and32__ a b ,(r)`、`__Number_or32__`、`__Number_xor32__`、`__Number_not32__ a ,(r)`、`__Number_shl32__ a n ,(r)`、`__Number_shr32__`、`__Number_ushr32__`。`32` は、32 ビットの意味であることを名前に出すため。被演算子が Number でなければ、ほかの Number のプリミティブと同じく VProc をエラーで止める。
* **機能の名前**：`(Uses "bitwise32")`（FAST-PLAN-ja.md の 3.4）。7 つのどれかを、定義のない大域の名前（プリミティブ）として使うモジュールに、フロントエンドが付ける。プログラムが同じ名前の関数を自分で定義していれば、付けない。
* **代わりの定義**：include/fast/bitwise32.se に、7 つを、同じ名前の簡潔版 ConMa の関数として書く（今の consh/include/sjs_rt.se の `Sjs_to_int32`、`Sjs_bits` などを、ConMa 全体の道具にしたもの）。大域の名前に定義があれば、プリミティブでなくその関数が呼ばれる（Python VM と Rust VM で確かめた。高速版の VM に簡潔版に直したモジュールを渡しても、定義のほうが動く）ので、変換器は定義を足すだけでよい。
* **変換器**：bin/concise は、`(Uses "bitwise32")` のあるモジュールに、代わりの定義のモジュール（include/fast/bitwise32.se をフロントエンドで作ったもの）の大域の定義を、名前で突き合わせて入れる（モジュールの G の未定義の項目を定義に替え、定義が使う大域の名前は、同じ名前の項目に、なければ新しい項目にする）。`(Uses)` は消す。

## 4. 作り方（段階）

各段階は、そこまでのテストが通り、test/ と consh/test がすべての VM で通ることで終わりとする。

| 段階 | 内容 | 確かめ方 |
|---|---|---|
| W1 | 代わりの定義（include/fast/bitwise32.se）。今の名前のまま、簡潔版 ConMa の関数として、Python VM と Rust VM で動かす | 負の数、2^31 と 2^32 の前後、整数でない数、`-0`、`NaN`、無限大、シフトの量の 32 以上と負の数を、Node.js で求めた値と比べる新しいテスト（test/ の step） |
| W2 | 高速版の VM のプリミティブ：Rust VM（vm/src/prim/number.rs、prim/mod.rs、`INLINE_PRIMS`、`FAST_FEATURES` に `"bitwise32"`）、wasm 版（consh/js/build.sh で作り直す）、変換版の VM（vm/conma2rust/conma2rust.py の `FAST_FEATURES` と表、vm/src/transpiled.rs） | W1 のテストを、`(Uses "bitwise32")` を付けたモジュールで、Rust、wasm、変換版の VM で動かし、代わりの定義と同じ値になること |
| W3 | 変換器：bin/concise が `"bitwise32"` を代わりの定義に直す。test/vm.sh は、Python VM で動かすとき、`Uses` のあるモジュールを bin/concise で直してから動かす | 比べるテスト（FAST-PLAN-ja.md の 7）：同じプログラムを、高速版の VM と、変換して Python VM で動かした結果が同じこと。代わりの定義とプリミティブで、同じ結果になること |
| W4 | フロントエンドが `Uses` を書く：Python 版（bin/resolver）と ConMa 版（include/resolver.se。vm/frontend.module.se を bin/update-frontend で作り直す）。7 つを未定義の大域の名前として使えば `(Uses "bitwise32")` | 両方のフロントエンドで、同じモジュールが、バイトまで同じになること（test/frontend）。使わないモジュールは今と同じになること |
| W5 | richards：bench/richards/richards.se の `xor_bits` を `__Number_xor32__` に、`control & 1` と `control // 2` を `__Number_and32__` と `__Number_shr32__` に替える。bench/README.md に、ビット演算のプリミティブ（高速版）を使うことと、前後の時間を書く | 出力が今と同じで、CPython とも同じ。Rust、変換版の VM で測る。Python VM は変換して動かす |
| W6 | SJS：ビット演算を consh/include/sjs_bits.se（高速版のプリミティブを呼ぶ）に分け、consh/reference/sjs_bits.se には今の ConMa の作り方を残す（ConSh の高速版と参照版の分け方。FAST-PLAN-ja.md の 6）。consh/include/sjs_rt.se の `Sjs_band` などは、型の確認の後にそれを呼ぶ | consh/sjs/test のビット演算のテスト（Node.js と比べたもの）と、境目の値を足したテスト。consh/test を、速い版（高速版）と参照版で。Python VM の ConSh は、変換したモジュールで動くこと |
| W7 | 測って書く：richards、SJS のビット演算の 1 回のステップ、consh/sjs/bench と consh/bench（変わらないこと）。docs/PRIMITIVES.md に高速版のプリミティブの節を足すか、FAST-PLAN-ja.md に書くか。docs/ROADMAP.md | 表にする |

* **見込み**：richards が約 30% 速くなる（1 の試し）。SJS のビット演算が、1 回 4,500 ステップから数十ステップになる。
* **ConMa で書いた変換器**：FAST-PLAN-ja.md の F1 は、「ConMa で書いた変換器と、フロントエンドから呼ぶことは、フロントエンドが初めて `Uses` を書く段階で作る」とした。W4 がその段階に当たる。最初は bin/concise（Python）だけで作り、ConMa 版（`conma build` が簡潔版を出せるようにすること）が要るかは、W4 のときに決める（Python のない環境で、簡潔版の VM を使うことがあるか）。

## 5. 今の状態

1 の試しは、作業の木の外の worktree で行い、リポジトリには入れていない。

* **W1（作った、2026-11-12）**：include/fast/bitwise32.se に、7 つを同じ名前の簡潔版 ConMa の関数として書いた（使う関数の名前は `Bitwise32_` で始める。pure.se を include する）。ToInt32 と ToUint32 は、NaN と無限大を 0 にし、0 の方へ切り捨て、2^32 で割った余りを求める（どれも丸めずに求まる）。and、or、xor は、下のビットから、両方が 0 になるまで 1 つずつ求める。シフトは 2^k を掛けるか、割って floor する。test/step41 が、25 の値と 12 のシフトの量や右の被演算子の組（負の数、2^31 と 2^32 の前後、整数でない数、`-0`、`NaN`、無限大、1e300）で、7 つの値を Node.js の値（test/step41/test.js で作ったログ）と比べる。Python、Rust、変換版、wasm の VM で通る。
    * プリミティブとの違い：被演算子が Number でないとき、どちらも VProc をエラーで止めるが、代わりの定義のメッセージの名前は、中で使う算術のプリミティブ（`__Number_subtract__` など）になる。ConMa には、決めたメッセージで VProc を止める手段がないため。
* **W2（作った、2026-11-12）**：Rust VM（vm/src/prim/number.rs の `to_uint32`、`to_int32` と 7 つ、prim/mod.rs の表、module.rs の `INLINE_PRIMS` と `FAST_FEATURES` の `"bitwise32"`）。wasm 版は vm/src を使うので、consh/js/build.sh で作り直した。変換版の VM は、7 つを conma2rust.py の `CHAIN` と transpiled.rs の `SAFE_PRIMS` に入れ（変換したコードは、ほかのプリミティブと同じく、VM の表のプリミティブを呼ぶ。`NUM_OPS` のように Rust の変数で求めることはまだしない）、`FAST_FEATURES` に `"bitwise32"` を足した。test/fast5 は、step41 と同じ式を、代わりの定義を入れずに `(Uses "bitwise32")` を付けたモジュールで動かし、Rust、wasm、変換版の VM で step41 と同じログ（Node.js の値）になり、Python VM は、機能を知らないというエラーになる（W3 で、変換してから動かすようにした）。
* **W3（作った、2026-11-12）**：bin/concise が `"bitwise32"` を直す。include/fast/bitwise32.se を bin/includer と bin/resolver でモジュールにし（SInfo のファイル名は、リポジトリの上のディレクトリからの相対パス）、その大域の項目を名前で突き合わせて入れる：モジュールの未定義の項目（プリミティブ）は定義に替え、ほかの名前は、同じ名前の項目の番号に、なければ後ろに足した番号にする（定義の中の `(G n)` も付け替える）。モジュールが自分で定義した名前は替えない。`(F ...)` に定義のファイルを足し、`(Uses ...)` を消す。テキストのまま直すので、ほかの所は読んだとおり。test/vm.sh は、Python VM で動かすとき、`(Uses "...")` のあるモジュールを bin/concise で直してから動かす（直せない機能なら、そのまま動かして VM のエラーにする。test/fast1 は変わらない）。test/fast5 は、どの VM でも step41 と同じログになり、bin/concise で直したモジュールも、どの VM で動かしても同じになる（高速版の VM でも、定義のほうが使われる）。自分で `__Number_and32__` を定義したモジュール（own.se）は、直しても自分の定義を使う。
* **W4（作った、2026-11-12）**：フロントエンドが `(Uses "bitwise32")` を書く。Python 版（bin/resolver の `FAST_PRIMITIVES`）と ConMa 版（include/resolver.se の `__Resolver_uses__`。`__Resolver_warnings__` は `(Uses ...)` を飛ばす）は、7 つのどれかを定義のない大域の名前として使うモジュールに、`(F ...)` の後に置く。vm/frontend.module.se を bin/update-frontend で作り直し、Rust VM（`conma build` と `conma run` が使う）と consh/js/web を作り直した。test/frontend（ConMa 版のフロントエンドが自分を同じに作ること、Python 版と同じモジュールになること）は通る。test/fast5 は、フロントエンドが書いた `(Uses "bitwise32")` で動かし、`conma build` も bin/resolver と同じモジュールを作ることを確かめる。
    * ConMa で書いた変換器は作らない：簡潔版の VM は Python VM だけで、それは Python を要るので、bin/concise（Python）で足りる。
* **W5（作った、2026-11-12）**：bench/richards/richards.se のアイドルのタスクは、`control & 1`、`control // 2`、`^ 0xD008` を `__Number_and32__`、`__Number_shr32__`、`__Number_xor32__` で求める（`xor` と `xor_bits` は消した）。フロントエンドがモジュールに `(Uses "bitwise32")` を付け、Python VM は bin/concise で直して動かす。出力は今と同じで、CPython とも同じ。Rust VM で、richards 10 が 3.755 秒から 2.616 秒、richards 20 が 7.53 秒から 5.24 秒（5 回の最良。−30%）。変換版の VM は W7 で測る。bench/README.md に書いた。
* **W6（作った、2026-11-12）**：SJS のビット演算を consh/include/sjs_bits.se に分け、`Sjs_band`、`Sjs_bor`、`Sjs_bxor`、`Sjs_bnot`、`Sjs_shl`、`Sjs_shr`、`Sjs_ushr` は、型の確認（`Sjs_need2`、`Sjs_need`）の後にプリミティブを呼ぶだけにした。今までの算術の作り方（`Sjs_to_uint32`、`Sjs_bits` など）は consh/reference/sjs_bits.se にそのまま移した（ConSh の参照版と、`--include consh/reference` で作った SJS のモジュールが使う。`Uses` は付かない）。sjs_rt.se が sjs_bits.se を include する。consh/sjs/test/step8 の case9 が、20 の値と 8 のシフトの量で 7 つを Node.js と比べる（`"a" & 1` だけは、SJS の型の確認で TypeError。参照版のモジュールでも同じ出力）。SJS のループの 1 回のステップは、`x = x ^ i` が 4,573 から 68、`x = x & 1023` が 4,505 から 65、`x = x << 1` が 359 から 63（`x = x + i` は 45）。ConSh の `(( ))` は、高速版のモジュールに `(Uses "bitwise32")` が付き、Python VM でも動く（test/vm.sh が直す）。
* **W7（作った、2026-11-12）**：変換版の VM を bench/build.sh で作り直して測った。初めは、7 つを `CHAIN` のプリミティブとして呼んでいたので、richards 20 が 0.694 秒から 0.643 秒にしかならなかった（前は算術を Rust の `f64` の変数で求めていた）。そこで、conma2rust.py の `NUM_OPS` に 7 つを足し（`"bit"` と `"not"`。`number::to_int32` などで、融合した鎖の中で `f64` の変数で求める）、0.626 秒（−10%）になった。Rust VM では 7.53 秒から 5.24 秒（−30%）。出力は CPython と同じ。docs/PRIMITIVES.md に「16. Primitives of Fast ConMa」の節を、docs/ROADMAP.md に Step 41 を足し、FAST-PLAN-ja.md と bench/README.md を直した。test/ は Python、Rust、変換版の VM で通る。
    * 測ったとき、ほかのプロセスで機械の負荷が高く（load average 5 くらい）、nbody などの時間は bench/README.md の表と比べられなかった（7 つの名前を使わないので、変換したコードは変わらない）。後で（2026-11-12）、比べられなかった主な理由は、負荷ではなく機械の違いだとわかった。この段階を測った機械は Apple M2 で、bench/README.md の表は Apple M4 で測ったものだった（bench/README.md の「Now: 2026-11-12, Apple M2」）。

ビット演算のプリミティブは、これで計画のとおり作り終えた。
