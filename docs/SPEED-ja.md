# 高速化の記録：手法と効果の一覧

これまでに試した高速化の手法を、効果（時間、大きさ、ステップの数）と、入れたかどうかとともに一覧にする。詳しい計測と理由は、それぞれの「詳細」のドキュメントにある。

* 数は、**記録した時点のもの**である。測った VM、モジュール、機械の負荷が違うので、表どうしの数を比べるときは、詳細のドキュメントの条件を見る。
* 時間はふつう user + sys（3 回か 5 回のうち最速）、wasm 版は実時間（SJS の数の関数の wasm 版の表は user + sys）。機械は Apple M4 か Apple M2（SJS の数の関数の wasm 版の表だけ Apple M1）で、ドキュメントによって違う（bench/README.md の 2026-11-08 までの表、vm/README.md、consh/sjs/README-ja.md は M4。consh/bench/README-ja.md、ビット演算の W5〜W7、wasm 版の実験、bench/README.md の 2026-11-12 の表は M2）。違う機械の時間は比べず、CPython や Rust 版との比で比べる。
* 比は「後 ÷ 前」の時間（0.80 なら 2 割速い）。
* 古くなった数と、測り直しが要るものは 5 にまとめた。測り直しは、必要になったものだけ行う。

## 1. Rust VM（解釈する VM。wasm 版も同じコード）

| 手法 | 日付 | 効果 | 採否 | 詳細 |
|---|---|---|---|---|
| 小さなブロックの割り当て器（16 バイトごとの free list、`alloc.rs`） | 2026-11-05 | ConMa のフロントエンドで ConSh のモジュールを作るのが、文字列の変更と合わせて 9.9 s → 3.7 s | 入れた | vm/README.md「Speed」 |
| 実行時に作った String を、ソースの形でなく中身で持つ | 2026-11-05 | 同上（字句解析の時間の約 40% が、文字列のエスケープの変換だった） | 入れた | 同上 |
| ほかの VProc が待っていなければ、同じ VProc のステップを続けて行う | 2026-11-05 | 1 つの VProc で回る ConSh のループが約 10% 速い（`$( )` やパイプのあるループは変わらない） | 入れた | 同上 |
| OS の確かめ（`poll`、`waitpid`）を、SIGCHLD が来たときだけ行う | 2026-11-05 | `for i in $(seq 1 20000); do n=$i; done` が 1.85 s → 0.61 s、`echo $i` 2 万回が 3.27 s → 1.29 s | 入れた | 同上 |
| 大域変数の値を、コンパイルした式に持つ | — | 効果なし（時間は値の複製にかかっていた） | 入れなかった | 同上 |
| Dict を定数の文字列のキーで引く場所ごとのキャッシュ（`DictSite`）と、被演算子をその場で読む | 2026-11-11 | キャッシュだけで 1〜2%、被演算子をその場で読むのと合わせて 4〜5%（02_for_seq_100000 が 1.29 s → 1.22〜1.24 s） | 入れた | vm/README.md「Speed」、FAST-PLAN 4.5 |
| F2：引数の数がちょうど合う適用を 1 ステップで行う | 2026-11-11 | consh/bench が 0.89〜0.98 倍、bench/ が 0.89〜0.92 倍 | 入れた | FAST-PLAN 4.1 |
| F3：評価済みの LazyList を、`__LazyList_force__` の定義を通さずに返す | 2026-11-11 | F2〜F5 の合計に含まれる（下） | 入れた | FAST-PLAN 4.2 |
| F4：条件が Boolean の条件分岐で、節の Closure を作らない | 2026-11-11 | 02 が 0.93 倍、10 が 0.94 倍、07 が 0.91 倍 | 入れた | FAST-PLAN 4.3 |
| F5：値を LCont に返すだけのプリミティブ（22 個）を、LCont の Closure を作らずに行う | 2026-11-11 | 02 が 0.89 倍、10 が 0.83 倍 | 入れた | FAST-PLAN 4.4 |
| F2〜F5 の合計（F8 で測った） | 2026-11-11 | ConSh のループが 0.62〜0.82 倍、bench/ が 0.70〜0.84 倍、wasm 版が 0.68〜0.94 倍、SJS が 0.83〜0.84 倍。変換版は変わらない | — | FAST-PLAN 8.1、consh/sjs/README-ja.md |
| Number のプリミティブが、被演算子をその場で読み、OList で返す | 2026-11-12 | bench/ が 0.77〜0.92 倍。ConSh は変わらない | 入れた | vm/README.md「Speed」 |
| ConSh がよく使うプリミティブ（`__is_null__`、LList、Ref、条件分岐など）も同じにする | 2026-11-12 | consh/bench が 0.93〜0.97 倍 | 入れた | 同上 |
| 束縛の変数名を intern して、参照数の増減をなくす | 2026-11-12 | 0.98〜1.00 倍 | 戻した | FAST-PLAN 8.2 |
| ローカル環境の節から変数名を外し、名前は Body ごとの名前の列から引く（トレースと Closure の表示だけが使う） | 2026-11-13 | bench/ が 0.94〜0.99 倍、consh/bench の 02〜11 が 0.95〜0.99 倍。起動（00）は 1.04 倍。名前を別の表に持って節を捨てるたびに消す方式は 1.02〜1.06 倍 | 入れた | FAST-PLAN 8.2、vm/README.md「Speed」 |
| 引数の束縛で、ローカル環境の節を使い回す | 2026-11-12 | 0.98〜1.02 倍（使い回せたのは 35%） | 戻した | FAST-PLAN 8.2 |
| 文字列が書いてある `__Number_fromString__` を、変数の代わりに数を置いた Body にして読み込む（`load_const`。ステップの数は同じ） | 2026-11-13 | richards 20 で、使う所で数を作る `richards_const.se` が 4.196 s → 3.754 s（`richards_tuned.se` の 1.13 倍 → 1.01 倍）。`richards_tuned.se` は変わらない | 入れた（`CONMA_NO_FOLD=1` で止める） | bench/README.md「Numbers written where they are used」 |

