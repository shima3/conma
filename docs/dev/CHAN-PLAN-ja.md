# チャネルのプリミティブの計画

ConSh のチャネル（consh/include/consh_chan.se。関数どうしのパイプと `$( )` で、いくつもの VProc が書き、1 つの LazyList として読むもの）を、拡張 ConMa のプリミティブにする計画である（2026-11-16）。docs/dev/BITWISE-PLAN-ja.md と同じく、拡張 ConMa の機能（docs/PRIMITIVES.md の 16）を 1 つ足す形にする。

## 1. 目的と、測ったこと

* **ConSh のパイプが遅い**：consh/bench の Bash 版との比べ（consh/bench/README-ja.md の 2026-11-16）で、wasm 版の VM の最適化版は、`10_pipe_for_stdin` が Bash の 21 倍、`18_pipe_funcs` が 12 倍の時間だった。Rust 版の ConSh（consh-rs）は、チャネルを Rust（consh/rust/src/chan.rs）で作っていて、それぞれ 3.5 倍と 2.1 倍。
* **ステップの内訳**（Rust 版の VM の `CONMA_PROFILE`、最適化版、2026-11-16）：

| スクリプト | 全ステップ | consh_chan.se | `Subst_settle*`（`$( )` の状態） | util.se（`__LazyList_force__` など） |
|---|---|---|---|---|
| `10_pipe_for_stdin`（10 万要素） | 6,770 万 | 37.1% | 約 18% | 17.2% |
| `18_pipe_funcs`（5 万要素を 2 段） | 1 億 5,600 万 | 34.8% | 約 6% | 9.8% |
| `02_for_seq_100000`（チャネルなし、比べるため） | 3,320 万 | 0% | 約 34% | 26.1% |

* `10` で要素 1 つを渡すのに、`Chan_wait_room`（45 ステップ）を 2 回、`Chan_take`（62）、`Chan_put`（35）、`Chan_next`（29）を 1 回ずつ通る。中身のほとんどは、書き手と読み位置の Dict の読み書き（`"consumed"`、`"cap"`、`"tail"` など）と Ref の操作で、プリミティブにすれば、それぞれ 1 歩になる。
* **見込み**：`10` と `18` のステップの約 35% がなくなり、約 1.5 倍速くなる。変換した VM では、今は待つたびに VM に戻っている（vm/README.md の Counts）ので、もう少し効くかもしれない。チャネルを使わないスクリプトは変わらない。要素 1 つあたりの残り（`02` で約 330 ステップ：`$( )` の状態、for の 1 回、`__LazyList_force__`）は減らないので、Rust 版の ConSh や Bash には届かない。

## 2. 方針

* **拡張 ConMa に足す**：コア ConMa（docs/SPEC.md、docs/PRIMITIVES.md の 1〜15）は変えない。機能 `"chan"` の代わりの定義（include/core/chan.se）を、今の consh/reference/consh_chan.se を ConSh によらない形にしたコア ConMa の関数として書く。Python 版の VM は、bin/to-core がそれを入れたモジュールを動かす。
* **組み込みは Rust で 1 つ作る**：vm/src/prim に作れば、wasm 版の VM（vm/wasm/src/lib.rs は vm/src の prim、sched.rs、transpiled.rs を `#[path]` で取り込む）にも入り、変換した VM の生成コードからも呼べる。
* **待つプリミティブ**：書き手が一杯で待つとき、読み手が要素を待つときは、`__LazyList_claim__` が他の VProc の評価を待つのと同じく、Operator と OList を残したまま VProc を待たせ、起きたらプリミティブをやり直す（vm/src/prim/lazy.rs の `claim`、sched.rs の `wait_lazy`、`wait_change`）。
* **ConSh の参照版はコア ConMa のまま**：ConSh の参照版はコア ConMa で作る（include/core を使う）ので、代わりの定義で動く。最適化版はプリミティブで動く。consh/test を両方で流せば、プリミティブと代わりの定義を比べることになる。
* **ConSh 専用の部分は ConSh に残す**：ConSh の書き手の行き先（`Target_make "chan"`）、`Pipe_cap`、`Sh_abort` は ConSh のまま。

## 3. 形

今の consh_chan.se の関数を、そのままプリミティブにする形である（4 で決めた）。チャネル、書き手、読み位置は、今と同じ Dict と Ref で、プリミティブもそれを読み書きする（4 の 3）。

| プリミティブ | 今の関数 | 待つか |
|---|---|---|
| `__Chan_new__ cap state ,(chan)` | `Chan_new`（と `$( )` の `__Dict_put__ chan "state" state`） | 待たない |
| `__Chan_writer__ chan ,(writer)` | `Chan_writer` | 待たない |
| `__Chan_of__ writer ,(chan)` | `Chan_of` | 待たない |
| `__Chan_put__ onCancelled writer item ,()` | `Chan_put`（`Chan_wait_room` を含む） | 一杯なら待つ |
| `__Chan_close__ writer ,()` | `Chan_close` | 待たない |
| `__Chan_cancel__ chan ,()` | `Chan_cancel` | 待たない |
| `__Chan_reader__ chan ,(reader)` | `Chan_list` の読み位置を作るところ | 待たない |
| `__Chan_take__ reader ,(item found)` | `Chan_next` の、取り出して待つところ（`Chan_take_from`、`Chan_register`、`Chan_take`） | 要素がなく、開いた鎖があれば待つ |

