# JavaScript 版 ConSh の計画

JavaScript 版 ConSh は、ConMa の VM を WebAssembly（wasm32）にして JavaScript の処理系の上で ConSh を動かし、組み込みコマンド `js` で JavaScript のコードを呼べるようにしたものである。

この文書は、memo.txt の 2026/11/8 の「JavaScript 版 ConSh の方針」を計画にしたものである。関係するファイルは consh/js に置く（VM 自体の変更は vm/ に入る。2.3 を参照）。

## 1. 決めること

次の 3 点は、まだ決まっていない。以下では、それぞれ (推奨) の案で計画を書く。

1. **実行環境**
    - (推奨) まずブラウザで動かす。memo の例（`location.href`、`fetch`）はブラウザを想定している。
    - Node.js は後回しにする（段階 7）。外部コマンドとファイルが使える代わりに、OS 層の作り直しが大きい。また、この Mac の /usr/local/bin/node は Intel 版で動かない（`Bad CPU type in executable`）。テストのために arm64 版を入れ直す必要がある（2.4）。
2. **何を wasm にするか**
    - (推奨) ConMa の VM（vm/）を wasm にし、ConMa 版 ConSh（consh/include）を動かす。memo の「先に ConMa の VM を wasm32 向けにビルドできるようにする」に従う。ConMa の VM が OS 以外の環境でも動くことの確認にもなる。
    - Rust 版 ConSh（consh/rust）を wasm にするのは別の案である。速いが、ConMa 言語を使わない。また std::process と std::io::pipe に深く依存している。
3. **JavaScript の処理系**
    - (推奨) wasm を読み込んだホスト（ブラウザ）の処理系をそのまま使う。`js` のコードは、ホストの JavaScript から間接 eval で評価する。
    - QuickJS などを wasm に組み込む案は取らない。ページの DOM や `fetch` が使えなくなるからである。

## 2. 方針

### 2.1. js の仕様（案）

ConSh の PLAN-ja.md の 1.2 に、次の規則を加える。

1. `js` は組み込みコマンドとする。
    - 優先順位は、ほかの組み込みコマンドと同じく、関数、組み込みコマンド、外部コマンドの順である。
    - `js` という名前の関数があれば、`builtin js` で呼ぶ。
2. 書き方は `js [変数名=] コード {引数}` とする。
    - 展開した後の最初の語が「識別子 `=`」だけでできていれば（`=` の後ろに何もなければ）、`=` より前を変数名、次の語をコード、残りを引数とする。
    - そうでなければ、最初の語をコード、残りを引数とする。
    - JavaScript のコードが `識別子=` だけで終わることはない（構文エラーになる）ので、コードを変数名と取り違えることはない。
        - `js 'x=1'` は代入式のコードである。
        - `x => x * 2` や `a == b` もコードのままである。
    - スペースのない `js sum='...'` は、変数名の指定にならない。これは語全体 `sum=...` をコードとして評価する（多くは代入式になる）。
    - 変数名の後ろにコードの語がなければ、エラーとする（`js: no code`）。`js sum= $code` で `$code` が空になって語が消えた場合も同じである。引数がなく、コードもない `js` だけの場合も同じエラーとする。
    - 例：

      ```
      js '1+2'                                           # 3 を標準出力に書く
      js sum= '(a, b) => Number(a) + Number(b)' $x $y    # sum に戻り値を入れる
      js url= location.href                              # url に戻り値を入れる
      js alert hello                                     # alert("hello") を呼ぶ
      js r= $code                                        # 変数に入れたコードも使える
      ```
3. コードは、JavaScript の式として評価する。
    - 評価には `(0, eval)("(" + code + ")")` を使う。これは大域スコープでの評価で、ConSh の変数は見えない。
    - 式の値が関数なら、引数を渡して呼び、その戻り値を結果とする。
    - 関数でなければ、式の値を結果とする。このとき引数があればエラーとする（`js: arguments given to a value that is not a function`）。
4. **引数の渡し方**
    - 文字列の引数は、JavaScript の文字列として渡す。
    - LazyList の引数（`${list}` を関数に渡す場合と同じく、1 つの値として渡したもの）は、配列として渡す。要素が LazyList なら、入れ子の配列にする。
    - `$list` は、関数呼び出しと同じく葉ごとの引数になる。
