# ConMa のレビュー

ConSh のレビューは consh/REVIEW-ja.md にある。このファイルは、ConMa 本体（言語、VM、フロントエンド、テスト）の設計と実用性についてのレビューである。

## 2026-11-06: 設計と、中間言語としての実用性

対象は f9625cd（ConSh の段階 20）である。目的は、独自言語 ConMa の実用性の検証である。

### 確かめた方法

確かめた環境は Linux（Ubuntu 24.04）、Python 3.12.3、rustc 1.75（apt）で、作者の環境（macOS、Apple M4）とは違う。そのため、絶対値ではなく比を見る。

* ドキュメント（DESIGN.md、SPEC.md、SELFHOST.md、ROADMAP.md の概要、vm/README.md）と、include/ と consh/include/ の代表的なコードを読んだ。すべてのファイルは読んでいない。
* Rust 版 VM（vm/）をビルドし、`CONMA_VM=rust` で test/ の step1〜10、12、17、18、20、selfhost4、selfhost7、module1、module2 を実行した。step3、10、12、17、20 は、Python 版でも実行した。
* 速度と、エラーの出方を調べる小さなプログラムを書いて実行した（次の「実測」と 1〜4）。

### 実測

| 項目 | Rust VM | Python VM | CPython（同じ処理） |
|---|---|---|---|
| 100万回のループ（`n` を 0 まで減らし、`acc` を増やす） | 1.4 秒（13.0M ステップ。13 ステップ/回、約 108 ns/ステップ） | 30.5 秒 | 0.037 秒 |
| fib(22)（57,313 回の呼び出し） | 0.12 秒（1.06M ステップ。18.5 ステップ/回） | 2.6 秒 | 約 0.002 秒 |
| selfhost7（ConMa 版フロントエンドで ConSh のモジュールを作る） | 3.7 秒 | 未測定 | |

* Rust 版は Python 版の約 22 倍速い（ループ）。
* Rust 版は、素の CPython に比べて、このループで約 38 倍、fib で数十倍遅い。
* 結果の値は正しかった（ループ 1000000、fib(22) 17711）。

測ったプログラムは、この文書の末尾に付けた。

### 1. 実行時エラーにソース位置が出ない（中）

VProc がエラーで終わると、次のように出る。

```
Error in VProc 1 at step 1: __Number_add__: expected Number, got String
```

* VM は位置を持っている。`--trace` の出力には、`(SInfo (22 3) "/tmp/b/err.se" "22" "3")` があり、includer が各 Body に自動で入れている。エラーのメッセージに使っていないだけである。
* 出るのは「VProc の番号」と「全 VProc を通したステップ数」で、プログラムの利用者が見て分かる情報ではない。
* 修正案：VProc の現在の SInfo（ファイル名、行、列）を、`Error in VProc N at step S: message` に続けて出す。SInfo が空のときは今のままにする。Python 版と Rust 版の両方を直し、期待ログを更新する。

### 2. 引数の個数の間違いが、遠くで、誤解を招くメッセージで見つかる（中）

ConMa は、関数がすべてカリー化された CPS の言語で、引数が足りないときは部分適用、多いときは余った引数を戻り値に適用する。このため、引数の個数の間違いはエラーにならず、後で別の形で現れる。次の実験をした（`add2` は `,(a b) __Number_add__ a b`）。

| 呼び出し | 結果 |
|---|---|
| `add2 one ,(r) __Number_toString__ r ,(s) …`（足りない） | `__Number_toString__: expected Number, got Closure`（step 11） |
| `add2 one one one ,(r) __Number_toString__ r ,(s) …`（多い） | `Primitive not implemented: (__Number__ 2)`（step 12） |
| `__Number_add__ "x" "y" ,(r) …`（型が違う） | `__Number_add__: expected Number, got String`（step 1） |