## 2. 変換版の VM（vm/conma2rust）

| 手法 | 日付 | 効果 | 採否 | 詳細 |
|---|---|---|---|---|
| ConMa のモジュールを Rust にして VM に組み込む（変換版の VM） | 2026-11-07 | nbody 100000 が 6.59 s（Rust VM）→ 3.80 s | 入れた | bench/README.md「Results」 |
| 融合した鎖（変数を Rust の変数に置く）から、LCont 付きの呼び出しを Rust の呼び出しにするまで（6 つの変更） | 2026-11-07 | nbody 100000 が 3.80 s → 0.67 s | 入れた | bench/README.md「up to calls with an LCont」 |
| その後の 14 の変更（自分を呼ぶ関数のループ化、直接の入口、`f64` の計算、値の移動、キーのハッシュ、Dict の形とキャッシュ、小さな関数のインライン化など） | 2026-11-07〜08 | nbody 0.49 → 0.25 s、richards 20 2.10 → 0.55 s、fib 32 0.68 → 0.32 s、loop 0.65 → 0.07 s、ConSh 0.89 → 0.50〜0.53 s。変更ごとの数は詳細の表 | 入れた | bench/README.md「since then」 |
| 文字列が書いてある `__Number_fromString__` を、融合した鎖の `f64` の定数にする（`number_literal`） | 2026-11-13 | richards 20 で、`richards_const.se` が 0.807 s → 0.468 s（`richards_tuned.se` の 1.69 倍 → 0.98 倍）。ConSh などの 1,156 か所が定数になる | 入れた（`CONMA2RUST_NO_FOLD=1` で止める） | bench/README.md「Numbers written where they are used」 |
| 変換版のコードを VM と別の crate にし、関数ごとのモジュールに分けてコンパイルする（VM はライブラリにして前もってコンパイルし、変換版のコードは実行時に登録する） | 2026-11-13 | nbody のコードだけのビルドが 11.0 s → 4.8 s（16 モジュール、opt-level 3、速さは同じ）。ConSh（62 MB、128 モジュール）は opt-level 3 で 6 分 56 s、1 で 6 分 16 s（速さはほぼ同じ）、0 で 2 分 56 s（Rust VM より 1.1〜1.4 倍遅い） | 試しただけ（リポジトリは変えていない） | FAST-PLAN 8.4 |
| インライン展開の制限を緩める（段数 2 → 3、または Body の数 8 → 16） | 2026-11-12 | richards 20 が、Body 16 で 0.94〜0.95 倍（2 回とも）。ほかの bench/ と consh/bench、ConSh の 15,000 回の呼び出しは、どちらも 0.90〜1.09 倍の中でばらつき、決まった向きがない。ConSh の変換したコードは 62.8 MB が 65.1 MB（段数 3）、73.7 MB（Body 16） | 変えなかった | vm/README.md「Inlining」 |