5. **結果の出力**（変数名がない場合）
    - 結果は標準出力の LazyList に書く。`console.log` の出力（7）も同じ標準出力に書く。
    - `js` は関数呼び出しと同じく値を出力するので、`> (a b . rest)` は 1.2 の「関数呼び出し > (a b . rest)」の規則に従う。
    - 文字列は、そのまま値 1 つとする。
    - 数値、真偽値、BigInt は、`String()` で文字列にした値 1 つとする。
    - 配列は、要素ごとに値 1 つとする。要素が配列なら、LazyList を値 1 つとする。
    - `undefined` と `null` は、何も出力しない。
    - そのほかのオブジェクトは、`JSON.stringify` の文字列 1 つとする。`JSON.stringify` が例外を出したら（循環参照など）、エラーとする。
    - 関数は、エラーとする（`js: the result is a function`）。
    - Promise（`then` を持つ値）は、解決するまで待ち、その値を同じ規則で出力する。待つ間、ほかの VProc は動き続ける。外部コマンドの終了を待つのと同じである。
    - 同期のイテレータと非同期のイテレータ（ジェネレータを含む）は、要素ごとに値 1 つとする。要素を 1 つ受け取るたびに書くので、`js '...' | head` のように途中までしか読まない使い方もできる（段階 6）。
6. **変数に入れる**（変数名がある場合）
    - 結果は変数に入れ、標準出力には書かない。標準出力に出るのは `console.log` の出力だけになる。
    - 値は、5 の規則に合わせて次のようにする。
        - 文字列、数値、真偽値、BigInt は、文字列 1 つとする。
        - 配列は LazyList とする。要素が配列なら、入れ子の LazyList とする。
        - `undefined` と `null` は空文字列とする（値が足りないときの `> (a)` と同じ）。
        - そのほかのオブジェクトは、`JSON.stringify` の文字列とする。
        - 関数は、エラーとする。
        - Promise は、解決した値を同じ規則で入れる。
        - イテレータは LazyList とする。段階 6 では、要素ができるたびに伸びていく遅延の LazyList にする。
    - 変数は、変数定義の文と同じく、今のシェルの局所変数に入れる。
        - 関数の中なら、その関数の局所変数である。
        - パイプの途中、`&`、`$( )` の中で使った場合は、シェル本体の変数に反映しない（PLAN-ja.md 1.2 の「パイプの途中で行った変数定義は〜」と同じ）。
    - エラーのとき（9）は、変数を変えない。
7. **標準入力**
    - コードの中では、`stdin` という名前で標準入力が見える。
    - `stdin` は非同期のイテレータで、入力の LazyList の値を、文字列化した順に返す。`stdin.lines()` は、全行の配列を返す Promise とする。
    - `stdin` を使わないコードは、標準入力を読まない。
    - 例：`cmd | js n= 'async () => { let n = 0; for await (const l of stdin) n++; return n }'`
8. **console**
    - コードを評価し、結果を出力し終えるまでの間、`console.log` は ConSh の標準出力に、`console.error` は標準エラーに書く。
    - 変数名がない場合、`console.log` の出力と結果は同じ標準出力に混じる。分けたいときは、変数名を書く。
    - 非同期の処理の途中で、2 つ以上の `js` が同時に動いている場合、どの `js` の `console.log` かは区別できない。このときは端末に直接書く（2.4 の制限）。結果は、どの `js` のものかを区別できるので、この制限を受けない。
9. **エラーと終了状態**
    - 構文エラーと例外では、`js: ` に続けてメッセージ（`err.name + ": " + err.message`）を標準エラーに書き、`$?` を 1 にする。
    - 拒否された Promise も同じ扱いである。そうでなければ `$?` を 0 にする。
    - `js` のエラーではシェルは止まらない。外部コマンドのエラーと同じく、`if` で調べられる。
10. **JavaScript のない版**（Python と Rust の VM で動く ConMa 版、Rust 版 ConSh）
    - `js` は、変数名があっても変数を変えずに、`js: JavaScript is not available` を標準エラーに書き、`$?` を 127 にする。