* 1 つ目は、`add2` の呼び出しではなく、その後の `__Number_toString__` で見つかる。位置が出ないので（1）、`add2` の呼び出しが原因だとは分からない。
* 2 つ目のメッセージは、実際には「関数でない値（Number）を呼んだ」ことを言っている。「プリミティブが実装されていない」は誤解を招く。
* これは、DESIGN.md に書かれた意味（部分適用、余った引数）の帰結で、実装の誤りではない。ただし、手でコードを書く人には、非常に見つけにくい誤りになる。
* 修正案：
    * 「関数でない値を呼ぶ」エラーを別のメッセージにする（4 の修正と合わせる）。
    * リゾルバで、呼び出し先が大域の `define` で、Head の長さが分かり、引数の個数が決まっているときだけ、個数の不一致を警告する（部分適用を意図した呼び出しを除くため、警告にとどめる）。

### 3. VProc がエラーで終わっても、終了コードが 0 である（中）

DESIGN.md の「Scheduling and Termination」に、「code 0 when every VProc has finished, whether or not some of them finished with an error」とある。実際に、1 と 2 の実験は、すべて `exit 0` で終わった。

* ConSh のように、スクリプトやほかのプログラムから呼ばれる使い方では、失敗を終了コードで知れない。
* 他の VProc に影響しないことは、意図した設計である。そのうえで、「最初の VProc（`__main__` など）がエラーで終わったときは、終了コード 1」にすることを提案する。`__OS_exit_error__` と同じ意味になる。
* 他の VProc のエラーまで終了コードにするかは、設計の判断である。

### 4. 「関数でない値を呼ぶ」エラーが、2 つの VM で違う（中）

2 の `too_many` で、2 つの VM の出力が違った（`test/vm_compare.sh` の出力）。

```
Python: operator: Number(2.0)
        Error in VProc 1 at step 12: Primitive not implemented: Number(2.0)
Rust:   Error in VProc 1 at step 12: Primitive not implemented: (__Number__ 2)
```

* Python 版（test/vproc_step.py の 661〜663 行）は、`{operator!r}` で Python の `repr` を出す。さらに、標準出力に `operator: …` というデバッグの行を出している。
* Rust 版（vm/src/step.rs の 148〜151 行）は、値を S 式にして出す。
* vm/README.md は「同じ出力、トレース、エラーメッセージ」を目標にしている。この差は、異常系のテストが少なく、見つからなかったものだと思う。
* 修正案：Python 版の `print` を消し、2 つの VM のメッセージを S 式の形にそろえる。「関数でない値を呼んだ」ことを言うメッセージ（2 のとおり）にして、test/step2 などに異常系のテストを足す。

### 5. 中間言語としての検証が、まだない（高。方針の問題）

SPEC.md は、ConMa を「より初心者向けの言語から変換して実行する中間言語」と位置づけている。しかし、リポジトリには、高級言語から ConMa へ変換する処理が見当たらなかった。

* ConSh（7,670 行、consh/include）は、ConMa で手で書かれている。これは、「ConMa は、継続や並行性を使う実用的なプログラムを書ける」ことの証拠にはなる。段階 20 では `reset` と `shift` まで実現していて、ConMa の継続の機能が使われている。
* 「中間言語として使える」ことの証拠にはならない。変換された ConMa のコードは、手で書いたものと形が違う（大きさ、引数の個数、継続の使い方、エラー位置の対応づけ）。
* 提案：小さな言語（ConSh の部分集合、または ML 風の小さな言語）を 1 つ選び、ConMa に変換する処理を作る。そこで、生成されたコードの大きさ、性能（7）、エラー位置の対応づけ（1）を測る。1〜4 は、変換器を作ると最初に問題になる点である。

### 6. レコード（フィールドを名前で持つ値）がない（中）

ConSh の実行文脈 `sh` が、文字列のキーを約 26 個持つ Dict であること（consh/REVIEW-ja.md の 1）は、ConMa にレコードがないことの帰結である。

* Dict のキーの綴りの誤りは、実行時まで分からない。
* リゾルバは、変数を de Bruijn 指標（`(L index)`、`(G index)`）にしている。同じように、フィールド名をスロット番号に解決するレコードを足すと、綴りの誤りを静的に見つけられ、Dict の探索も要らなくなる。
* IR を最小にしたいという方針とは、緊張がある。レコードを言語に足さずに、変換器側（5）が Dict への変換と、アクセサ関数の生成を引き受ける、という選び方もある。どちらにするかを、決めておくとよい。

