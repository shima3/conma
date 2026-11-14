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
* 5：SJS（JavaScript の安全なサブセット。consh/SAFEJS-PLAN-ja.md）から ConMa への変換器を、ConMa で書いた（consh/include の sjs_lex.se、sjs_parse.se、sjs_gen.se と実行時の支え sjs_rt.se。consh/sjs/README-ja.md）。関数、クロージャ、配列とオブジェクト、ループ、例外（`try`、`finally`）、ジェネレータ、`async` と `await` まで変換し、ConSh の `(( ))` で、ネイティブの VM、Node.js、ブラウザで動く。測った結果（consh/sjs/README-ja.md の「段階 B7」）：
    * 大きさ：SJS 1 行が、ConMa の約 200 バイト（約 9 倍）。実行時の支えは約 1 MB のモジュールになる。
    * 性能（7）：bench/ の loop、fib、nbody を SJS で書くと、手で書いた ConMa の 13〜21 倍のステップになる（直す前は 21〜57 倍。プロファイルで、配列の添字を文字列にして読み戻していた所などを直した）。多くは、動的な型の確認（算術の 1 回に 19 ステップ）と、メンバーの読み書きの種類の判定である。変換器が型を知っている所（数のリテラルとの演算など）で確認を省くこと、添字で読み書きできる列のプリミティブ（6 と同じ問題の、配列の側）が、次に効く。
    * エラーの位置（1）：VM の位置は、変換したコード（1 行の長いソース）を指すので使えない。変換器が、文ごとに SJS の行を記録する（文 1 つと呼び出し 1 つに 2 ステップずつ）。桁と式の中の位置はない。中間言語として使うには、生成したコードに元の言語の位置を付ける仕組み（SInfo に元のファイルと行を書けるようにするなど）があるとよい。
    * 引数の個数（2）：変換したコードの呼び出しは、引数を 1 つの LList で渡す形にしたので、生成したコードでは起きない。一方、変換器を手で書くときに、4 つの引数で `Cat5` を呼び、カリー化された Closure が値になる誤りを 3 回起こした（引数の数を数えるスクリプトで見つけた）。2 の警告（`--warn-arity`。少ないとき）が、まさにこれを見つける。
    * その後、変換器で型の分かる所の確認を省くと（数のリテラルの定数、数だけを入れる変数、算術と比較のプリミティブ、使わない継続を作らないループ）、6.7〜12.5 倍になった（loop 100000 が 0.42 秒、nbody 2000 が 2.22 秒）。生成するコードの側で、静的に分かることを使えば、VM を変えずに大きく変わる。
    * 継続：`return`、`break`、`continue`、例外、ジェネレータ、`async` が、すべて `__Cont_get__` の脱出の継続と PDict だけで書けた。継続は、取った VProc が終わった後も、別の VProc で呼べるので、ジェネレータを別の `(( ))` の評価で続けられる。
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

## 2026-11-14: f4b9702〜4617a89 のコミット

対象は、36a8036（`__TRUE__`、`__FALSE__` と制御の関数を高速版だけの組み込みにしたもの）の前後に入った、ほかの人のコミット 13 個である（f4b9702、3243b7d、89bd100、8471157、61fde4e、1e4c864、edd5945、b20f15a、9d465dc、1dda238、f04c6fd、4617a89、fdf3a11）。その後に入った b361933 と 9a7e69a（機能 `"wasm"`）は見ていない。

### 確かめた方法

* コードの大きいコミット（1e4c864 と edd5945、b20f15a と f04c6fd、9d465dc、4617a89）は、4 つに分けて、それぞれ別のエージェントがコミットの説明と差分と周りのコードを読み、リポジトリの外で小さなプログラムを動かして確かめた。重大なもの（1、2）は、自分でも再現した。
* 文書だけのコミット（f4b9702 の測定の部分、3243b7d、89bd100、8471157、1dda238、fdf3a11）と 61fde4e は、説明と差分を読んだ。
* 4617a89 の時点で、Rust VM、wasm 版、変換版の VM を作り直し、test/ を Python、Rust、変換版の VM で、consh/test を Rust VM（速い版と参照版）と変換版の VM で、test/ と consh/test を wasm 版で、consh/js/test と consh/sjs/test を流した。すべて通った（wasm 版は、前からの test/step40、consh/test の step8 と step17 だけが違う）。

### 1. 定数の畳み込みとフレームが重なると、別の変数を読む（高）