### 2.2. VM の構成

VM は、wasm32-unknown-unknown 向けにも、今の macOS と Linux 向けにもビルドできるようにする。

1. **依存するクレート**
    - vm/ はクレートを使わない方針なので、wasm-bindgen も使わない。
    - 次の 2 つで、ホストの JavaScript とつなぐ。
        - `extern "C"` の import（JavaScript から渡す関数）
        - `#[no_mangle] pub extern "C"` の export
    - 文字列は、wasm のメモリ上のバイト列（UTF-8）と長さで受け渡す。
    - つなぐための JavaScript（consh/js/web/conma.js）は手で書く。
2. **OS 層**
    - sys.rs と、prim の os.rs、process.rs、stream.rs が使う OS の機能を、`cfg(target_arch = "wasm32")` で分ける。
    - wasm 版は、次のようにする。
        - **標準入出力**：標準入力は、ホストから入った行を溜める。標準出力と標準エラーは、ホストの関数に書く。
        - **ファイル**：`__OS_open__`、`__OS_listdir__`、`__OS_chdir__`、`__OS_getcwd__` は、メモリ上の仮想ファイルシステムを使う（パスからバイト列への表）。ページから初期のファイルを置ける。
        - **パイプ**：`__OS_pipe__`、`__OS_dup__`、`__OS_close__` は、VM の中のバッファで作る。
        - **プロセス**：`__OS_spawn__`、`__OS_spawn_detached__`、`__OS_kill__`、`__OS_wait__` は onError に進む（`not supported`）。ConSh は、外部コマンドを `command not found` と同じように扱う。
        - **環境変数**：`__OS_get_envDict__` は、ホストが渡す小さな Dict を返す（`HOME=/`、`PWD=/` など）。
        - **シグナル**：`__OS_trap_interrupt__` は、ホストが Ctrl-C で立てる印を見る。
3. **スケジューラ**
    - ブラウザのメインスレッドでは、待つ間に止まることができない（OS の `poll()` に当たるものがない）。
    - そこで VM は `conma_run(budget)` を export する。これは、ステップを最大 budget 回進めて、次のいずれかの状態を返す。
        - まだ動ける
        - 入力待ち
        - JavaScript 待ち
        - 終了（終了コード付き）
    - ホストは、この状態を見て `setTimeout` か入力のイベントで再び呼ぶ。
    - ネイティブ版のスケジューラの動作（ステップの順と、テストの結果）は変えない。
4. **js のプリミティブ**
    - プリミティブ `__JS_call__ onError code argLList inputLazyList ,(job)` を足す。
        - JavaScript の評価を始め、job（新しい型 JSJob）を返す。
    - プリミティブ `__JS_next__ onError job ,(more value)` を足す。
        - 次の出力値が出るまで、VProc を Blocked にして待つ。
        - `more` が `__FALSE__` なら終わりで、value は終了状態とメッセージである。
    - 標準入力は、JavaScript が `stdin` を読むと、ホストが VM に要求する。VM はそれを入力の LazyList から 1 つ読む仕事として扱い、その VProc の中で値を作って返す。
    - 新しいプリミティブは、docs/PRIMITIVES.md と docs/ROADMAP.md に書く。Python の VM（test/vproc_step.py）にも、onError に進むだけの版を足して、両方の VM の結果をそろえる。
5. **モジュールの読み込み**
    - ConSh のモジュール（約 5 MB）は、ネイティブで bin/includer と bin/resolver（または ConMa の front end）で作っておき、静的なファイルとしてページと一緒に置く。
    - スクリプトの構文解析は、Python の consh_parse がないので、ConMa 版の構文解析（consh_parser.se、`--conma-parser` と同じ）を使う。
    - Rust の VM はモジュールの読み込みに約 130 MB を使う。wasm のメモリでも足りるか、段階 1 で確かめる。

### 2.3. ファイルの置き場所