### 7. 性能の構造（低）

「実測」のとおり、単純なループでも 1 回に 13 ステップかかる。

* `if` の枝は `(,() …)` というクロージャで渡すため、実行のたびに枝のクロージャが作られる（vm/README.md が言う「thunk」。`vm_profile.py --thunks` はこの調査のためにある）。Step 35 で `__if_then_else__` を組み込みにしたのも、同じ理由だと思う。
* ステップあたり約 108 ns で、時間は「ステップ」「スケジューラ」「値の複製と破棄」に分散している（vm/README.md のとおり）。
* 変換器（5）を作る場合、変換後のコードがどのくらいのステップになるかが、実用の目安になる。
* 検討できる方向（検証していない）：`if` の枝のように、その場で 1 回だけ呼ぶクロージャを、リゾルバが印を付けて、VM が作らずに実行する。よく使う組み込みの並びを、1 ステップにまとめる。

### 8. 実装が多重で、配布の形が決まっていない（中）

| 部品 | 実装 |
|---|---|
| フロントエンド（lexer、comment_remover、parser、includer、resolver） | Python（bin/、1,395 行）、ConMa（include/、2,387 行） |
| VM | Python（test/vproc_step.py、2,833 行）、Rust（vm/src、5,387 行） |
| ConSh | ConMa（7,670 行）、Rust（5,639 行）。構文解析は、さらに Python（consh_parse、1,602 行）がある |

* 2 つの VM が、ステップごとのトレースまで一致することを確かめる方針（vm/README.md の 129 回の実行）は、強い。4 のような不一致は、この方針のおかげで見つかった。
* 一方で、どれを「基準」、どれを「製品」とするかが、書かれていない。`__VProc_spawn__` は、外部コマンドの `bin/includer` と `bin/resolver`（Python）を起動する（DESIGN.md）ので、Rust 版の VM だけでは、プログラムを読み込めない。
* 提案：ConMa 版のフロントエンドを `__VProc_spawn__` から使えるようにして、Rust 版の VM だけで動く構成を作る（selfhost7 で、同じモジュールができることは確かめられている）。

### 9. スケジューラの決定性は、ステップ数に頼っている（低）

* スケジューラはラウンドロビンで、1 回に 1 ステップ進める。OS の確認は 64 ステップごと（`POLL_INTERVAL`）なので、OS を待つ VProc があるトレースは、実行のたびに違いうる（DESIGN.md に書いてある）。
* ブロックする入出力（`__OS_get_String__` など）は、すべての VProc を止める（同じく DESIGN.md）。
* consh/REVIEW-ja.md の 2026-11-04 の 2 のように、ステップ数の偶然に頼って正しく動いていた不具合が、すでに見つかっている。
* Rust 版 ConSh には、スレッドが出会う所で乱数で止まる `CONSH_JITTER` と、それを種を変えて繰り返す `consh/test/stress.sh` がある。ConMa の VM（Python 版、Rust 版）には、スケジュールを乱す仕組み（種を指定して、実行する VProc の順を変えるなど）が見当たらない（`jitter`、`seed`、`shuffle`、`random` を検索して、見つからなかった）。
* 提案：VM に、種を指定して VProc を選ぶ順を乱すオプションを足し、step10、17、20（VProc、LazyList）と ConSh のテストを、種を変えて繰り返す。ConSh の Rust 版で効果があった方法を、ConMa の VM に持ってくる形になる。

### 10. テストの衛生（中）

* 各 `test.sh` は、`../resolver.sh test.se` で `/tmp/test.module.se` を作り、各ケースのログを `/tmp/$c.log` に書く（test/step3/test.sh など）。どのステップも同じ名前を使うので、2 つのテストを同時に実行すると、互いに壊しあう。
* `test/resolver.sh` は、カレントディレクトリにあるファイル名を前提にしている。パスを付けて呼ぶと、`/tmp//tmp/b/bench.ast.se: No such file or directory` になり、`BrokenPipeError` の長い出力が出る。
* 期待ログが macOS の出力で作られている（consh/REVIEW-ja.md の 5）。
* 修正案：`mktemp -d` で作業ディレクトリを作り、`trap` で消す（`test/vm_compare.sh` はすでにそうしている）。`resolver.sh` は、`base=$(basename ...)` を使う。