* 場所：vm/src/module.rs の `const_fold`、`subst_func`、`subst_body`（edd5945 と 9d465dc の組み合わせ）。
* `const_fold` は、LCont の Body そのものにフレームがないことしか見ない。LCont の中の関数式が、すでに `frame_chain` でフレームにまとめられていて、その Body がフレームの変数を読まない（`S` がない）と、`subst_func` はその中にも入る。そこでは `L` の添字がフレームの数え方（`L(i-m+1)`）になっているのに、LCont の引数の数だけずらして数え、`new_body` でフレームの印も消す。そのため、置き換えるはずの変数を外し、別の変数を読む。
* 例：`__Number_fromString__ __print_exit_error__ "3" ,(x)` の後に、`run (,() ...)` の中で 8 つの値を束縛してから `x` を使う。Python VM と `CONMA_NO_FOLD=1` の Rust VM は `3` を出すが、Rust VM は 7 つ目の値（文字列）を読み、`__Number_toString__: expected Number, got String` で止まる。簡潔版のモジュールでも同じで、Python VM と結果が違う。
* 直し方の案：`subst_body` が、フレームを持つ Body（`b.frame != 0`）や、フレームにまとめた Function を中に含むときは、畳み込みをやめる。

### 2. 畳み込んだ所では、onError を評価しない（中）

* 場所：vm/src/step.rs の `load_const`、vm/conma2rust/conma2rust.py の 754〜760 行（edd5945）。
* VM は Body を読み込むときに被演算子をすべて評価するので、`__Number_fromString__ noSuchGlobal "1" ,(x)` は、Python VM と、`CONMA_NO_FOLD=1` か `--trace` の Rust VM では `G2 ('noSuchGlobal') Undefined` で止まる。畳み込むと止まらずに進む（`1` を出す）。`--trace` の有無で結果が変わる。
* 直し方の案：畳み込むときも onError を評価する（`__TRUE__` と `__FALSE__` を融合した連鎖で選ぶ所と同じ）。

### 3. 長い数字の文字列で、変換版の VM が作れない（中）

* 場所：vm/conma2rust/conma2rust.py の `number_literal`（83〜87 行。edd5945）。
* 正規表現 `-?[0-9]+(\.[0-9]+)?` に合う 400 桁の数字は、`float()` で無限大になり、Rust にない `inf_f64` を書く。VM の `fold_number` は無限大を除いている（onError で "out of range" になる）が、`number_literal` は除いていない。

### 4. 印刷した SeqClosure の変数名が `?` になる（中）

* 場所：vm/src/step.rs の `kept_scope` と、`apply_closure` の可変長と余分な被演算子の所（b20f15a）、vm/src/trace.rs。
* 変数の名前の情報（スコープ）は `--trace` のときだけ残すが、値の印刷は `__OS_print__` や `__String_concat__` などでも使う。可変長の関数の引数を印刷すると、Python VM は `(__LVEnv__ ("x" "ab"))`、Rust VM は `(__LVEnv__ ("?" "ab"))` を出す。コミットの説明の「印刷した Closure は Python VM と同じ」は、Closure には当てはまるが、SeqClosure には当てはまらない。

### 5. フレームを持つ Closure の印刷が、Python VM と違う（中）

* 場所：vm/src/trace.rs の `closure_to_sexp`、`lvenv_to_sexp`（9d465dc）。
* トレースのときはフレームを使わないので一致するが、印刷とエラーのメッセージでは、フレームが最後の引数の名前で 1 つの `(__Frame__ ...)` として出る。Python VM は、変数ごとに名前を付けて出す。

### 6. 試しに足した 6 つの条件分岐に、機能の印がない（中）

* 場所：vm/src/prim/mod.rs、vm/src/module.rs の `FAST_FEATURES` のコメント（1e4c864）。
* `__if_null_then_else__`、`__unless_null__`、`__Number_if_zero_then_else__`、`__Number_unless_zero__`、`__Number_if_greater__`、`__Number_if_greater_then_else__` は、定義のない大域の名前として、`Uses` のない簡潔版のモジュールからも Rust VM と wasm 版で使える。Python VM では Undefined になり、include/ref に代わりの定義もない。`FAST_FEATURES` のコメントはこれらを `"control"` に入れているが、フロントエンドは書かない。
* 「Rust VM だけ」とあるが、vm/src を共有する wasm 版にも入っている（畳み込みと `CONMA_NO_FOLD` も同じ）。
* テストがない。ステップとトレースが速い道と遅い道で同じこと、数でない値のエラー、NaN、-0、無限大の比較（引き算して符号を見るのと同じであること）、"-0" や指数の書き方の文字列の畳み込みを確かめるものがない。

### 7. 小さいもの（低）

* `(-0) ** 0.5` が、SJS では -0 になる（JavaScript では +0）。consh/include/sjs_rt.se の 2379 行の `Sjs_pow` が、指数が 0.5 のとき `__Number_sqrt__` を使うからである。前からのコードだが、4617a89 で SJS が -0 を作るようになったので表に出た。
* 畳み込んだ Body と、その中で作り直す FuncDef は、関数の番号（`func_id`）を持たない。変換版の VM で一度 VM に戻ると、その Function の残りと、その中で作る Closure は、変換したコードに戻らない（速さだけの問題）。
* フレームにまとめた連鎖の中の `__Number_fromString__` は畳み込まれない（速さだけの問題）。
* 6 つの条件分岐のうち 4 つは、数でない値のエラーのとき、`in_place_failed` で被演算子を片付けない（今は害がない）。
* `CONMA_FAST_REPORT` の行（`fast __if_then_else__, __if__, __unless__: N Bodies`）は、新しい種類も数えるので、名前が古い。
* 4617a89 の `__Number_is_negative_zero__` は、変換版の VM では作り直しも、テストもされていない（表では、ほかの数の述語のような速い道を通らない）。
* 61fde4e の bench/richards/richards_tuned.se は、リポジトリの外にあった richards2.se と、先頭の名前の行だけが違う。