```
consh/js/
  PLAN-ja.md        この文書
  README-ja.md      使い方（段階 4 で書く）
  build.sh          VM を wasm32 でビルドし、ConSh のモジュールを作って web/ に置く
  web/
    index.html      端末の画面（入力欄と出力）
    conma.js        wasm の読み込み、import する関数、conma_run の駆動
    consh.js        ConSh の起動、仮想ファイルシステム、js の評価
  test/
    run.mjs         Node.js でテストを動かす（ページの代わりに conma.js と consh.js を読む）
    stepN/          ConSh の test/stepN と同じ形のテスト（スクリプトと期待するログ）
```

VM の Rust のコード（wasm 用の OS 層とプリミティブ）は、ネイティブ版と同じ crate に入れる必要があるので、vm/src に置く。ConSh の `js` の組み込みコマンドは、consh/include/consh_eval.se と consh/rust/src/eval.rs に置く（2.1 の 10）。

### 2.4. 当面の制限

- 外部コマンド、`&` で起動する外部コマンド、`kill` は使えない（ConSh の関数と組み込みコマンド、`&` の関数のジョブは使える）。
- ファイルはページを閉じると消える。
- 行の編集と履歴は、ブラウザの入力欄のものだけである。
- 2.1 の 8 の、同時に動く `js` の `console.log` の区別（結果は変数名を書けば区別できる）。
- テストを Node.js で動かすには、arm64 版の Node.js が必要である。今の /usr/local/bin/node は Intel 版で動かない（Rosetta を入れるか、arm64 版を入れ直す）。
- wasm32 のターゲットも必要である（`rustup target add wasm32-unknown-unknown`）。

## 3. 段階

各段階は、そこまでのテストが通り、ネイティブ版のテスト（test/ の両方の VM と consh/test）がすべて通ることで終わりとする。

| 段階 | 内容 | 確かめ方 |
|---|---|---|
| 1 | VM を wasm32 でビルドする。OS 層を 2.2 の 2 のように分け、`conma_run` を export する | test/step1〜8 など OS を使わないテストを Node.js で動かし、ネイティブ版と同じ出力になる。ConSh のモジュールを読み込めて、メモリが足りる |
| 2 | ConSh を wasm 版の VM で動かす。仮想ファイルシステム、パイプ、標準入出力、`--conma-parser` の経路 | consh/test のうち外部コマンドを使わないケースが、Node.js で同じログになる |
| 3 | ブラウザの画面（web/index.html）。対話モード、Ctrl-C、スクリプトの読み込み | ブラウザで `echo`、関数、パイプ、`$( )`、`&` の関数のジョブが動く |
| 4 | `js` の組み込みコマンド（2.1 の 1〜6、9、10。`js [変数名=] コード {引数}` の見分け方と、変数に入れる値を含む）と `__JS_call__`、`__JS_next__`。Promise を待つ間、ほかの VProc が動く | consh/js/test に `js` のテスト。変数名のある形とない形、`js sum=` だけのエラー、`js 'x=1'` が代入式になることを確かめる。ネイティブ版では 2.1 の 10 のエラーになるテストを consh/test に足し、ConMa 版と Rust 版の両方で通る |
| 5 | `stdin` と `console`（2.1 の 7、8） | `cmd \| js '...'`、変数名がある場合とない場合の `console.log` のテスト |
| 6 | イテレータの結果を 1 つずつ出す（2.1 の 5 の最後と、6 の遅延の LazyList） | 無限のジェネレータを `head` 相当の関数で途中まで読むテスト |
| 7 | （後回し）Node.js のホスト。`node:fs` でファイル、`node:child_process` で外部コマンド | consh/test の全ケース |

段階 4 で、ConSh の PLAN-ja.md（1.2 の組み込みコマンド）、README-ja.md、FOR-BASH-USERS.md に `js` を書く。

## 4. 未解決の点

- 2.1 の 3 で、文（`let x = 1; x + 1`）をそのまま書けるようにするか。今の案では式だけなので、`(() => { ... })()` か、関数の `{ ... }` で書く。
- 結果のオブジェクトを JSON の文字列でなく ConMa の Dict にするか。ConSh にはまだ Dict の値がない（ConSh のレビューの 1、ConMa の 6 のレコードとつながる）。
- Rust 版 ConSh も wasm にして、`js` を使えるようにするか（1 の 2）。