### 11. ビルド条件と README（低）

* `Cargo.lock` が version 4 で、Cargo 1.78 以降が要る。どちらの `Cargo.toml` にも `rust-version` がない（consh/REVIEW-ja.md の 4）。`vm/` は依存クレートがないので、ロックファイルを消すとビルドできる（rustc 1.75 で 19 秒、警告なし）。
* README.md が 2 行で、docs/index.md と vm/README.md、consh/README-ja.md へ誘導していない。新しく見る人の入口がない。ドキュメント自体（DESIGN.md、SPEC.md、PRIMITIVES.md など）は、詳しく、よく書かれている。

### 問題なしと確かめたところ

* Rust 版 VM は、確かめたテスト（上の「確かめた方法」）をすべて通る。`--trace` を含むステップ単位の一致は、自分で書いた 4 つのプログラム（1〜4 の実験）のうち 3 つで確かめた。差があったのは 4 だけである。
* Rust 版 VM の `unsafe` は、アロケータ（`alloc.rs`）、C 関数の宣言（`sys.rs`）、プロセス（`prim/process.rs`）に集まっている。依存クレートはない。
* ConMa 版フロントエンド（selfhost7）は、ConSh のモジュールを、Python 版と同じ形で作る。
* 継続の機能（`__Cont_get__`、`CChain`）は、`return`、`break`、`continue`、`reset` と `shift` に実際に使われている。
* ドキュメントが VM の状態の遷移（Operator、OList、LCont、CChain）まで書いていて、2 つの VM を同じ仕様で作れる。

### 未確認

* メモリ使用量（`/usr/bin/time` が使えなかった）、サイクル回収（`gc.rs`）の正しさ、長時間の実行。
* test/ のすべてのステップ（step21〜36）と、`consh/test/stress.sh`。
* 変換器を作った場合の、実際のステップ数（5）。

### 対応

指摘のあと、11 の 1 つ目（`Cargo.lock` と `rust-version`）を直した（consh/REVIEW-ja.md の 4 と同じ。`vm/Cargo.toml` に `rust-version = "1.75"` を書き、1.75 でビルドできることを確かめ、`Cargo.lock` を version 3 にした）。

* 3：提案のとおり、最初の VProc（入口の関数を実行するもの、番号 1）がエラーで終わったときは、すべての VProc が終わった後の終了コードを 1 にした。ほかの VProc のエラーは終了コードを変えない（ConSh の `$( )` やパイプラインの要素のように、ほかの VProc のエラーを意図して受け流す使い方があるので）。Python 版（`vproc_step.py` の `Scheduler.main_failed`、`vproc_run.py`）と Rust 版（`sched.rs`、`main.rs`）の両方を直し、DESIGN.md の「Scheduling and Termination」を書き換えた。test/ で、VProc 1 がエラーで終わる 14 ケースの期待ログの `exit: 0` を `exit: 1` にした。差はそれだけで、step4、5、6、8、10、12、19、20、22、24 が両方の VM で通る。consh/test には、VProc 1 がエラーで終わるケースはない。
* 4：「関数でない値を呼ぶ」エラーを、2 つの VM で `Not a function: 値` にそろえた。値はトレースと同じ S 式の形で書く（`Not a function: (__Number__ 2)`）。Python 版の `operator: …` を出す `print` は消した。モジュールを読むときの、実装されていないプリミティブ（`PrimFunc`）のエラーは、今の `Primitive not implemented` のままである。
* 1：VProc がエラーで終わったときのメッセージに、その時の VProc の SInfo の位置を `ファイル名:行:列` で付けた（`Error in VProc 1 at step 12: conma/test/step2/test.se:71:3: Not a function: (__Number__ 2)`）。位置は、呼び出しに失敗した Body の位置（プリミティブのエラーなら、それを呼んだ Body の位置）である。SInfo がなければ、今のとおり位置を付けない。これまで一部のエラー（`Null Operator`、`SeqClosure`、`Partial apply`、`__Cont_get__`）だけが自分で `ファイル名:行` を付けていたが、それをやめ、スケジューラがすべてのエラーに付けるようにした（`vproc_step.py` を 1 ステップだけ実行するときの `Error: …` も同じ）。Python 版（`vproc_step.py`）と Rust 版（`step.rs`、`sched.rs`、`prim/cchain.rs`）の両方を直し、DESIGN.md の「Scheduling and Termination」と VPROC_STEP.md に書いた。
    * 期待ログは、エラーの行に位置が付くだけ変わった（step4、5、6、8、10、12、17、19、20、22、24 と module1、module2）。
    * test/step2 に、異常系の case5〜8（型の違い、引数が多い、引数が足りない、文字列を呼ぶ）を足した。2 つの VM の出力とトレースは、`vm_compare.sh` で一致する。
    * 2 の 1 つ目（引数が足りないときに、原因と違う所で見つかる）は、位置が出るようになったが、出る位置はエラーが見つかった所（`__Number_toString__` を呼んだ所）で、`add2` を呼んだ所ではない。これを見つけやすくするのは、2 の 2 つ目（リゾルバの警告）である。