### 問題なしと確かめたところ

* 36a8036 と b3e99ef は、取り込む前の 93adb34 と 6737cc2 と同じである（`git range-diff`）。
* b20f15a、f04c6fd：`--trace` のときの Rust VM のトレースは、Python VM と 1 バイトも違わない（可変長と部分適用のプログラムで確かめた）。部分適用の名前の順、変換したコードが作る Closure、VM が作る Closure（`bool_closure` など）、`__VProc_spawn__` で読むモジュールの名前は正しい。新しい道に `unwrap` はなく、名前のない所は `?` になる。f04c6fd で、wasm 版も `--trace` で名前を残す。
* 9d465dc：フレームを、その場で大きくするのは、Closure、LVNode、Vec のどれもほかから参照されていないときだけで、共有されていれば写す。`__Cont_get__` の継続を Ref に入れて 2 度戻るプログラムは、Python VM と同じ出力になる。`S(m-1-i)` と `L(i-m+1)` の添字、GC がフレームの中の値をたどること、ステップの数は正しい。
* 1e4c864、edd5945：速い道のステップの数は、遅い道と同じである（条件分岐の 1 か 2 ステップ、`load_const` の 3 ステップ）。
* 4617a89：Python VM（`copysign`）と Rust VM の `__Number_is_negative_zero__` は、NaN を含めて同じ値になる。-0 の文字列は、どちらも "0" である。SJS の -0 を、ビット演算、`Math` の関数、`%`、比較、`JSON.stringify`、配列とオブジェクトの添字などの式で Node.js と比べ、上の `**` のほかは一致した。

### 対応

* 1：直した。`subst_body` が、フレームを持つ Body（`b.frame != 0`）に入ったら畳み込みをやめる（vm/src/module.rs）。`const_fold` が LCont の Body そのものを見ていた確かめを、中の関数式の Body にも広げたことになる。フレームの変数を読む Body はもともと `S` で止まっていたので、畳み込まなくなるのは、フレームにまとめた連鎖を中に持ち、その変数を読まない所だけである。test/fast8 を足した（畳み込みにはこれまでテストがなかった）。case1 が指摘の例で、直す前の Rust VM では `expected Number, got String` で止まる。case2 はフレームの先の変数も読む例、case3 はフレームのない関数式に畳み込む例で、Python、Rust、wasm の VM で同じログになり、Rust VM では `CONMA_NO_FOLD=1` と `CONMA_FRAME_MIN=`（フレームなし）でも同じになる。test/ を Python、Rust、wasm の VM で（wasm は前からの step40 だけが違う）、consh/test を Rust VM（速い版と参照版）で、consh/sjs/test と consh/js/test を流して通った。変換版の VM は作り直していない（変換したコードのあるモジュールにはフレームを使わないので、この変更は関わらない）。
* 2：直した。Rust VM の `load_const`（vm/src/step.rs）は、onError が大域の名前なら、畳み込む前にそれを読み、値がなければ畳み込まずに、そのまま読み込む（そこで、畳み込まないときと同じエラーとステップの数で止まる）。局所変数と関数式は、読んでも止まらないので読まない。conma2rust.py も、`__TRUE__` と `__FALSE__` を融合した連鎖で選ぶ所と同じく、大域の onError を先に読んでから f64 の定数を書く。test/fast8 に case4（`__Number_fromString__ noSuchGlobal "1"`）を足した。Python、Rust、wasm の VM で同じログになり、Rust VM では `CONMA_NO_FOLD=1` でも同じになる。変換版の VM は、test/fast8 のコードだけを入れたものを別のディレクトリに作って確かめた（case1〜4 が同じ出力。onError を読む行を消すと、case4 は `1` を出す）。リポジトリの vm/target-transpiled は作り直していない。
* 3：直した。conma2rust.py の `number_literal` は、f64 にすると無限大になる数字を畳み込まず（None）、プリミティブに任せる（onError に "out of range" を渡す。VM の `fold_number` と同じ）。test/fast8 に case5（400 桁の 9）を足した。Python、Rust、wasm の VM で同じログになる。test/fast8 のコードだけを入れた変換版の VM を別のディレクトリに作ると、直す前は `cannot find value inf_f64` でコンパイルできず、直した後は case1〜5 が期待ログどおりになる。
* 4：直した。VProc の今の LVEnv の名前（`VProc::scope`）を、`--trace` のときだけでなく、いつも覚える。VM が Body を読み込んで VProc の LVEnv を決める所（`load_body_interp`、`load_if` の何もしない節、`load_prim` が LCont の Body に進まずにスケジューラに戻るとき）で、その Body のスコープを入れる。`load_prim` が LCont の Body に進むときは、その Body が入れるので入れない。変換したコードに入るときは空にする（変換したコードが VProc に残す LVEnv には名前がないので、その後に作った SeqClosure は、今までどおり `?` を出す）。SeqClosure は、作ったときのそれを持つ（vm/src/step.rs の `kept_scope`）。test/fast9 を足した。普通の読み込み、`load_prim`、`load_if`、`load_const` の後で作った SeqClosure を印刷し、Python VM と比べる。直す前の Rust VM では 4 つとも名前が `?` で違い、直した後は Python、Rust、wasm の VM で同じになる。
    * case4 の畳み込みの後では、畳み込んだ変数が LVEnv にないので、Rust と wasm の VM は値が 1 つ少ない（case4.fold.log。`CONMA_NO_FOLD=1` なら case4.log と同じ）。これは名前でなく、畳み込み（edd5945）そのものの違いである。test/fast9 のコードを入れた変換版の VM では、その変数を `Null` として出す（変換したコードは、もう読まない変数を Null にして LVEnv を作る）。どちらも、印刷した LVEnv だけの違いで、指摘の外として残す。
    * 速さ：Body を読み込むたびにスコープ（`Rc`）を写す手間で、bench/ の nbody 30000、fib 27、loop 3000000、richards_tuned 6 と consh/bench の 00、02、05、07、10、11 が、直す前の VM の 1.000〜1.016 倍になった（交互に 5 回の最速、Apple M2）。直す前の VM どうしでも 0.995〜1.017 倍の揺れがある。`load_prim` でもいつも写すと 1.009〜1.035 倍だった。