## 3. ConMa のプログラム（ConSh、SJS）の書き方

| 手法 | 日付 | 効果 | 採否 | 詳細 |
|---|---|---|---|---|
| `__Cont_get__` をプリミティブにする（Step 34） | 2026-11-05 | `echo` 1000 回が 4,360,473 → 2,601,741 ステップ、7.4 s → 4.6 s | 入れた | docs/ROADMAP.md Step 34 |
| `__if_then_else__`、`__if__`、`__unless__` をプリミティブにする（Step 35） | 2026-11-05 | ループ 1 回が 759 → 681 ステップ | 入れた | docs/ROADMAP.md Step 35 |
| ConSh のためのデータのプリミティブ（Step 36） | 2026-11-05 | `y=$(f $i)` 3000 回が 17,550,750 → 11,988,797 ステップ、0.96 s → 0.63 s | 入れた | docs/ROADMAP.md Step 36 |
| ConSh の高速版（consh/include）：条件を先に計算する、1 段の LazyList、`Single_list_fields` など | 2026-11-08〜09 | 参照版と比べて、02 が 112.59M → 33.17M ステップ（-71%）、4.000 s → 1.341 s。ほかは -2〜-32% | 入れた | consh/include/README-ja.md |
| 呼び出しごとに 1 ステップしか減らない書き換え（よく呼ばれる関数から `__Cont_get__` を除くなど） | — | ほとんど効果なし | 入れなかった | consh/include/README-ja.md |
| richards を手で調整して書き直す（`bench/richards/richards_tuned.se`。定数を実行ごとに 1 回だけ引いて Closure の変数に持つ、スケジューラがフラグを 1 回ずつ読む、変わるフラグだけを書くなど） | 2026-11-13 | richards 2 が 20,110,425 → 15,199,166 ステップ（0.76）。richards 20 が Rust VM で 5.17 s → 3.93 s（0.76）、変換版で 0.590 s → 0.476 s（0.81）。Apple M2 | 別のファイルとして入れた（richards.se は Python の写しのまま） | bench/README.md「A tuned richards」 |
| SJS の変換器と実行時の支えの改良（型の分かる所の確認を省く、配列の添字、Ref のない `const`、数のリテラルとの比較、行の記録を減らす） | 2026-11-11〜12 | 段階 B7 と比べて、loop 100000 が 1.28 s → 0.166 s、fib 20 が 0.50 s → 0.123 s、nbody 2000 が 9.94 s → 1.098 s | 入れた | consh/sjs/README-ja.md「速さ」「経緯」 |
| SJS の数だけを扱う関数を WebAssembly にする（高速版 ConMa の `"wasm"`、wasm 版の VM のホストで動かす。段階 1〜4） | 2026-11-14 | wasm 版の VM で、loop 10000000 が 22.14 s → 0.172 s（起動を含む。Rust 版の VM の SJS は 15.77 s、手で書いた ConMa は 2.738 s）、fib 27 が 4.729 s → 0.149 s。Apple M1。配列を使う nbody は変わらない | 入れた | consh/SJS-WASM-PLAN-ja.md 16、consh/sjs/README-ja.md「速さ」 |

## 4. 言語とプリミティブ