* 11 の 2 つ目：README.md に、ConMa の特徴と、docs/index.md、vm/README.md、consh/README-ja.md への案内を書いた。
* 10：test/ のテストが、`/tmp` の決まった名前を使わないようにした。
    * test/common.sh を足した。各 `test.sh` はこれを source し、テストのディレクトリに移って、`mktemp -d` で一時ディレクトリ `$tmp` を作る（`trap` で消す）。`resolve test.se` で `$tmp/test.module.se` を作り、ケースのログも `$tmp` に書く。45 の `test.sh` をすべてこの形にした。
    * `resolver.sh` は、`basename` でファイル名を取り、2 つ目の引数で出力先のディレクトリを選べる（既定は今のとおり `/tmp`）。パスを付けて呼べる。includer が失敗したときも止まる（`pipefail`）。
    * `vproc_step.sh`（module1、module2）は、作業ファイルを `mktemp -d` のディレクトリに置き、ログは標準出力にだけ書く（これまでは `/tmp/caseN.vproc.log` にも書いていた）。
    * 出力に `$tmp` のパスが入る所（step1 のトレースのモジュール名、selfhost2 と selfhost5 のファイル名）は `<tmp>` と書く。期待ログはその分だけ変えた。
    * selfhost6 の期待ログに、別の場所の絶対パス（`/Users/shima/tmp/conma`）が入っていて、どちらの VM でも通らなかった。テストのディレクトリの外にあるリポジトリのパスを `<root>` と書くようにして、直した。
    * すべてのテスト（`test/run_all.sh` を足した）が、Python 版と Rust 版の両方で通る。Python 版の全体を実行している間に、Rust 版で別のステップを同時に実行しても、互いに壊さないことを確かめた。
    * 同じテストを同時に 2 つ実行することもできるようにした。step14、16、23、24 は、common.sh の `rundir` で、テストのディレクトリのファイルを `$tmp/run` に写し、そこで `work/` を作って実行する（プログラムが読む `../test.se` や `input.txt` もそこにある。`<dir>` はそのディレクトリになる）。selfhost6 の `link.se`（`a.se` へのシンボリックリンク）は、いつも同じなので、リポジトリに入れ、selfhost6 と selfhost7 は作ったり消したりしない。selfhost6 の `work/abs*.se`（中身が絶対パス）は `$tmp/work` に作り、ログでは `<tmp>` と書く（case3 と case4 の期待ログはその分だけ変わった）。step33 は、すでに `STEP33_DIR` で `$tmp` に書いている。consh/test も、その後、ログを一時ディレクトリに書くようになっている。変えた 7 つのテストを、それぞれ 2 つ同時に実行して通ることと、test/ の全テストが両方の VM で通ることを確かめた。