* 5：直した。vm/src/trace.rs の `lvenv_to_sexp` は、フレームの節を、その変数ごとに（後に束縛したものから）出し、名前は、フレームのない数え方の添字で引く（frame_chain で作り直した Body も、フレームのないときのスコープを持つので）。Closure、SeqClosure、VProc の LVEnv の印刷が、どれもこれを使う。test/fast9 に case5（引数、フレームの 8 つ、フレームに足した 1 つを持つ継続の Closure）と case6（フレームの変数を持つ LVEnv の SeqClosure）を足した。直す前の Rust VM では 2 つとも違い、直した後は Python、Rust、wasm の VM で同じになり、Rust VM ではフレームなし（`CONMA_FRAME_MIN=`）でも同じになる。test/ を Python、Rust、wasm の VM で（wasm は前からの step40 と、-j 8 で 1 度の step31 だけが違う。step31 は 1 つだけで 3 回通る）、consh/test を Rust VM（速い版）で流して通った。
* 6：直した（6 つを取り除かず、機能 `"control"` に入れることにした）。bin/resolver と include/resolver.se（conma build。vm/frontend.module.se を作り直した）は、6 つのどれかを定義なしで使うモジュールにも `(Uses "control")` を書く。include/ref/control.se に、確かめるプリミティブ（`__is_null__`、`__Number_is_zero__`、`__Number_subtract__` と `__Number_is_positive__`）と `__if_then_else__`、`__if__`、`__unless__` で書いた定義を足したので、bin/concise がそれを入れ、Python VM でも動く（bin/concise は定義のファイルの名前を見るので、変えたのは説明だけ）。変換版のコードは、今までどおり VM に任せる。docs/PRIMITIVES.md の 15 に 6 つの説明を、16 の `"control"` に名前を足し、vm/README.md、docs/SPEED-ja.md、docs/FAST-PLAN-ja.md の 4.6 の「Rust VM だけ」「フロントエンドは知らない」を直した（wasm 版にもあることも）。
    * test/fast10 を足した。6 つを、その場に書いた節（速い道）と変数で渡した節で、`__NULL__` と Null でない値、0、-0、1、-1e-320、無限大、NaN、大小と等しい組、0 と -0、無限大どうし、差が無限大になる組で使い、組み込み（test.se）と定義（conma.se）が同じ出力になることを確かめる（定義は a - b が正かで選ぶので、引き算して符号を見るのと同じことも確かめたことになる）。数でない値（case4〜6）は、組み込みは自分の名前とステップで、定義は確かめるプリミティブの名前とステップで止まる（caseN.python.log）。Rust VM と wasm 版では、`--trace`（1 ステップずつ）となしで、最後のエラーのステップの番号が同じになることも確かめる。6 つのどれか 1 つだけを使うモジュールに、bin/resolver と conma build が `(Uses "control")` を書くことも確かめる（直す前の bin/resolver は書かなかった）。Python、Rust、wasm の VM で通る。簡潔版のモジュールに 6 つの定義が入るので、test/fast4 の、印を付けた Body の数を 158 から 166 にした。test/ を Python、Rust、wasm の VM で（wasm は前からの step40 だけが違う）、consh/test を Rust VM（速い版と参照版）で、consh/sjs/test と consh/js/test を流して通った。指摘の、-0 や指数で書いた文字列の畳み込み（edd5945 の `fold_number`）のテストは、その後、test/fast8 の case6 として足した。"-0"、"-0.0"、"0"、"+5"、"1e3"、"-2.5E-3"、".5"、"5."、"-.25"、"1e+2"、"1e-400"（0）、"-1e-400"（-0）を畳み込み、範囲の外の "1e400" と、数でない "1e"、"."、空の文字列、" 1" は畳み込まずにプリミティブが onError を呼ぶ。値と -0 かどうか（`__Number_is_negative_zero__`）を出し、Python、Rust、wasm の VM で同じになり、Rust VM では `CONMA_NO_FOLD=1` と `CONMA_FRAME_MIN=` でも同じになる。test/fast8 のコードを入れた変換版の VM を別のディレクトリに作り、case1〜6 が同じ出力になることも確かめた（conma2rust.py は、"-0"、"-0.0"、"0" を f64 の定数にし、"-0" は `-0.0_f64` になる。ほかの書き方はプリミティブに任せる）。