* `Chan_list` が返す LazyList は、`__LazyList_new__ (,() __Chan_take__ reader ,(item found) ...)` の形で、ConSh に残す。LazyList の節は 1 つずつ評価されるので、読み位置を同時に 2 つの VProc が変えることはない（今と同じ）。
* **順番の規則は今と同じにする**：1 つの鎖の中は書いた順。鎖のあいだは、取り出した鎖を末尾に回し、新しい鎖は、知っている鎖に要素がないときと 16 個（`Chan_register_every`）ごとに探す。consumed を cancelled より先に読む（consh_chan.se の `Chan_wait_room` の注釈）。チャネルを閉じたと決めるのは、新しい鎖を探した後だけ。
* **`$( )` の状態**：`__Chan_new__` の `state` が Ref なら、書き手が待つ間は `"blocked"`、起きたら `"running"` にする（`Subst_settle` がそれを待つ）。null なら使わない。
* **待つ**：プリミティブは、待つとき Operator と OList を残して、Ref（`__Chan_put__` は鎖の consumed、`__Chan_take__` はチャネルの wake）が変わるまで VProc を待たせ、起きたらやり直す。`__Chan_put__` は、待った書き手の Dict に印を付けておき、やり直したときに `state` を `"running"` にする。
* **エラー**：チャネル、書き手、読み位置でない値は、VProc を止める（代わりの定義では、中で使う `__Dict_lookup_or__` などのエラーになる。BITWISE-PLAN-ja.md の W1 と同じく、名前が違う）。読み手が cancel した後の `__Chan_put__` は、書かずに `onCancelled` を呼ぶ（今と同じ。SIGPIPE に当たる）。

## 4. 決めたこと（2026-11-16、ユーザーと）

1. **形**：3 の形（今の関数をそのままプリミティブにする）。ConSh の書き換えが小さく、代わりの定義を今の consh/reference/consh_chan.se から作れる。
2. **`$( )` の状態**：`__Chan_new__` に状態の Ref を渡し、プリミティブが `"blocked"` と `"running"` にする（もう 1 つの案は、`__Chan_put__` が一杯なら待たずに `"full"` を返し、待つのと状態は ConSh に残すものだった。プリミティブは汎用になるが、待つところが ConMa に残り、そのぶん遅い）。
3. **値の型**：決めたときは、拡張 ConMa では `"Chan"` などの新しい型にする案だった。作るときに、プリミティブも代わりの定義と同じ Dict と Ref を読み書きすることにした（2026-11-16）。型は VM によらず `"Dict"` で、GC（vm/src/gc.rs）とトレースの表示に新しい型を足さずに済み、待つのも `__Ref_wait_change__` と同じ仕組みでできる。Dict を引く手間はあるが、Rust の中なので、ConMa のステップよりずっと小さい。
4. **名前**：`__Chan_*__` と機能の名前 `"chan"`。

## 5. 作り方（段階）

各段階は、そこまでのテストが通り、test/ と consh/test が、その段階に関わる VM で通ることで終わりとする（全部を流すのは、別の担当者がする。docs/dev/TESTS-ja.md）。

| 段階 | 内容 | 確かめ方 |
|---|---|---|
| C1 | 代わりの定義：include/core/chan.se に、3 のプリミティブを同じ名前のコア ConMa の関数として書く（consh/reference/consh_chan.se から、ConSh によらない部分）。 | test/ に新しい step：書き手 1 つと 3 つ、cap を超える書き込み、閉じる、cancel、後から加わる書き手、`$( )` の状態の Ref。Python 版と Rust 版の VM で同じログ |
| C2 | Rust 版の VM のプリミティブ（vm/src/prim の新しいファイル、prim/mod.rs の表、value.rs の型、module.rs の `EXT_FEATURES` に `"chan"`）。待つのは sched.rs の仕組みで。wasm 版を作り直す | C1 のテストを `(Uses "chan")` のモジュールで、Rust 版と wasm 版の VM で。代わりの定義と同じ出力 |
| C3 | bin/to-core が `"chan"` を代わりの定義に直す。フロントエンド（bin/resolver と include/resolver.se）が `(Uses "chan")` を書く。vm/frontend.module.se を bin/update-frontend で作り直す | test/frontend、test/fast の新しいケース（BITWISE-PLAN-ja.md の W3、W4 と同じ） |
| C4 | 変換した VM：待たないプリミティブを conma2rust.py の `CHAIN` に。`__Chan_put__` と `__Chan_take__` は、待たずに済むときだけその場で求め、待つときは VM に戻す（`__LazyList_claim__` の `d_lazylist_claim_done` と同じ） | test/transpiled、C1 のテストを変換した VM で |
| C5 | ConSh：consh/include/consh_chan.se をプリミティブを呼ぶ形にする（`Chan_list` の LazyList だけ残す）。参照版は consh/reference/consh_chan.se を、include/core/chan.se を使う形にする | consh/test を最適化版と参照版で、Rust 版、変換した、wasm 版の VM で。`CONSH_JITTER` のテスト。Python 版の VM の ConSh |
| C6 | 測って書く：consh/bench の `10`、`18`（ほかは変わらないこと）を Rust 版、変換した、wasm 版の VM で。docs/PRIMITIVES.md の 16 に機能 `"chan"`、docs/dev/ROADMAP.md に Step、consh/bench/README-ja.md | 表にする |

## 6. 今の状態

計画を書き、4 を決めた（2026-11-16）。