* 9：環境変数 `CONMA_SEED`（0〜2^64-1 の数）を足した。これがあると、スケジューラは、実行できる VProc が 2 つ以上のとき、SplitMix64 の乱数 `r` で、待ち行列の `r mod n` 番目の VProc を先頭に移してから実行する（ほかの順は変えない）。2 つの VM は同じ乱数を同じように使うので、同じ種なら、トレースまで一致する（step10、17、20 の全ケースを、種 1 と 2 で `vm_compare.sh` で確かめた）。数でない値は、終了コード 2 のエラーにした。Python 版（`vproc_step.py` の `SplitMix64`、`Scheduler.run`）と Rust 版（`sched.rs`）の両方を直し、DESIGN.md の「Scheduling and Termination」に書いた。
    * `test/stress.sh [回数] [ディレクトリ ...]` を足した。種を 1 から回数まで変えて、テストを繰り返す（既定は全テスト、Rust 版）。エラーのメッセージの `at step N` は、全 VProc を通したステップ数で、順によって変わるので、それだけが違う diff は出さない。`../consh/test/step6` のように書けば、ConSh のテストも回せる。
    * test/ の全テストを種 1〜20 で、ConSh のテスト（ConMa 版、examples と step1〜20）を種 1〜5 で回した。見つかったのは、step26 の case2 の 1 つだけで、テストの誤りだった。`__Ref_wait_change__` は同じオブジェクトかどうかで比べるが、テストは `__Ref_new__ "a"` と `__Ref_wait_change__ ref "a"` に別の文字列を渡していたので、子の VProc は待たずに進んでいた。ラウンドロビンでは、子が `println "not reached"` に着く前に、親が子を止めていたので通っていた。同じ文字列を渡すように直し、種 1〜40 で通ることを確かめた。
    * そのとき、10 で入れた不具合も直した。`test.sh` をパスを付けずに（`bash test.sh`）呼ぶと、`${0%/*}` が `test.sh` になり、common.sh を読めなかった。`dirname "$0"` を使うようにした。
* 2 の 2 つ目：リゾルバ（`bin/resolver` と `include/resolver.se`）に、大域の define の関数（Head が k 個で、可変長でない）を、0 < m < k 個の引数で呼ぶ所の警告を足した（`Warning: conma/test/step2/test.se:78:3: too few arguments to add2: 1 of 2 (partial application)`）。ただし、既定では出さず、`bin/resolver --warn-arity` のときだけ標準エラー出力に出す。終了コードは 0 のままである。
    * 既定で出さないのは、作者の方針による。ConMa では、部分適用と可変長引数のために、引数の数が違う呼び出しは正しく、よくあるものなので、警告しない。引数の数の誤りをエラーにするのは、部分適用（`bind`）と可変長引数（`. rest`）に明示的な構文がある ConSh の側で行う（consh/PLAN-ja.md で扱う）。
    * 警告を「少ない」ときだけにしたのは、ConMa のコードを数えた結果による。ConSh（2,565 の呼び出し）と include/ には、「少ない」呼び出しが 1 つもなかった。「多い」呼び出しは、Sink を返す関数（`stdout str "\n" ,(out)` など）で、引数のない呼び出しは、関数を値として返すのに、意図して使われている。「多い」の誤りは、結果が関数でなければ、VM が呼び出した位置で `Not a function` を出す（1 と 4）。
    * ConMa 版は `__Resolver_warnings__ Module ,(messages)` で、同じ警告を同じ順で返す（出すかどうかは呼ぶ側が決める）。selfhost7 は、`bin/resolver --warn-arity` の警告と ConMa 版の警告も比べ、case6（arity.se）を足した。
    * RESOLVER.md と SELFHOST.md に書いた。