| 手法 | 日付 | 効果 | 採否 | 詳細 |
|---|---|---|---|---|
| 平方根のプリミティブ（Step 39） | 2026-11-07 | nbody が ConMa でニュートン法を書かずに済む | 入れた | docs/ROADMAP.md Step 39 |
| 高速版 ConMa のビット演算のプリミティブ（`"bitwise32"`、W1〜W7） | 2026-11-12 | richards 20 が Rust VM で 7.53 s → 5.24 s（0.70）、変換版で 0.694 s → 0.626 s（0.90）。SJS の `^` が 1 回 4,573 → 68 ステップ | 入れた | docs/BITWISE-PLAN-ja.md |
| レコード（F6） | 2026-11-12 | Dict のキャッシュの実験から、速さの効果はほとんどない見込み | 作らないと決めた | FAST-PLAN 4.5 |
| 脱出の継続を作らずに戻る（F5 の案、段階 B8） | 2026-11-12 | `__Cont_get__` は 02 のステップの 3.2%、全部なくしても約 3%。使える関数も 637 個のうち 48 個 | 作らないと決めた | FAST-PLAN 4.4、consh/sjs/README-ja.md |
| 変換版の VM が高速版のモジュールを入力にする（F7） | 2026-11-12 | 高速版に構文がないので、渡す情報がない | 構文ができるまで作らない | FAST-PLAN 6 |
| 部分適用、完全適用、過剰適用の印をソースやモジュールにつける | 2026-11-12 | 数えると、Closure の適用の 99.7〜100% が引数の数のちょうど合う適用で、部分適用は 0 回だった。VM は整数の比較 1 回で見分けている | 作らない | FAST-PLAN 4.1 |
| 値を自分で確かめる条件分岐の組み込み（`__if_null_then_else__`、`__unless_null__`、`__Number_if_zero_then_else__`、`__Number_unless_zero__`、`__Number_if_greater__`、`__Number_if_greater_then_else__`）。確かめ、Boolean の束縛、条件分岐を 1 ステップにする | 2026-11-13 | `richards_tuned.se` のコピーで、richards 2 が 15,199,166 → 13,542,821（null の 11 か所）、14,995,439（zero の 5 か所）、15,198,686（`__Number_if_greater__` の 1 か所、初期化だけ）、14,407,034（`__Number_if_greater_then_else__` の 5 か所）、12,546,962（`__Number_if_greater__` 以外の 21 か所）ステップ。richards 20 が Rust VM で 0.903、0.987、0.998、0.955、0.840 倍 | Rust VM と wasm 版に入れた。2026-11-14 から高速版の機能 `"control"` の組み込みで、フロントエンドが `(Uses "control")` を書き、Python VM は `include/ref/control.se` の定義で動かす（変換版のコードは VM に任せる。test/fast10） | vm/README.md「Speed」、コミット 1e4c864 |
| ローカル環境を配列のフレームにする | 2026-11-12 | 変数をたどる深さは平均 2〜3 と浅く、読む速さは上がらない。節を作って捨てる手間は、関数ごとのフレームとエスケープ解析なら減らせる見込み（未検討） | 作っていない | vm/README.md「Speed」、FAST-PLAN 8.2 |
| Closure を作らず、LCont で 8 個以上の変数を束縛する鎖だけ、その変数を LVEnv の 1 つの節（フレーム）に置き、スロットで読む（モジュールを読み込むときに選ぶ。`--trace` と変換版のコードがあるモジュールでは使わない） | 2026-11-13 | nbody（`pair_step` が 43 個、読む深さの平均 7.45）が 0.87〜0.89 倍。ほかの bench/ と consh/bench は 0.98〜1.02 倍。閾値を 0 や 1 にすると loop と consh/bench の一部が 1.03〜1.04 倍、8 と 16 はフレームなしと同じ | 入れた（`CONMA_FRAME_MIN=n`、空で止める） | vm/README.md「Speed」、FAST-PLAN 8.2 |

## 5. wasm 版

| 手法 | 日付 | 効果 | 採否 | 詳細 |
|---|---|---|---|---|
| ConSh の変換版のコードを wasm 版に入れる | 2026-11-12 | ループが 0.73〜0.83 倍。wasm が 0.83 MB → 78 MB（gzip で 0.22 MB → 12.5 MB）、起動が +0.09 s で、短いスクリプトは 1.10〜1.17 倍に遅くなる | 入れなかった | FAST-PLAN 8.3 |

## 6. 大きさ

2026-11-12 のもの。