* 7：
    * `(-0) ** 0.5`：直した。consh/include/sjs_rt.se の `Sjs_pow` は、指数が 0.5 のとき、底が 0 か -0 なら +0、-Infinity なら Infinity を返す（平方根では -0 と NaN になる。JavaScript の `**` は +0 と Infinity）。consh/sjs/test/step8 の case10 に `**` の行を足し、Node.js と同じ出力になることを確かめた。直す前は `(-Infinity) ** 0.5` も NaN で、JavaScript と違っていた。
    * 4 つの条件分岐の `in_place_failed`：直した。`__Number_if_zero_then_else__`、`__Number_unless_zero__`、`__Number_if_greater__`、`__Number_if_greater_then_else__` は、数でない値のとき、ほかのプリミティブと同じく、その場で読んだ被演算子を外してから止まる（vm/src/prim/control.rs）。
    * `CONMA_FAST_REPORT` の行：`fast conditionals: N Bodies` にした（test/fast3 の report.log も）。数えるのは、`load_if` の速い道に印を付けた Body で、6 つの条件分岐も入る。
    * 変換版の VM の `__Number_is_negative_zero__`：test/transpiled の case1 に、融合した連鎖で -0 を作り、その Boolean を 2 つの値に適用し、`__if_then_else__` の条件にもする `negzero` を足した。Python、Rust、wasm の VM で通り、case1 のコードだけを入れた変換版の VM を別のディレクトリに作って、同じ出力になることを確かめた（変換したコードは、このプリミティブを OList を通して呼ぶ。vm/target-transpiled は作り直していないので、そこでは変換したコードを使わない）。
    * 畳み込んだ Body の `func_id` と、フレームにまとめた連鎖の中の畳み込み：速さだけの問題なので、直していない。
    * richards_tuned.se と richards2.se：61fde4e の説明のとおり、リポジトリの外で手で直したものを入れたので、そのままにする。

## 2026-11-14（2）: SJS-WASM のコミット

対象は、consh/SJS-WASM-PLAN-ja.md と consh/SJS-WASM-ARRAY-PLAN-ja.md を作ったほかの人のコミット 9 個である（b361933、9a7e69a、959ee3b、d22dc3b、fe4d82e、94fc307、881e76c、aaeafb1、df26f30）。その間に入ったレビューへの対応のコミットは見ていない。

### 確かめた方法

* 3 つに分けて、それぞれ別のエージェントが、コミットの説明と差分と周りのコードを読んだ。分け方は、9a7e69a と df26f30（プリミティブとアセンブラ）、959ee3b と aaeafb1（SJS から WAT への変換）、d22dc3b、fe4d82e、94fc307 と計画の 2 つ（燃料、ConSh、測定）である。それぞれ自分の worktree で Rust VM と wasm 版（Node.js）を作り、小さなプログラムを Node.js、Rust VM、wasm 版で比べた。
* 重大なもの（1〜3）は、alpha（df26f30 の後）の worktree で、自分でも再現した。4、9 はコードを読んで確かめた。

### 1. WAT に `;` が 1 つだけあると、アセンブラが止まらない（高）

* 場所：vm/src/prim/wasm_asm.rs の 180〜187 行（読み込みのアトムの枝。9a7e69a）。
* `;` の次が `;` でないと、コメントでなくアトムの枝に入る。しかし、アトムを読むループは `;` で止まるので、1 文字も進まず、空のアトムを積み続ける。`(module a;b)` のような 1 文字の誤りでも起きる。
* 例：`__Wasm_compile__ onError "(module ;)"`。Rust VM は返らない（エージェントの測定では、2 秒で RSS が約 730 MB）。wasm 版は、メモリを確保できずに VM ごと落ちる（`RuntimeError: unreachable`）。onError に位置とメッセージを渡すべきところ。
* 直し方の案：アトムが空なら、`;` の誤りとして onError にする。