* 7：作者の方針（memo.txt の 2026-11-06）で、ConMa から Rust への変換を始めた。最初の版（vm/conma2rust の ConMa から Rust への変換器。最初は vm/native と呼んでいた。vm/README.md の「Transpiled code」）は、VM なしで実行できる Body の列（組み込みの呼び出し、`__NULL__`、その場に書いた枝の `__if_then_else__` など）を 1 ステップで実行し、関数の呼び出しで VM に戻る。ConSh のテストはすべて通るが、速さは、単純なループで約 1.5 倍、ConSh で約 1.2 倍にとどまる（ConSh は、単純なコマンド 1 つに約 4,300 ステップかかり、その多くは小さな関数の呼び出しと戻り）。次の版で、関数の呼び出し、戻り、`__Cont_get__` の継続、安全な組み込みも、VM の 1 ステップの中で（1,000 回まで）続けて実行するようにした。単純なループで約 1.7 倍、ConSh で 1.35〜1.45 倍になった。残りの時間は、値の複製と破棄、LVEnv、Closure、循環の回収にかかっていて、さらに速くするには、値の表し方を変える本格的なコンパイル（ローカル変数を Rust の変数にする、組み込みを型ごとに直接呼ぶ、小さな関数のインライン化）が要る。test/step20 の case11 も、実行の順（ステップ数で待つ）に頼っていることが分かった。さらに、よく使う組み込み（`__is_null__`、`__String_equal__`、`__LList_uncons__`、`__Dict_lookup_or__`、数の計算など）を、OList を介さずに直接呼ぶようにした（VM の組み込みと処理を共有する）。単純なループで約 1.9 倍、ConSh で 1.5〜1.6 倍になった。値の持ち方を調べると、`Value` は 24 バイト、ローカル変数の参照は平均 2.1 段で、すぐに直せる無駄はなかった。一方、循環の回収が、ConSh がコマンドごとに誤って作っていた循環（Dict の中の関数がその Dict を取り込む）を回収していたので、ConSh を直した（consh/PLAN-ja.md）。ConSh の関数呼び出しのループは、VM で 3.29 秒から 2.88 秒、ネイティブコードで 1.90 秒（元の 1.7 倍）になった。途中で、`__Ref_set__` をまとめて実行すると、`__Ref_wait_change__` で待つ VProc が値の一時的な変化を見逃すことが分かり、まとめる対象から外した。また、test/step20 の case21 が VProc の実行の順に頼っていることが分かった（普通の VM でも `CONMA_SEED=130` で出力が変わる）。
* 6：作者の方針で、レコードは言語に足さず、ConMa から Rust への変換器（とそれを動かす VM）の側で作った。Dict に「形」（String のキーを入れた順。同じ形の Dict では、各キーが同じ位置にある）を持たせ、変換したコードの、定数の String のキーで Dict を読み書きする所ごとに、最後に見た形とキーの位置を覚える（インラインキャッシュ）。形が同じなら、索引を引かずにその位置を読む。キーを消した Dict、String でないキーを入れた Dict、キーが 64 個を超える Dict は形を持たない。プログラムのすることは変わらない（test/ の全テストが、普通の VM でトレースまで同じ、変換した VM でも通る。consh/test も変換した VM で通る）。richards 20 は 0.78 秒から 0.69 秒（11.5%）になった。nbody、fib、ループと ConSh は変わらない。作る前に測ると、Dict の操作は richards の約 22%（索引を引くのが 13%）、ConSh の約 1% だった（ConSh の時間は、主に値の複製と破棄、LVEnv、呼び出しにかかる）。memo.txt の「Dict をレコードに変更して、どの程度高速化できるか？」への答えで、ConSh への効果はないという見立てのとおりだった。キーの綴りの誤りを見つけることにはならないので、consh/REVIEW-ja.md の 1 は残る。vm/README.md の「Shapes and inline caches」と bench/README.md の表に書いた。
* 8（一部）：作者の判断で、方針を「ConMa 版のフロントエンドのモジュールをリポジトリに入れる」に変えた（memo.txt の 2026-11-06 では「入れず、ビルドのときに作る」だった）。ソースからのビルドにも、ConMa のプログラムの実行にも、Python が要らなくなる。
    * `include/frontend.se`：ConMa 版のフロントエンドのプログラム（`[--warn-arity] [--include <dir>]... <source>`、`bin/includer` と `bin/resolver` を続けたものと同じ）。そのモジュール `vm/frontend.module.se`（520 KB、1 行）をリポジトリに入れた。
    * 欠点を抑えるため：`.gitattributes` で生成物の印を付け、`git diff` に中身を出さない。`bin/update-frontend` だけで作り直す（Python 版のフロントエンドでモジュールを作り、それを VM で動かして自分のモジュールを作る、2 段）。`test/frontend` が、リポジトリのモジュールが自分を 1 バイトも違わずに作ること（不動点）と、Python 版のフロントエンドと同じモジュールになること（`selfhost_compare.py`）を確かめるので、フロントエンドのソースを変えて作り直し忘れると、テストが落ちる。
    * Rust 版の VM はこのモジュールを埋め込み、`conma build <source>` でモジュールを書き、`conma run <source> [<arg>...]` で作って実行する。`__VProc_spawn__` も、`bin/includer` と `bin/resolver` の代わりにこれを使う（別のプロセスとして動かすので、ステップの外で、2 つの VM のトレースは同じまま）。起動に失敗したときの `onError` のメッセージは、2 つの VM とも、フロントエンドのエラーの最初の行にした（`bin/includer` が付け足す `Error: parser exited with code 1` は出さない。step10 の case5 の期待ログから、その行を除いた）。
    * test/ の全テストが両方の VM で通る。基準（Python 版の VM とフロントエンド）は今までどおり Python を使う。変換器（`vm/conma2rust`）は、作者の方針で、改良中のため当面は Python のままとし、将来は ConMa で書く（そのときは、フロントエンドと同じく、Python 版を基準として出力を比べる）。
    * ConMa 版 ConSh の起動からも Python をなくした。スクリプトは ConMa 版の構文解析（`consh_parser.se`）で読み（以前の `--conma-parser`。Python の `consh_parse` は `--python-parser` で使う）、モジュールは、Rust 版の VM があれば `conma build` で作る。`CONMA_VM=rust` なら、ConMa 版 ConSh も Python を使わない（`python3` を、呼ばれたら失敗するものに替えて確かめた）。構文エラーは、Rust 版と同じく `consh:` で始まる。consh/test は、ConMa 版（Rust 版の VM。モジュールを `conma build` で作り直して）と Rust 版で通る。