| もの | 大きさ | 備考 |
|---|---|---|
| Rust VM（vm/target/release/conma） | 1.9 MB | |
| 変換版の VM（vm/target-transpiled/release/conma） | 41 MB | 2026-11-13 に作り直したもの（`CONMA_BENCH_REFERENCE=1 bench/build.sh`、34 分。変換した Rust は 86.8 MB）。richards は richards.se と richards_tuned.se を含む（richards_const.se は、その後に bench/ に入れた）。2026-11-12 に同じように作ったものは 190 MB（62 分）で、その間に 36a8036（制御の組み込み）と数の定数化が入ったが、どちらでどれだけ小さくなったかは調べていない。bench/ の 4 つ、test/transpiled、ConSh の高速版と参照版の変換版のコードを含む。前に作ったもの（どちらの版を含めたかは記録がない）は約 100 MB。bench/ と test/transpiled だけなら 8 MB |
| wasm 版（consh/js/web/conma.wasm） | 0.83 MB（gzip で 0.22 MB） | |
| wasm 版に ConSh の変換版のコードを入れたもの | 78 MB（gzip で 12.5 MB） | 実験（FAST-PLAN 8.3） |
| ConSh の高速版のモジュール（bin/includer と bin/resolver、1 行の形） | 5.1 MB | F8 では 5,011,489 バイト、参照版は 4,948,125 バイト |
| ブラウザの ConSh のモジュール（consh/js/web/consh.module.se） | 16.6 MB | 中身は上と同じで、字下げのある形 |
| ConSh のモジュールを変換した Rust | 63 MB | conma2rust.py の出力 |
| SJS のモジュール | 1.0〜1.5 MB | 実行時の支え（sjs_rt.se、約 1 MB）を含む。SJS 1 行が ConMa の約 200 バイト |

## 7. 古くなった数と、測り直しが要るもの

2026-11-12 に、変換版の VM を作り直し（ConSh の高速版と参照版を含む）、次を測り直した（Apple M2、3243b7d）。

* **bench/README.md**：「Now: 2026-11-12, Apple M2」の表を足した。変換版の時間は、CPython 3.9.6 の 0.19〜0.76 倍（nbody 0.42、richards 0.41、fib 0.76、loop 0.19）。2026-11-08 までの表は Apple M4 のもので、そのまま残した。W7 で nbody などを表と比べられなかったのは、負荷より、機械の違いのためだった。
* **consh/bench/README-ja.md の「結果」の表**：測り直して置き換えた。Rust 版の VM は、2026-11-09 の表より速くなった（高速版の 02 が 1.818 → 1.106 秒、参照版が 5.434 → 3.006 秒）。起動は 0.025 → 0.05 秒に遅くなった（理由は調べていない）。
* **変換版の VM の ConSh**：W6 の後のモジュールで作り直したので、変換したコードが使われる。test/ と consh/test（高速版、参照版）は、変換版の VM、Rust VM で通った。

まだ合わないもの：

* **大きさの記述**：consh/js/PLAN-ja.md の「約 7.0 MB」は、計画を書いたときの数で、今の大きさと合わない（consh/js/README-ja.md の「wasm 約 700 KB」は、約 0.8 MB に直した）。
* **CPython**：`bench/run.sh` は PATH の最初の `python3` を使う。この機械では Homebrew の 3.15.2 で、表の 3.9.6（/usr/bin/python3）より 2.2〜2.6 倍速い。

## 8. 記録の場所

| ドキュメント | 記録しているもの |
|---|---|
| bench/README.md | bench/ の 4 つのプログラムと CPython の比較、変換版の VM の変更ごとの効果 |
| docs/FAST-PLAN-ja.md | Rust VM の F1〜F8、時間の内訳と戻した実験（8.2）、wasm 版の実験（8.3） |
| docs/BITWISE-PLAN-ja.md | ビット演算のプリミティブ |
| vm/README.md「Speed」 | Rust VM の割り当て器、文字列、スケジューラ、OS の確かめ、Dict のキャッシュ、プリミティブの読み方 |
| docs/ROADMAP.md | プリミティブにしたもの（Step 34〜36、39、41）のステップの数 |
| consh/include/README-ja.md | ConSh の高速版と参照版のステップの数と時間 |
| consh/bench/README-ja.md | ConSh のスクリプトの、版と VM の組み合わせごとの時間 |
| consh/sjs/README-ja.md | SJS の時間、ステップの数、大きさ |