### 2. ブロックを出た let と const を、wasm 版は局所の変数のまま読む（高）

* 場所：consh/include/sjs_wasm.se の `Wx_id`（878〜883 行）、`Wx_new_local`（562〜582 行）、ブロックと for の変換（639、669〜677 行。959ee3b）。
* 局所の変数の表（`locals`）が関数で 1 つで、ブロックや for を出ても名前が消えない。そのため、ブロックの外の同じ名前が、外の変数でなく、ブロックの中の変数を読む。
* 例：`let t = 100; function f(a) { { let t = a; } return t }; console.log(f(7))` は、Node.js と Rust VM が 100、wasm 版が 7。for の `let k` をループの後で読むと、Node.js と Rust VM は外の `k`（-1）、wasm 版は 2。ループの変数 `i` を後で読むと、Node.js と Rust VM は `ReferenceError`、wasm 版は 3。
* 直し方の案：ブロックごとに名前の表を重ね、出るときに戻す（WAT の局所の変数は、名前を変えて使い分ける）。または、ブロックで宣言した名前を外で読む関数を、数の関数にしない。

### 3. 分割代入で Math を書き換えても、数の関数のままになる（高）

* 場所：consh/include/sjs_wasm.se の `Wx_env_node`（158〜162 行。959ee3b）。
* `mathWritten` が立つのは、代入の先そのものが `Math.名前` のときだけで、分割代入の中の `Math.floor` は見ない。段階 2 の `Sjs_wasm_fn` は呼ぶときに Math を確かめないので、wasm 版は組み込みの floor で計算する。
* 例：`function fl(x) { return Math.floor(x) }; [Math.floor] = [(x) => 42]; console.log(fl(1.5))` は、Node.js と Rust VM が 42、wasm 版が 1。`({ floor: Math.floor } = ...)` も同じ。普通の代入、`Math['floor'] = ...`、`const M = Math; M.floor = ...` は、正しく数の関数から外れる。
* 直し方の案：分割代入の先もたどる（`Wx_walk` が集める先に、メンバーの式を入れる）。

### 4. Math を使う数の関数を ConMa から呼ぶと、wasm を使わないときより遅い（中）

* 場所：consh/include/sjs_rt.se の `Sjs_wasm_math_ok`（3505〜3510 行）と `Sjs_wasm_fn_core`（d22dc3b、fe4d82e）。
* 呼ぶたびに、メンバーの名前の文字列を 1 文字ずつ分け直し（`Sjs_wasm_words`）、10 個のメンバーを比べる。この確かめは busy を見る前に動くので、ConMa 版で動いている再帰の各段でも行う。
* エージェントの測定：ConSh で `hyp = (a,b) => Math.sqrt(a*a+b*b)` を ConMa の関数から 5 万回呼ぶと、wasm 版 7.04 秒、`CONMA_NO_WASM=1` 2.18 秒、Rust VM 1.59 秒（出力は同じ）。Math を使わない関数では差がない。
* 直し方の案：名前の LList を一度だけ作って持つ。Math の書き換えに番号を付け、番号だけを比べる。

### 5. 燃料を使い切らせても、memory.grow の結果は戻らない（中）

* 場所：consh/js/conma.js の `host_wasm_call`（df26f30）。メモリの中身は 0 で埋め直すが、大きさは戻さない。docs/PRIMITIVES.md の 16 は memory.size しか書いていない。
* エージェントの例：`(memory 1 2)` で `memory.grow 1` の結果を返す関数は、十分な燃料なら 1 を返すが、燃料 10 で切れた後に 2 倍の燃料でやり直すと -1 を返す。「同じ引数なら同じ結果」「燃料の切れた呼び出しは何も残さない」（計画 5.5 のやり直しの前提）が崩れる。今の SJS の変換は memory.grow を書かないので、SJS では起きない。
* 直し方の案：呼ぶたびにメモリを作り直すか、大きさを最小に戻す。そうしないなら、文書に書く。

### 6. 深い入れ子と深い再帰で、wasm 版が落ちる（中）

* WAT：`(f64.neg ...)` を 30 万重ねた関数を `__Wasm_compile__` に渡すと、wasm_asm.rs の `encode_folded` と `encode_seq` の再帰で、ホストのスタックが足りず、Node.js ごと終わる（Rust VM は onError に進む）。深さに上限を設け、onError にするのがよい。
* ConSh：2 つの引数を持つ数の関数を `(( ))` で作り、`$(( g(0, 1000000) ))` で 100 万段の再帰をさせると、wasm 版は ConMa 版に替わった後、GC の中でメモリを確保できずに落ちる（`CONMA_NO_WASM=1` と Rust VM では通る。90 万段なら通る）。原因は分かっていない（4 GB の上限の近くで、wasm を使う道が余分に使っている）。