* ほかはまだ。

### 付録: 実験に使ったプログラム

ファイルのあるディレクトリで `bash <repo>/test/resolver.sh <file>.se` を実行し（10 のとおり、パスは付けない）、`CONMA_VM=rust <repo>/test/vm.sh /tmp/<file>.module.se <関数名>` で動かす。

`bench.se`（実測。関数名は `loop1m` と `fib22`）:

```
(include "util.se")
(define num ,(s) __Number_fromString__ __print_exit_error__ s)

(define loop ,(one n acc)
  __Number_is_zero__ n ,(z)
  __if_then_else__ z
    (,() __NULL__ acc)
    (,() __Number_subtract__ n one ,(n2)
         __Number_add__ acc one ,(acc2)
         loop one n2 acc2))

(define fib ,(one n)
  __Number_is_zero__ n ,(z0)
  __if_then_else__ z0
    (,() __NULL__ n)
    (,() __Number_subtract__ n one ,(n1)
         __Number_is_zero__ n1 ,(z1)
         __if_then_else__ z1
           (,() __NULL__ one)
           (,() __Number_subtract__ n1 one ,(n2)
                fib one n1 ,(a)
                fib one n2 ,(b)
                __Number_add__ a b)))

(define show ,(n)
  __Number_toString__ n ,(s)
  __OS_print__ s ,()
  __OS_print__ "\n")

(define loop1m ,()
  num "1" ,(one) num "1000000" ,(n) num "0" ,(zero)
  loop one n zero ,(r) show r)

(define fib22 ,()
  num "1" ,(one) num "22" ,(n)
  fib one n ,(r) show r)
```

`err.se`（1〜4 の実験。関数名は `too_few`、`too_many`、`wrong_type`、`silent_end`）:

```
(include "util.se")
(define show ,(s) __OS_print__ s ,() __OS_print__ "\n")
(define add2 ,(a b) __Number_add__ a b)
(define num ,(s) __Number_fromString__ __print_exit_error__ s)

(define too_few ,()
  num "1" ,(one)
  add2 one ,(r)
  __Number_toString__ r ,(s)
  show s)

(define too_many ,()
  num "1" ,(one)
  add2 one one one ,(r)
  __Number_toString__ r ,(s)
  show s)

(define wrong_type ,()
  __Number_add__ "x" "y" ,(r)
  show "unreachable")

(define silent_end ,()
  num "1" ,(one)
  add2 one one
  )
```