### 7. Ctrl-C（前からの問題。段階 3 の説明と食い違う）（中）

* Node.js の wasm 版の対話モードでは、終わらない `(( ))` に Ctrl-C が届かない。consh/js/vm-node.js の 392 行は、子プロセス、JS、入力待ちがあるときしかイベントループに戻らないので、SIGINT の受け手が動かない（`CONMA_NO_WASM=1` でも同じ）。d22dc3b の「やり直しの合間に SIGINT が届く」を確かめた test/fast7 の case7 は、子プロセスが動いているときだけ成り立つ。
* Rust VM でも、Ctrl-C で止めた `(( ))` の SJS の VProc（consh/include/consh_jsstmt.se の 533 行の `__VProc_spawn__`）は止まらず、裏で CPU を使い続ける（`__VProc_kill__` も `Kill_tree` も届かない）。これを直すと、`Sjs_wasm_fn_core` の busy が減らないまま残る（その関数は、その後ずっと ConMa 版で動く）ことも直す必要がある。
* 時間の区切り：燃料の切れ目で区切りを終える仕組み（vm/src/sched.rs の `end_slice`）は、VProc が 1 つで、ほかに何も待っていないときは働かない（内側のループが budget まで続ける）。trap を登録している ConSh では働く。

### 8. 小さいもの（低）

* アセンブラが、正しくない WAT を受け付ける：`end` の後の違うラベル（`block $a nop end $b`）、`i32.const 0x-1`、`local.get +0`、`1__0`。逆に、正しい `else $l` を拒む。
* `__Wasm_call_memory__` に、メモリの max に入らない数の語を渡すと、onError でなく `trap` になる。
* consh/sjs/bench/run.sh の 55 行の WAT の大きさは、`grep -o 'Sjs_wasm_module "[^"]*"'` が最初の `\"` で止まるので、補助の関数の分しか数えない（fib の WAT は 2,038 バイトだが 1,700 と出る）。consh/sjs/README-ja.md と計画の 16 の「1,700 バイト」もそのため。
* 測った機械：docs/SPEED-ja.md、計画の 16、consh/sjs/README-ja.md は「Apple M1」と書くが、測った機械は Apple M2 と見られる（同じ機械で run.sh を流すと、記録とほぼ同じ数が出る）。
* 計画 5.6 は、wasm のモジュールも ConMa のモジュールと一緒に覚えるとするが、実装は評価のたびに `__Wasm_compile__` で作り直す（差は小さい）。計画の 15 に、この違いがない。
* consh/sjs/test/step10/test.sh は、TMPDIR を見ずに `/tmp/sjs-test-$$.log` に書く。

### SJS-WASM のコミットより前からの、JavaScript との違い

* `%` は `a - b * trunc(a / b)` で求めるので、`5.5 % 1.1` は 0（Node.js は 1.0999999999999996）、`1e17 % 3` と `1e300 % 7` は 0（Node.js は 1）。ConMa 版も wasm 版も同じ値を出す（consh/sjs/README-ja.md が「大きな数では下の桁が違うことがある」と書くものより広い）。
* 値が undefined の `$(( ... ))`（値を返さない数の関数の呼び出しも）は、ConSh を `ref` と出して終了ステータス 1 で終える。

### 問題なしと確かめたところ

* 9a7e69a、df26f30：test/fast7 は Rust VM と wasm 版で通る。LEB128、i32 と i64 の定数の範囲、ラベルの深さ、引数と局所の変数と関数の番号、セクションの順序、燃料の大域変数と、関数の入口と各 loop での燃料の確かめ、memarg、予約した export の名前、引数の数と型と燃料の確かめ（NaN、無限大、2^53）、"ok" のときだけ語を書き戻すこと、-0、メモリのないモジュールへの `__Wasm_call_memory__` の onError、`Value::Wasm` の GC と解放、フロントエンドと bin/concise の `"wasm"`。import、memory（A1 の前）、global、start を受け付けないのは、b361933 の決定どおり。
* 959ee3b、aaeafb1：Math の関数、ビット演算と ToInt32、比較、複合代入、++ と --、三項演算子、`&&` と `||`、break と continue のあるループ、再帰（深さ 10 万も）、燃料を倍にしてやり直す長いループを、0、-0、±0.5、2^31 と 2^32 の前後、2^53+1、NaN、±Infinity、非正規化数で試し、上の 2、3 と `%` のほかは、Node.js、Rust VM、wasm 版が一致した。数でない引数、足りない引数と余る引数では ConMa 版に戻る。A0 の、値を返さない関数と、一番上の定数の式。数の関数から外れるべきもの（`**=`、`||` の値、ラベル付きの continue、関数名への代入）は外れる。step9 の数の関数の一覧は .wasm.txt と同じ。不正な WAT は一度も出なかった。
* d22dc3b、fe4d82e、94fc307：燃料は 2^16 から 2^30 まで倍にし、超えるか trap なら ConMa 版に替わるので、やり直しは止まり、あふれない。step10 と consh/test/step21 の case9 は wasm 版で通る。外の変数に代入する関数、宣言していない名前に代入する関数、自由変数を読む関数は数の関数にならない。`(( ))` で作った関数や Math を後で書き換えても、Rust VM と同じ出力になる。パイプと `&` の中の `$(( ))` も同じ。run.sh の fib 27 と loop 10^7 の時間は、記録とほぼ同じ比で再現した。

### 対応

* 1：直した。vm/src/prim/wasm_asm.rs の読み込みは、アトムが空なら（コメントを始めない `;` の上で止まったとき）`unexpected ;` で onError に渡す。
* 6 の前半：直した。リストの入れ子（読み込み）と、block、loop、if と folded の命令の入れ子（`encode_seq`、`encode_folded`。`Ctx::depth`）を、10000 段までにし、超えれば `nested more than 10000 deep` で onError に渡す（読み込み、組み立て、リストを捨てるときの再帰の深さが、これで決まる。wasm 版の VM で 3 万段まで動いたので、余裕をとった）。リストの段数は module と func の分も数える。
    * test/fast7 に case9 を足した。`(module ;)`、アトムの中の `;`、行コメント、f64.neg を 9990 段（作れる）、10001 段、3 万段重ねた folded の命令、block を 10000 段（作れる）と 10001 段重ねたもの、3 万段のリスト。ネイティブの VM（case9.log）は組み立ててから "not available"、wasm 版（case9.wasm.log）は作れるものを "compiled"、Python VM（case9.python.log）はすべて "not available" になる。直す前の wasm 版の VM では、case9 の最初の `;` で、メモリを確保できずに VM ごと落ちた。test/ を Python、Rust、wasm の VM で（wasm は前からの step40 だけが違う）、consh/test を Rust VM（速い版）で、consh/sjs/test と consh/js/test を流して通った。
* 3：直した。consh/include/sjs_wasm.se の `Wx_env_node` は、代入の先（分割代入、for の宣言）をたどるとき、その中の `Math.名前` も Math の書き換えとみなす（既定値や計算したキーの中で読むだけの `Math.名前` も書き換えとみなすので、慎重すぎる側に倒れる。そのときは数の関数にしないだけで、結果は変わらない）。consh/sjs/test/step9 に case6 を足した（配列とオブジェクトの分割代入で Math.floor と Math.round を書き換え、Math を使わない関数は数の関数のまま）。直す前の wasm 版は `1 2 3` を出し、Node.js は `42 -1 3`。
* 2：直した。sjs_wasm.se の `Wx_scoped` が、ブロック（`{ }`、for、for...of）で宣言した let と const を、ブロックを出るときに局所の変数の表（`locals`）から外す（`Wx_declare` が、今のブロックの名前の列 `"scope"` に足す）。外した後の同じ名前は、外の数の定数を読むか、なければ数の関数にしない（ConMa 版が JavaScript と同じく外の変数を読むか、ReferenceError にする）。WAT の局所の変数（`$l番号`）は関数で 1 つずつのまま。兄弟のブロックで同じ名前を宣言する関数は、前は宣言の重なりとして数の関数にしなかったが、今は数の関数になる。step9 に case7 を足した（ブロック、for、for...of、入れ子のブロック、if の両方の枝、兄弟のブロックの同じ名前、for の変数を後で読むと ReferenceError）。Node.js、Rust VM、wasm 版で同じ出力になり、直す前の wasm 版は `7 2 ...` と、`ReferenceError` の代わりに 3 を出した。
* 8 のアセンブラ：直した。vm/src/prim/wasm_asm.rs は、`end` と `else` の後のラベルが、その block（if）のラベルでなければ誤りにし、`else $l` を受け付ける（`end_label`）。数の `_` は数字と数字の間だけ（`digits_ok`。f64 も）、`0x` の後は 16 進の数字だけ、局所の変数、ラベル、関数の番号と offset とページ数は符号なし（`parse_uint`）。test/fast7 に case10 を足した（ログは case9 と同じく 3 つ）。
* 2、3、8 のアセンブラの後で、test/ を Python、Rust、wasm の VM で（wasm は前からの step40 と、-j 8 で 1 度の step31 だけが違う。step31 は 1 つだけで 3 回通る）、consh/test を Rust VM（速い版と参照版）で、consh/test/step21 を wasm 版で、consh/sjs/test と consh/js/test を流して通った。
* 8 の測った機械（Apple M1 と M2）：直す必要なし。測った人が使っている機械の違いで、速さの記録には機種を書くのがよく、機種が違ってもかまわない（2026-11-14 にユーザーと確かめた）。
* 6 の後半、4、5、7、8 のほか：まだない（4 と run.sh は、SJS-WASM-ARRAY の段階 A5 の測定と順番を合わせる）。
