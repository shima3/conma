# JavaScript 版 ConSh の計画

JavaScript 版 ConSh は、ConMa の VM を WebAssembly（wasm32）にして JavaScript の処理系の上で ConSh を動かし、組み込みコマンド `js` で JavaScript のコードを呼べるようにしたものである。

この文書は、memo.txt の 2026/11/8 の「JavaScript 版 ConSh の方針」を計画にしたものである。関係するファイルは consh/js に置く（VM 自体の変更は vm/ に入る。2.3 を参照）。

## 1. 決めたこと

次の 3 点は、それぞれ (推奨) の案に決めた（2026/11/8）。4 は、JS の文（2.5）を作ったときに決めた。

1. **実行環境**
    - (推奨・決定) まずブラウザで動かす。memo の例（`location.href`、`fetch`）はブラウザを想定している。
    - Node.js のホストで外部コマンドを使えるようにするのは後回しにする（段階 7）。OS 層の作り直しが大きい。テストは、Node.js（vm-node.js）か、macOS の JavaScriptCore のシェル（jsc、vm-jsc.js）で動かす（2.4）。
2. **何を wasm にするか**
    - (推奨・決定) ConMa の VM（vm/）を wasm にし、ConMa 版 ConSh（consh/include）を動かす。memo の「先に ConMa の VM を wasm32 向けにビルドできるようにする」に従う。ConMa の VM が OS 以外の環境でも動くことの確認にもなる。
    - Rust 版 ConSh（consh/rust）を wasm にするのは別の案である。速いが、ConMa 言語を使わない。また std::process と std::io::pipe に深く依存している。
    - Rust 版 ConSh は wasm にしないことにした（2026/11/8）。ネイティブでは ConMa 版より 8〜30 倍速いが、パイプライン、`$( )`、`&` を OS のスレッドと `Condvar` で動かしているので、スレッドがなく、メインスレッドで待てないブラウザのために、並行の仕組みを作り直す必要がある。ブラウザで ConSh と JavaScript を組み合わせるには ConMa 版の速さで足り、ConMa を中間言語として確かめる意味もある。
3. **JavaScript の処理系**
    - (推奨・決定) wasm を読み込んだホスト（ブラウザ）の処理系をそのまま使う。`js` のコードは、ホストの JavaScript で評価する（今は `new Function`。2.1 の 3）。
    - QuickJS などを wasm に組み込む案は取らない。ページの DOM や `fetch` が使えなくなるからである。
4. **JS の文は JavaScript のある版だけ**（2.5。2026/11/9 に決めた）
    - 先頭の語がコマンドでない文を JavaScript にするのは、wasm 版の VM で動く ConMa 版だけである。ネイティブの VM で動く ConMa 版と、Rust 版の ConSh（consh/rust）、Python の consh_parse には入れない。そこでは、今までどおりコマンドが見つからないエラーになる。
    - そのため、`x = 1` のような行は、版によって意味が変わる。どの版でも同じに動かしたいスクリプトでは、`js '...'` を使う（ネイティブ版では `js` もエラーになるが、黙って違う動きにはならない）。

## 2. 方針

### 2.1. js の仕様

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
    - 評価は大域スコープで行い（`new Function("stdin", "args", "return (" + コード + ")")`。strict でない）、ConSh の変数は見えない。式として読めなければ、async 関数の本体にする（下の 3 つ目）。今の評価は、この 2 段である（JS の文は 3 段。2.5 の 5）。
    - 式の値が関数なら、引数を渡して呼び、その戻り値を結果とする。
    - 関数でなければ、式の値を結果とする。このとき引数があればエラーとする（`js: arguments given to a value that is not a function`）。
    - 式として読めないコードは、async 関数の本体として動かし、`return` の値を結果とする（`return` がなければ `undefined`、何も出さない）。`await` を書け、`stdin` と、引数の配列 `args` が見える。式として読めるコードは、文としても読めても式とする（`{a: 1}` はオブジェクト）。
        - 例：`js 'let x = 1; return x + 1'`、`js 'const r = await fetch("data.txt"); return r.status'`
        - 文の並びの最後の式の値（eval の完了値）は使わない。`stdin` と `args` を見せられず、また実行時の SyntaxError（`JSON.parse` など）と構文エラーを見分けられないので、コードを 2 度動かすおそれがあるからである。
        - どちらとしても読めなければ、式としての構文エラーを出す。
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
    - 値は、5 の規則に合わせて次のようにする（2026/11/9 から、文字列とイテレータのほかは JS の値として入れる。2.5 の 4。`$` で読んだときの値は、下の規則のままである）。
        - 文字列、数値、真偽値、BigInt は、文字列 1 つとする。
        - 配列は LazyList とする。要素が配列なら、入れ子の LazyList とする。
        - `undefined` と `null` は空文字列とする（値が足りないときの `> (a)` と同じ）。
        - そのほかのオブジェクトは、`JSON.stringify` の文字列とする。
        - 関数は、エラーとする（JS の値になってからは、`$f` で呼べる関数値になる）。
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
    - どの `js` の出力にするかは、次の順に決める（3.5）。コードが同期に動いている間（評価、`then` の中、ジェネレータの `next`）は、その呼び出し。Promise を待つ間などで、それがなければ、終わっていない呼び出しが 1 つだけならそれ。
    - 非同期の処理の途中で、2 つ以上の `js` が同時に動いている場合、どの `js` の `console.log` かは区別できない。このときは端末（ホストの標準出力と標準エラー出力）に直接書く（2.4 の制限）。終わっていない呼び出しがなければ、元の `console` に渡す。結果は、どの `js` のものかを区別できるので、この制限を受けない。
9. **エラーと終了状態**
    - 構文エラーと例外では、`js: ` に続けてメッセージ（`err.name + ": " + err.message`）を標準エラーに書き、`$?` を 1 にする。
    - 拒否された Promise も同じ扱いである。そうでなければ `$?` を 0 にする。
    - `js` のエラーではシェルは止まらない。外部コマンドのエラーと同じく、`if` で調べられる。
10. **JavaScript のない版**（Python と Rust の VM で動く ConMa 版、Rust 版 ConSh）
    - `js` は、変数名があっても変数を変えずに、`js: JavaScript is not available` を標準エラーに書き、`$?` を 127 にする。

### 2.2. VM の構成

VM は、wasm32-unknown-unknown 向けにも、今の macOS と Linux 向けにもビルドできるようにする。

1. **crate とクレート**
    - wasm 版は、consh/js/wasm の crate（conma-wasm、cdylib）でビルドする。vm/src のファイルを `#[path]` でそのまま使い、sys だけを consh/js/wasm/src/sys.rs に替える。vm/Cargo.toml とネイティブ版のビルドは変わらない。
    - vm/ はクレートを使わない方針なので、wasm-bindgen も使わない。
    - 次の 2 つで、ホストの JavaScript とつなぐ。
        - `extern "C"` の import（JavaScript から渡す関数）
        - `#[no_mangle] pub extern "C"` の export
    - 文字列は、wasm のメモリ上のバイト列（UTF-8）と長さで受け渡す。
    - つなぐための JavaScript（consh/js/conma.js）は手で書く。ブラウザでも、Node.js でも、macOS の JavaScriptCore のシェル（jsc）でも動く。
2. **OS 層**
    - prim の os.rs、stream.rs が直接使っていたファイル、環境変数、標準出力は、vm/src/sys.rs の関数（`open_file`、`is_dir`、`canonicalize`、`read_dir`、`env_vars`、`getpid`、`write_std`）を通す。process.rs と value.rs の Unix にしかないところは `cfg(unix)` で分ける。
    - wasm 版の sys.rs は、同じ関数を、メモリ上の仮想の OS で作る。何も待たない（データのない読み込みは EAGAIN）。
        - **標準入出力**：標準入力は、ホストから入ったバイト列を溜める。標準出力と標準エラーは、ホストの関数 `host_write` に書く。
            - `__OS_read__` は、データがなければ `""` を返すので、待たずに読み直すプログラムでは、VM が止まらない。そこで、VProc が空の標準入力を見たら印を立て（`conma_wants_input`）、ホストはそれを見て入力を渡す。
        - **ファイル**：`__OS_open__`、`__OS_listdir__`、`__OS_chdir__`、`__OS_getcwd__` は、メモリ上の仮想ファイルシステムを使う（パスからバイト列への表）。ページから初期のファイルを置ける。
        - **パイプ**：`__OS_pipe__`、`__OS_dup__`、`__OS_close__` は、VM の中のバッファで作る。大きさに上限がないので、書き込みは待たない。
        - **プロセス**：`__OS_spawn__`、`__OS_spawn_detached__` は onError に進む（`__OS_spawn__: not supported: <コマンド>`）。`__VProc_spawn__` も、bin/includer を起動できないので onError に進む。ConSh は、外部コマンドを `command not found` と同じように扱う。
        - **環境変数**：`__OS_get_envDict__` は、ホストが渡す小さな Dict を返す（`HOME=/`、`PWD=/` など）。
        - **シグナル**：`__OS_trap_interrupt__` は、ホストが Ctrl-C で立てる印を見る。
3. **スケジューラ**
    - ブラウザのメインスレッドでは、待つ間に止まることができない（OS の `poll()` に当たるものがない）。
    - そこで VM は `conma_run(budget)` を export する。これは、ステップを約 budget 回進めて、次のいずれかを返す。
        - -1：まだ動ける VProc がある（`Slice::Paused`）
        - -2：どの VProc も外（標準入力、Ctrl-C、段階 4 からは JavaScript）を待っている（`Slice::Idle`）
        - 0 以上：終了コード
    - ホストは、この値を見て `setTimeout` か入力のイベントで再び呼ぶ。
    - vm/src/sched.rs の `Machine::run` は `run_slice(budget)` を呼ぶ形にした。`Machine::blocking` が真（ネイティブ版）なら、今までどおり OS で待つ。偽（wasm 版）なら、待つ代わりに `Slice::Idle` を返す。ネイティブ版のステップの順とテストの結果は変わらない。
4. **js のプリミティブ**（段階 4 で作った形。docs/PRIMITIVES.md、docs/ROADMAP.md の Step 40）
    - プリミティブは `__JS_call__ onError code args ,(inputStream)` の 1 つだけにした。新しい型も、スケジューラの変更もいらない。
        - VM は、仮想の OS の中にパイプを作り、書く端の fd と code、args（String と LList の入れ子を JSON の配列にしたもの）をホストの `host_js_call` に渡して、読む端の Stream を返す。
        - ホスト（conma.js）は、code を評価し、結果を 1 行の S 式にして、`conma_run` が戻った後に `conma_write` で書き、`conma_close` で閉じる：`(r s "文字列")`、`(r l (リスト))`、`(r n)`、`(e "メッセージ")`。
        - VProc は、ほかの Stream と同じく読めるまで待つ。その間、ほかの VProc は動き、`conma_run` は IDLE を返す。ホストは、結果を書いたら（`jsDone`）VM を動かし直す。Node.js の vm-node.js は、IDLE で JavaScript を待つ間と、動いている間にも時々、イベントループを回す。
        - ConSh の `Builtin_js` は、Stream を行の LazyList にして `__Sexp_read__` で読み、出力するか変数に入れる。
    - 段階 5 で、`__JS_call__ onError code args ,(inputStream outputStream)` にした。結果の行の前に、ホストは次の行を書く。
        - `(o "...")`、`(x "...")`：`console.log`（`info`、`debug`）と `console.error`（`warn`）の出力。ConSh は、1 回の呼び出しを 1 つの要素として、標準出力と標準エラー出力に書く。
        - `(i)`：コードが `stdin` を読んだ。ConSh は、標準入力の次の要素を文字列にして（LazyList は OFS でつなぐ）、`outputStream` に二重引用符の 1 行で書く。終わりなら `end` の行を書く（段階 5 では閉じていたが、段階 6 で閉じることの意味を変えた）。読み出し位置は、組み込みコマンド `stdin` と同じく進む。
        - wasm 版の `outputStream` は、書くとホストの `host_js_input` に、閉じると `host_js_input_end` に届く fd である。
        - コードは `new Function("stdin", "return (" + code + ")")` で評価する（大域スコープで、strict でない）。今の評価は 2.1 の 3 にある。
    - 段階 6 で、イテレータを足した。結果がイテレータ（配列と文字列以外の反復可能なもの。ジェネレータ、Set、Map も）なら、ホストは `(r t)` を書く。ConSh が `outputStream` に `n` の行を書くたびに、ホストは次の要素を `(t 要素)` で書く。終わりなら `(d)`、例外なら `(e "...")` を書く。要求された分だけ進めるので、終わらないジェネレータも、読む分だけ要素を作る。
        - ConSh が `outputStream` を閉じたら、読み手がいなくなったとして、ホストは呼び出しを止め、イテレータの `return()` を呼ぶ。その間の `console` の出力（ジェネレータの `finally` など）は、ホストの標準出力と標準エラー出力に直接書く。
        - 出力する形では、2 つの Stream を開いた Stream の表（`Open_track`）に入れてから要素を読む。`| head` などで読み手が先に終わってこの VProc が止められると、表から閉じられ、ホストに届く。
        - 変数に入れる形では、要素を読んだときに `n` を書いて受け取る遅延の LazyList にする。最後まで読まれなければ、2 つの Stream は開いたまま、ホストの呼び出しも終わらないまま残る（当面の制限）。
    - ネイティブ版（Python と Rust の VM）の `__JS_call__` は、引数を確かめた後、`onError` に進む（`__JS_call__: JavaScript is not available`）。
5. **モジュールの読み込み**
    - ConSh のモジュール（今は約 7.0 MB）は、ネイティブで bin/includer と bin/resolver で作っておき（build.sh）、静的なファイルとしてページと一緒に置く。
    - スクリプトの構文解析は、ConMa 版の構文解析（consh_parser.se。今は bin/consh の既定でもある）を使う。
    - Rust の VM はモジュールの読み込みに約 130 MB を使う。wasm のメモリでも足りるか、段階 1 で確かめる。

### 2.3. ファイルの置き場所

```
consh/js/
  PLAN-ja.md        この文書
  README-ja.md      使い方
  build.sh          VM を wasm32 でビルドし、web/ に wasm、conma.js、ConSh のモジュールを置く
  conma.js          wasm の読み込み、import する関数、conma_run の駆動（ブラウザ、Node.js、jsc で共通）
  vm-node.js        Node.js で VM を動かす（vm/target/release/conma と同じ引数）。ファイルは、このプロセスの
                    ファイルシステムのものを読み書きする（段階 7 の 1。hostFs）
  vm-jsc.js         jsc で VM を動かす（同じ引数に、今のディレクトリ、環境変数、ディレクトリとファイルの名前のファイルを足す）
  vm-wasm.sh        node が動けば vm-node.js、動かなければ vm-jsc.js を起動する（CONMA_WASM_HOST=node か jsc で選べる）。
                    jsc では、標準入出力と終了コードをつなぎ、今のディレクトリの下と /tmp の下の
                    ディレクトリとファイルの名前を渡す（中身は VM が開いたときに jsc が読む）。test/vm.sh が CONMA_VM=wasm で使う
  wasm/
    Cargo.toml      crate conma-wasm（2.2 の 1）
    src/lib.rs      export する関数（conma_start、conma_run など）
    src/sys.rs      仮想の OS（2.2 の 2）
  serve.py          web/ を出す HTTP サーバー（ディレクトリの一覧、Cache-Control: no-cache。3.14）
  web/              （段階 3）HTTP で開く（serve.py）
    index.html      端末の画面（出力と入力の行、「ファイルを置く」「スクリプトを実行」「中断」）
    consh.js        画面、入力、Ctrl-C、js の評価（conma.js の JsEngine）。Worker を起動する（3.19）
    consh-core.js   VM、ファイル（serverfs.js）、組み込みコマンド（COMMANDS）。Worker でもページでも動く
    consh-worker.js Worker の入口。js の呼び出しをページに渡し、__JS_value__ は SharedArrayBuffer で待つ
    conma.wasm、conma.js、consh.module.se   build.sh が置く（リポジトリには入れない）
  test/             （段階 4）ConSh の test/stepN と同じ形の js のテスト。stepN/test.sh は、CONMA_VM=wasm で
                    consh/test/common.sh の run を使う。run_all.sh ですべて実行する
```

VM の Rust のコードのうち、ネイティブ版と共通のもの（sys.rs の関数を通すようにした変更、スケジューラ、段階 4 の js のプリミティブ）は vm/src に置く。ConSh の `js` の組み込みコマンドは、consh/include/consh_eval.se（参照版は consh/reference/consh_eval.se）と consh/rust/src/eval.rs に置く（2.1 の 10）。

### 2.4. 当面の制限

- 外部コマンド、`&` で起動する外部コマンド、`kill` は使えない（ConSh の関数と組み込みコマンド、`&` の関数のジョブは使える）。
- ファイルはページを閉じると消える。
- 行の編集と履歴は、ブラウザの入力欄のものだけである。
- 2.1 の 8 の、同時に動く `js` の `console.log` の区別（結果は変数名を書けば区別できる）。
- テストは Node.js（vm-node.js）で動かす。vm-wasm.sh は、node が動けば Node.js を使い、動かなければ macOS の JavaScriptCore のシェル（jsc、vm-jsc.js）を使う。jsc でのテストは 3.3 の後は行わない（2026/11/8 から）。どちらでも、標準入力は、VM がそれを求めたときにだけ、読めた分ずつ渡す。
    - Node.js では、段階 7 の 1 から、VM はファイルをホストのファイルシステムで読み書きする（`conma_use_host_fs`。`host_fs_open` などの import を `fs.*Sync` で作る）。パスは OS が解決するので、`..` とシンボリックリンクもネイティブ版と同じである。それより前は、仮想ファイルシステムを通して読み（ファイルは開いたとき、ディレクトリは初めて一覧したときに、`host_file_needed` と `host_dir_needed` でホストに求める）、書いたファイルはメモリの中にだけあった。
    - jsc にはバイト列の入出力も終了コードもないので、vm-wasm.sh が 16 進数の行に直してつなぐ。jsc はディレクトリを調べられないので、VM に見えるのは、vm-wasm.sh が渡す名前（今のディレクトリの下の深さ 3 までと、/tmp の下の深さ 2 までのディレクトリ）と、VM が開いたファイルだけである。
    - jsc とブラウザでは、VM が書いたファイルは、実際のファイルシステムには書かれない。
    - 仮想ファイルシステムでは、パスは、ネイティブ版と同じく、途中のディレクトリを 1 つずつ確かめて解決する（`""` は ENOENT、`work/none/..` は `none` がなければ ENOENT、`file/x` は ENOTDIR）。ただし jsc では、途中のディレクトリがわからなければ、あるものとする（`conma_assume_dirs`）。
    - 仮想ファイルシステムにはシンボリックリンクがない。
- ConMa 版の ConSh には `source` がないので、ブラウザでは「スクリプトを実行」が、スクリプトの各行を入力として渡す。
- wasm32 のターゲットが必要である（`rustup target add wasm32-unknown-unknown`）。

### 2.5. JS の文とシェルの変数の JS の値（2026/11/9 に決めた仕様。作った結果と、仕様から変えたところは 3.15 と 3.16）

JavaScript のある版（ブラウザと Node.js の wasm 版）では、`js` と書かなくても JavaScript を書けるようにする。先頭の語がコマンドでない文は、行全体を JavaScript のコード（**JS の文**）とする。また、シェルの変数に JavaScript の値をそのまま入れられるようにし、`$` で参照したときに文字列（かリスト）にする。

決めたこと（ユーザーの選択）：

- 対話モードだけでなく、スクリプトでも JS の文を使える。
- JS の文が行の終わりで閉じていなければ（括弧、文字列など）、次の行も続きとして読む。
- JS の文の中で代入した値（`let` などの宣言の有無によらない）は、次の文でも使えるように残す。残す先は、JavaScript の大域変数でなく、シェルの変数とする（下の 4、5）。
- 文法の規則は単純にする：先頭の語が関数やコマンドでなければ、JS の文とする。代入の書き方を制限する規則（一時、`let` のない `n = ...` を構文エラーにする案があった）は設けない。
- `n=1+2`（空白のない `名前=` で始まる文）は、今までどおりシェルの変数への文字列の代入とする（`n` は `"1+2"`）。JS で計算するときは `n = 1+2` か `let n = 1+2` と書く。
- 大域変数を置き場所にしないのは、ConSh の変数が写しで動くためである。関数は定義した時点の変数の写しを持ち、パイプラインの要素、`$( )`、`&` も写しで動き、関数の中の変数はその呼び出しの局所変数である（FOR-BASH-USERS.md の Functions）。大域オブジェクトに置くと、これらがすべて共有になり、ネイティブ版と動きが変わる。また、ブラウザでは `location`、`name`、`status` などの大域変数と名前がぶつかる。

1. **有効にする版**
    - JavaScript のある版だけで有効にする。ConSh は、起動したときに `__JS_call__` が使えるかで決める。
    - ネイティブの VM（Python 版、Rust 版）で動く ConMa 版と、Rust 版の ConSh は、今と変わらない。Python の consh_parse と Rust 版の構文解析には、JS の文の節を足さない。
2. **JS の文の判定**（構文解析のとき）
    - 文の先頭（行の始め、`;` の後、`{`、`then`、`do` などの後）で、予約語でも前置の変数定義（`識別子=`）でもないとき、先頭の語を見る。
        - 先頭の語は、文の始めから、空白、改行、`;`、`|`、`&`、`<`、`>`、`)`、`}` の手前までの文字とする。`console.log("hi")` の先頭の語は `console.log("hi")` 全体、`ls -l` では `ls`、`$(true)` の中では `true` である。`)` や `}` で始まる文は、今までどおりシェルの構文エラーになる。
    - 次のどれかなら、今までどおりシェルの文とする。
        - 先頭の語が `$`、`'`、`"`、`\` で始まる（展開と引用符。`$cmd args`、`"cmd with space"`）。
        - 先頭の語の、`(`、`[`、`{`、引用符、`` ` ``、`=` より前の部分が `/` を含む（パスのコマンド。`./script`、`/bin/ls`）。`console.log("a/b")` は JS の文である。
        - 先頭の語が ConSh の予約語（`if`、`for`、`function`、`return`、`break`、`continue`、`export`、`then`、`do` など）か、`reset`、`shift` である。JavaScript の `if (...)`、`for (...)`、`function f () {...}` も、文の始めに書くとシェルの構文になる。JS で書くときは、ブロック（`{ for (...) ... }`）にするか、`let f = function () {...}` と書く。
        - 先頭の語が組み込みコマンド（`echo`、`js` など）か、ホストの組み込みコマンド（`CONSH_HOST_BUILTINS`）の名前である。
        - 先頭の語が関数の名前である。関数は、解析する時点のシェルに定義されているもの（対話モード）と、解析するテキストの中で `function 名前` と定義しているもの（呼び出しより後で定義していてもよい）とする。
        - Node.js 版では、先頭の語が、解析する時点の PATH にある外部コマンドである。
    - そうでなければ、JS の文とする。
        - `n = 1`、`let n = 1`、`n += 1`、`console.log(x)`、`(async () => {...})()`、`[1, 2].map(f)`、`{ ... }` は JS の文である。
        - `x=1`、`n=1+2` は、今までどおりシェルの変数定義である（4 で、JS の値でなく String を入れる。`n` は `"1+2"`）。`n==1` も `名前=` で始まるので、シェルの変数定義（`n` に `=1`）になる。
    - スクリプトでは、判定はスクリプト全体を解析するときに一度だけ行う。実行中に定義した関数や、実行中に PATH に置いたコマンドは、すでに JS の文と決まった文を変えない。対話モードでは、コマンドを読むたびに、そのときのシェルで判定する。
3. **JS の文の範囲**
    - JS の文は、先頭から、括弧（`()`、`[]`、`{}`）、文字列（`'`、`"`）、テンプレート（`` ` ``、`${ }` の入れ子を含む）、コメント（`//`、`/* */`）、正規表現リテラルの外にある改行までとする。正規表現リテラルは、`/` の前が、文の始め、`(`、`,`、`=`、`:`、`[`、`!`、`&`、`|`、`?`、`{`、`}`、`;`、演算子、`return` などのキーワードのときとする。
    - JS の文の中の `;`、`|`、`&`、`>` などは、すべて JavaScript のものである。JS の文は、パイプラインの要素にもならず、リダイレクトも付かない。標準入出力を変えるときは、関数の中に書くか、`js` を使う。
    - シェルの構文の中（関数の本体の `{ }`、`$( )`）では、対応する括弧のない `}` か `)` の手前でも終わる。`function f () { console.log(1) }` と 1 行で書ける。
    - 閉じていないまま入力が終わったら、構文エラー `unterminated JavaScript` とする。対話モードでは、これを入力が足りないとみて（`Parse_incomplete`）、PS2 で続きを読む。
    - `if 文; then` の条件の文を JS の文にすると、`; then` も JavaScript のコードになる。条件に JS を使うときは、`then` を次の行に書くか、`js` を使う。
    - 構文木では、新しい節 `(jsstmt 位置 コード 識別子 ...)` とする。識別子は、コードに現れる識別子の名前（文字列、テンプレート、コメントの外。重複なし）で、下の 5 で使う。
4. **シェルの変数の JS の値**
    - シェルの変数の値に、今の String、LazyList、関数値に加えて、**JS の値**を足す。VM の中には、値の番号（id）だけを置く。オブジェクトと関数は、ホストの JavaScript の表に置き、番号は `#n` とする。数、真偽値、BigInt、`undefined`、`null` は、表に入れず、番号を値そのもの（`number:3`、`boolean:true`、`bigint:5`、`undefined`、`null`）にする。文字列は、JS の値にせず、ふつうの String として入れる（JavaScript からも同じ文字列に見える）。
    - `$x`、`${x}`、`"$x"` などで参照したとき、2.1 の 6 の規則で変換する。
        - 文字列はそのまま、数値、真偽値、BigInt は `String()` の文字列とする。
        - 配列はリスト（LazyList）とし、要素が配列なら入れ子にする。
        - `undefined` と `null` は空文字列とする。
        - そのほかのオブジェクトは `JSON.stringify` の文字列とし、例外なら（循環参照など）`String()` の文字列とする。
        - 関数は、関数値（ConSh の PLAN-ja.md の段階 20）とする。`$f 1 2` は、`js` と同じく、その関数を引数を付けて呼び、結果を出す。`$f` を文字列として使うと、関数値と同じエラーになる。
    - 変換は、`$x` で読むたびに行い、結果を覚えない。プリミティブは ConSh が番号から直し、オブジェクトと関数は、VM の同期のプリミティブ `__JS_value__`（docs/PRIMITIVES.md）でホストに直してもらう（wasm の import は同期なので待たない）。JavaScript の側でオブジェクトを変えれば（別の名前の変数を通してでも、`setTimeout` で後からでも）、次の `$x` に表れる。大きなオブジェクトを何度も `$x` で読めば、そのたびに `JSON.stringify` の費用がかかる。
    - 外部コマンドの引数、`export x` で環境変数に入れる値も、同じ規則で文字列にする。
    - 変数の写し（関数、子の VProc）は、参照を写す。JavaScript のオブジェクトは、JavaScript と同じく共有され、束縛だけが写しになる。
    - `x=1` などシェルの変数定義は、今までどおり String を入れる。JavaScript からは文字列の `"1"` に見える。数にするには `Number(x)` と書く。
    - `$( )` の値は、今までどおり出力の行の LazyList である。中が JS の文でも、値は出力を通した文字列になる。`n=$(1+1)` の `n` は `"2"` 1 つのリストで、JavaScript からは配列の `["2"]` に見える（`n + 1` は `"21"`）。JS の数として入れるときは、`$( )` を通さずに `n = 1+1` と書く。
    - `js 変数名= コード` も、結果を JS の値のまま変数に入れる（2.1 の 6 を変える。3.17）。文字列は String、イテレータは今までどおり遅延の LazyList で、そのほかは JS の値になる。`$` で参照したときの値は、関数（エラーでなく、呼べる関数値になる）を除き、今と同じである。Promise は、今までどおり解決した値を入れる。
5. **JS の文の評価**
    - 評価は、2.1 の 3 の `js` と同じ順にする。まず式として読み、読めなければ `await` を書ける式（`return (コード)` の async 関数）、それも読めなければ async 関数の本体とする。式の値が関数でも、呼ばない（引数がない）。
    - その場で見えるシェルの変数のうち、識別子（3）に現れる名前のものを、同じ名前の JavaScript の変数として見せる。
        - JS の値はそのまま（同じオブジェクト）、String は文字列、LazyList は配列にして渡す。終わらない LazyList は、`$nat` と同じく終わらない。
        - 位置パラメータ（`$1`、`$@`）は、識別子にならないので渡さない。環境変数は渡さない。
    - コードは、シェルの変数を表すオブジェクトを `with` のスコープにして評価する。名前は、シェルの変数、JavaScript の大域変数の順に探す。
        - 渡したシェルの変数への代入（`n = n + 1`、`n += 1`、`n++`、ループの中の `total += x`）は、そのシェルの変数への代入になる。
        - シェルの変数でない名前への代入（`n = 3`）は、新しいシェルの変数になる。その名前が大域変数にあっても（ブラウザの `name`、`status`、Node.js の `process`）、シェルの変数にする。ホストは、コードの中で代入される名前（`名前 =`、`名前 +=` など、`名前++`、`++名前`、宣言。`.` の後ろは除く）を字句から拾い、評価の前にシェルの変数として用意する。大域変数を変えるには `globalThis.name = ...` と書く。大域変数のプロパティへの代入（`document.title = ...`、`location.href = ...`）は、今までどおりである。
        - トップレベルの宣言も、シェルの変数への代入に書き換えて評価する：`let x = e`、`const x = e`、`var x = e` は `x = e`、`function f () {...}` は `f = function f () {...}`、`class C {...}` は `C = class C {...}`。分割代入（`let {a, b} = o`）も同じである。宣言は何度でも書け（`let n = n + 1`）、`const` も後で再代入できる。
        - 代入した値は、文字列なら String、そのほかは JS の値として、シェルの変数に入る。
        - 書き込みは、JS の文が終わったとき（Promise を待つなら解決したとき）に、その文が代入した変数だけを、今のシェルの局所変数に入れる（代入していない変数を書き戻すと、`await` の間にほかの VProc が入れた値を消すことがある）。関数の中ならその関数の局所変数、パイプラインの要素、`&`、`$( )` の中ならその写しである（2.1 の 6 と同じ）。その後に非同期に代入しても（`setTimeout` など）、シェルの変数は変わらない。
        - 例外のときも、それまでの代入は入れる。ただし、打ち間違えたコマンドとしてシェルの文を実行するとき（7）は、何も入れない。
    - `stdin` と `console` は、`js` と同じ（2.1 の 7、8）。`args` は空の配列とする。
6. **出力**
    - 式の値を、2.1 の 5 の規則で標準出力に出す（Promise は待ち、イテレータは要素ごと、`undefined` と `null` は何も出さない）。
    - ただし、式が代入（`=`、`+=` など。括弧の外の一番外側の演算子）のときは出さない。`n = 3` は何も出さず、`n` は `3` を出す。宣言（`let n = 3`）も何も出さない（文なので）。
    - async 関数の本体として動かした文（宣言、`if` など、2 つ以上の文）は、`return` がなければ何も出さない。`let x = 1; x + 1` は何も出さないので、`console.log` を使う。
7. **エラー**
    - JS の文が、JavaScript としてコンパイルできない（構文エラー。`sl -la`、`nosuch | wc -l`）か、先頭の名前がシェルの変数でも JavaScript の大域変数でもない（`nosuchcommand`、`no-such-file` の `no`。ReferenceError）ときは、打ち間違えたコマンドとみて、同じテキストをシェルの文として実行する。コマンドが見つからなければ、今までと同じメッセージ（`consh: ファイル:行:桁: 先頭の語: No such file or directory: 先頭の語`）を出し、`$?` を 127 にする。`nosuch | wc -l` のパイプラインも、今までと同じく動く。
        - シェルの文としての解析は、JS の文を解析するときに行っておく（構文木の節に、代わりのコマンド行として持つ）。シェルの文として解析できなければ、JavaScript のエラーを `js: 名前: メッセージ` で出し、`$?` を 1 にする（`z = )`）。
        - wasm 版で consh/test を流しても、打ち間違えたコマンドのケースは、今までと同じ出力になる。
        - シェルの変数か環境変数の `CONSH_JS_DEBUG` が空でなければ、シェルの文として実行する前に、JavaScript としてのエラーを `js: メッセージ (run as a shell statement)` で出す（3.20）。
    - そのほかの例外と、拒否された Promise は、`js` と同じく `js: 名前: メッセージ` を出し、`$?` を 1 にする。
    - そうでなければ、`$?` を 0 にする。
8. **例**

    ```
    n = 3                              # JS。シェルの変数 n に、JS の数 3
    n * 2                              # 6
    echo $n                            # 3
    n += 1                             # n は 4（代入なので何も出さない。let n = n + 1 も同じ）
    n=1+2                              # シェルの変数定義。n は文字列の "1+2"
    n=$(1+1)                           # $( ) の中の 1+1 は JS の文。n は出力の行のリスト（JS では ["2"]）
    n = 3
    o = {a: 1, b: [1, 2]}
    o.a + n                            # 4
    echo $o                            # {"a":1,"b":[1,2]}
    xs = [1, 2, 3].map(i => i * n)
    for x in $xs; do echo $x; done     # 3 6 9
    name=world                         # シェルの変数定義（String）
    console.log(`hello ${name}`)       # hello world
    total = 0
    for (const x of xs) {
      total += x
    }
    echo $total                        # 18
    function f (a) { n=2; echo $n }    # シェルの関数。n は局所変数
    f 1; echo $n                       # 2、3
    let r = await fetch("names.txt")   # ブラウザ
    nosuchcommand                      # consh: ...: nosuchcommand: No such file or directory: nosuchcommand
    ```
9. **当面の制限**
    - ホストの JS の値の表は、どの変数も持たなくなった値を消す（3.18）。ただし、VM がその変数の値を捨てたときなので、関数の局所変数や、`for` で途中まで読んだ遅延の LazyList のように、VM の中の循環に残るものは、循環の回収まで、あるいはずっと残ることがある。
    - シェルの関数を JavaScript から呼ぶことはできない。JS の値の関数は、`$f 1 2` で呼べるが、`f 1 2` では呼べない（`f` は関数の名前でないので JS の文になり、構文エラーからシェルの文になって、コマンドが見つからない）。

## 3. 段階

各段階は、そこまでのテストが通り、ネイティブ版のテスト（test/ の両方の VM と consh/test）がすべて通ることで終わりとする。

| 段階 | 内容 | 確かめ方 |
|---|---|---|
| 1 | VM を wasm32 でビルドする。OS 層を 2.2 の 2 のように分け、`conma_run` を export する | `CONMA_VM=wasm test/run_all.sh` で、OS のプロセスとファイルを使わないテストが、ネイティブ版と同じログになる。ConSh のモジュールを読み込めて、メモリが足りる |
| 2 | ConSh を wasm 版の VM で動かす。仮想ファイルシステム、パイプ、標準入出力、`--conma-parser` の経路 | consh/test のうち外部コマンドを使わないケースが、jsc で同じログになる |
| 3 | ブラウザの画面（web/index.html）。対話モード、Ctrl-C、スクリプトの読み込み | ブラウザで `echo`、関数、パイプ、`$( )`、`&` の関数のジョブが動く |
| 4 | `js` の組み込みコマンド（2.1 の 1〜6、9、10。`js [変数名=] コード {引数}` の見分け方と、変数に入れる値を含む）と `__JS_call__`、`__JS_next__`。Promise を待つ間、ほかの VProc が動く | consh/js/test に `js` のテスト。変数名のある形とない形、`js sum=` だけのエラー、`js 'x=1'` が代入式になることを確かめる。ネイティブ版では 2.1 の 10 のエラーになるテストを consh/test に足し、ConMa 版と Rust 版の両方で通る |
| 5 | `stdin` と `console`（2.1 の 7、8） | `cmd \| js '...'`、変数名がある場合とない場合の `console.log` のテスト |
| 6 | イテレータの結果を 1 つずつ出す（2.1 の 5 の最後と、6 の遅延の LazyList） | 無限のジェネレータを `head` 相当の関数で途中まで読むテスト |
| 7 | Node.js のホストで、(1) ファイルを実際のファイルシステムで読み書きする、(2) `node:child_process` で外部コマンドを起動する（どちらも済み） | (1) test/ のファイルとシンボリックリンクのケース。(2) consh/test の全ケース |
| 8 | JS の文とシェルの変数の JS の値（2.5。済み。3.15〜3.17）。(1) VM の JS の値の型と同期の変換、(2) 構文解析の JS の文と判定、(3) 評価、`with` のスコープ、宣言の書き換え、出力とエラー、(4) `js 変数名=` を JS の値にする | consh/js/test に JS の文のテスト（判定、続きの行、変数の受け渡しと写し、出力、エラー、対話モード）。`CONMA_VM=wasm CONSH_IMPL=conma consh/test/run_all.sh` の差が今と同じ。ネイティブ版のテストがすべて通る |

段階 4 で、ConSh の PLAN-ja.md（1.2 の組み込みコマンド）、README-ja.md、FOR-BASH-USERS.md に `js` を書く。

### 3.1. 段階 1 の結果（2026/11/8）

- `consh/js/build.sh` で VM を wasm32 にビルドできる（約 700 KB）。vm/ の変更は 2.2 の 2 と 3 のとおりである。
- `CONMA_VM=wasm test/run_all.sh` で、test/ の 39 のステップのうち、次の 17 と selfhost3〜7 は、ネイティブ版と同じログにならない。どれも、wasm 版にないもの（OS のプロセス、実際のファイル、bin/includer を起動する `__VProc_spawn__`）を使うケースで、エラーのメッセージ（`__OS_spawn__: not supported: ...`、`No such file or directory`、`__VProc_spawn__: file not found: ...`）で止まる。
    - プロセス：step10、step11、step13 の case7、step15、step26、step31、step32
    - ファイル：step14、step16、step29、step33、step37、selfhost3〜7（ソースを読む）
    - `__VProc_spawn__`：step17、step20、step21、step22、step23、step24
- そのほかのケースは、すべてネイティブ版と同じログになる（標準入力を読む step9 の case5、step13 の case2、case3、step20 の case6 を含む）。
- ConSh のモジュール（約 5.8 MB）を読み込み、対話モード（`interactive - stdin`）で、関数、`$( )`、`for` が動く。jsc の最大メモリは約 140 MB で、足りる。

### 3.2. 段階 2 と 3 の結果（2026/11/8）

- 段階 2：仮想ファイルシステムに `/dev/null` を足した。VM が持っていないファイルを開くと、ホストに求める（`host_file_needed`。jsc は実際のファイルを読んで渡す）。
    - `CONMA_VM=wasm CONSH_IMPL=conma consh/test/run_all.sh` で、ネイティブ版と違うところは、どれも外部コマンド（`cat`、`printf`、`sh`、`sleep` など）を使うところで、`consh: ...: not supported: <コマンド>` で止まる。
    - `--conma-parser`（Python の consh_parse を使わず、ConMa 版の構文解析で読む経路）でも、Rust 版の VM と同じ出力になる。
    - `CONMA_VM=wasm test/run_all.sh` では、ファイルを読む step29、step37、selfhost3〜5 が通るようになり、step14 の差はプロセスを使うケースだけになった。残る差は、プロセス、`__VProc_spawn__`、シンボリックリンク（step16 の work/up、step23、selfhost6）、vm-wasm.sh が渡さないディレクトリ（selfhost6、selfhost7 の `../../include`）による。
- 段階 3：`consh/js/build.sh` の後、consh/js/web を HTTP で開くと、ConSh の対話モードが動く。ブラウザで、関数の定義と呼び出し、`$(stdin < names.txt)`、パイプ、`&` の関数のジョブと `wait`、ファイルへのリダイレクト、終わらない関数を「中断」（Ctrl-C）で止めること（`$?` は 130）を確かめた。外部コマンドは `not supported` になる。
    - VM は、ページが固まらないように、約 25 ms ずつ `setTimeout` で区切って動かす。入力を待つ間（`conma_run` が IDLE を返す間）は動かさない。
    - 「スクリプトを実行」は、ファイルを選ぶ画面を自動では操作できないので、まだ確かめていない。

### 3.3. Node.js のホスト（2026/11/8）

- arm64 版の Node.js（v26.13.0、Homebrew）を入れたので、vm-node.js を足し、vm-wasm.sh は node が動けばそれを使うようにした。
- 仮想 OS に、ディレクトリを初めて一覧するときにホストに求める `host_dir_needed` を足した。また、ファイルを書くときに親のディレクトリがなければホストに求め、パスは途中のディレクトリを確かめて解決するようにした（2.4）。
- `CONMA_VM=wasm test/run_all.sh` と `CONMA_VM=wasm CONSH_IMPL=conma consh/test/run_all.sh` を、Node.js と jsc（`CONMA_WASM_HOST=jsc`）の両方で流した。ネイティブ版と違うのは、どちらでも次のところだけである。
    - 外部コマンドとプロセス（`not supported`）、`__VProc_spawn__`（`file not found`）
    - シンボリックリンク（step16 の work/up、step23、selfhost6 と selfhost7 の link.se）
    - jsc だけ：途中のディレクトリをあるものとするので、step16 の `work/none/..` がエラーにならない
- ブラウザの画面も、作り直した wasm で動く。

### 3.4. 段階 4 の結果（2026/11/8）

- `js [変数名=] コード {引数}` を、ConMa 版の ConSh（consh/include/consh_eval.se の `Builtin_js`。参照版は consh/reference/consh_eval.se）に足した。2.1 の 1〜6、9、10 のとおりに動く。Rust 版の ConSh（consh/rust）の `js` は、同じ順に引数を確かめた後、`js: JavaScript is not available` を出して `$?` を 127 にする。
- テスト
    - consh/js/test/step1（wasm 版の VM、Node.js）：case1 は結果の出し方（値、配列、入れ子、`undefined`、`null`、オブジェクト、BigInt、改行とタブを含む文字列、代入式のコード）、case2 は変数に入れる形と引数（`$list` と `${list}`）、関数の中の変数、case3 は Promise と、待つ間にほかの VProc が動くこと、case4 はエラー（例外、構文エラー、関数でない値への引数、関数の結果、循環する配列、`js sum=`、`js` だけ）と、エラーのときに変数を変えないこと、`builtin js`。
    - consh/test/step17（JavaScript のない版）：ConMa 版（Python 版と Rust 版の VM）と Rust 版の ConSh で同じログになる。
    - test/step40（`__JS_call__`）：Python 版と Rust 版の VM で同じログになる。wasm 版では case1 が Stream を得るので違う。
- 構文エラーのメッセージ（`SyntaxError: Unexpected token ')'` など）は JavaScript の処理系によって違う。consh/js/test/step1 の case4 のログは Node.js v26 のものである。

### 3.5. 段階 5 の結果（2026/11/8）

- `stdin`（2.1 の 7）と `console`（2.1 の 8）を足した。`stdin` は非同期のイテレータで、`stdin.lines()` は全部の配列の Promise である。
- `console` の出力は、そのとき同期に動いているコードの呼び出しのものにする。Promise を待つ間などでそれがなければ、終わっていない呼び出しが 1 つだけならそれのものにする。2 つ以上ならホストの標準出力と標準エラー出力に直接書き、1 つもなければ元の `console` に渡す。
- テスト：consh/js/test/step2（Node.js）。case1 は `stdin`（数える、`lines()`、変数リストとファイルからの入力、空の入力、途中で `break` したときに残りを次のコマンドが読むこと、`stdin` を読まないコードは入力を読まないこと）、case2 は `console`（各メソッド、文字列でない引数、変数に入れる形、Promise を待つ間の出力、リダイレクトと変数リスト、例外の前の出力）、case3 は同時に動く 2 つの呼び出し。

### 3.6. 段階 6 の結果（2026/11/8）

- イテレータの結果を、要素ごとに出し、変数には遅延の LazyList として入れるようにした（2.2 の 4）。
- テスト：consh/js/test/step3（Node.js）。case1 はイテレータの種類（ジェネレータ、ジェネレータを返す関数、Set、Map、非同期のジェネレータ、途中の例外、空のイテレータ）、case2 は必要な分だけ作ること（変数リスト、読み手が先に終わるパイプ、`return()` と `finally`、`stdin` を読むジェネレータ）、case3 は遅延の LazyList（作った要素を覚えておくこと、途中の例外）。
- 終わらない LazyList の変数は、`${nat}` のように 1 つの値として使う。`$nat`（葉ごと）は、ConSh の規則（PLAN-ja.md の 1.2）どおり最後までたどるので、終わらない。`[` などの外部コマンドがないので、`for` を止めるのは `break` だけである。

### 3.7. 段階 7 の 1：実際のファイル（2026/11/8）

- 仮想 OS に、ファイルをホストに任せる形（`Fd::Host`、`host_fs_*` の import、`conma_use_host_fs`）を足し、vm-node.js がそれを `fs.*Sync` で作った。ブラウザは今までどおり仮想ファイルシステムを使う。
- `CONMA_VM=wasm test/run_all.sh` で、selfhost6、selfhost7（シンボリックリンク、`../../include`）も通るようになった。ネイティブ版と違うのは、プロセスと `__VProc_spawn__` を使うケースと、予定どおりの step40 だけである。consh/test は、外部コマンドを使うところと、予定どおりの step17 だけが違う。
- ファイルを本当に書くようになったので、consh/js/test/step2 の case1 は、テストのディレクトリでなく一時ディレクトリにファイルを作るようにした。

### 3.8. 段階 7 の 2：外部コマンド（2026/11/8）

- VM：wasm 版の `OSProcObj` は、`std::process::Child` の代わりにホストのプロセスの番号を持つ（`cfg(not(unix))`。ネイティブ版は変わらない）。`__OS_spawn__`、`__OS_poll__`、`__OS_kill__` は、仮想 OS の `spawn`、`proc_poll`、`proc_kill` から、ホストの `host_proc_spawn`、`host_proc_poll`、`host_proc_kill` を呼ぶ。
    - 子の標準入出力は、種類で渡す：ホスト自身の fd（0、1、2）、ホストのファイルの fd、VM のパイプの読む端と書く端（ホストは自分用に dup した VM の fd を `conma_read` と `conma_write` で読み書きし、閉じる）、`/dev/null`。
    - ホストは、プロセスが終わると `conma_child_event` を呼ぶ。スケジューラは、プロセスだけを待つ間、シグナルの数が変わらなければプロセスを調べ直さないので、この数をシグナルに加える。
    - `__OS_trap_interrupt__` は `host_trap_sigint` を呼び、ホストはそれから SIGINT を `conma_interrupt` に替える。それまでは、ネイティブ版と同じく SIGINT で終わる。
- Node.js（vm-node.js）：`child_process.spawn` で起動する。コマンドは、execvp と同じく、渡された環境変数の PATH から探す（見つからなければ ENOENT、実行できなければ EACCES を `onError` に渡す）。
    - VM のパイプと子の間は、ホストが写す：子の出力は `data` のたびに VM のパイプに書き、子の入力は `conma_run` の後ごとに VM のパイプから読んで書く。VM の読み手がいなくなったら（EPIPE）、ネイティブ版で書き手が受ける SIGPIPE を、その子に送る。
    - main のループは、プロセス、JavaScript、標準入力のどれかを待つ間、イベントループを回す（標準入力も、待つ間は非同期に読む）。終わるときは、動いているプロセスを待たない（ネイティブ版と同じ）。
    - 長い LList の解放などで再帰が深くなるので、vm-wasm.sh は、`ulimit -s` を上限まで上げ、`node --stack-size` をその 9 割にする（consh/test/step14 の case1）。
- `CONMA_VM=wasm` で test/ と consh/test を流すと、ネイティブ版と違うのは次だけになった。
    - test/：`__VProc_spawn__`（bin/includer を起動する。step10、11、14、15、17、20〜23）と、予定どおりの step40。
    - consh/test：予定どおりの step17 と、step8（子の標準出力がパイプかを調べるケース。Node.js は子の標準入出力を UNIX ドメインソケットでつなぐので、パイプでなくソケットに見える。Node.js では名前のないパイプを作れないので、当面の制限とする）。
- 当面の制限：Node.js のホストでは、対話モードの端末とプロセスグループ（`__OS_spawn_detached__` の前景との区別、Ctrl-C を子だけに送ること）は、ネイティブ版と同じにはならない。

### 3.9. ブラウザの cat（2026/11/8）

- ブラウザの画面に、JavaScript で作るプロセス（`hostProc`）を足し、`cat` を作った（`web/consh.js` の `COMMANDS`）。
    - 最初は外部コマンドとして起動していたが、ブラウザ版では組み込みコマンドにすることにした。ConSh（ConMa 版）は、環境変数 `CONSH_HOST_BUILTINS` にある名前を組み込みコマンドとし、ホストのコマンド `builtin:名前` を起動する（ConSh の PLAN-ja.md の 1.2）。ページはこの変数に `cat` を設定し、`builtin:cat` だけを起動できるようにする。ネイティブ版ではこの変数がないので、外部の `cat` は隠れない。
    - ページは、`conma.wasm`、`consh.module.se`、`names.txt` を毎回サーバーに確かめて読み（`cache: "no-cache"`）、`conma.js` と `consh.js` も毎回読み直す。build.sh の後に、ブラウザのキャッシュの古いモジュールで動いてしまったためである。
    - 標準入力は、端末（ホストの fd 0）、VM のパイプ、仮想ファイルシステムのファイル（`< file`）から読み、標準出力と標準エラーは、端末、パイプ、ファイル（`> file`）に書く。仮想ファイルシステムのファイルの fd も、パイプの端と同じく子に渡せるようにした（`sys::spawn`）。
    - 端末から読むプロセスが動いている間は、入力欄の行と Ctrl+D をそのプロセスに渡す。Ctrl+D は、端末と同じく、その 1 回の読み込みの終わりにする。Ctrl+C は、そのプロセスを止め（SIGINT）、ConSh にも割り込みを渡す。
- ConSh 自身が端末から読んでいるときの Ctrl+D は、今までどおり、ConSh の標準入力の終わりになる（ネイティブ版の ConMa 版と Rust 版も、端末で `stdin > file` の後に Ctrl+D を押すと、シェルも終わる）。

### 3.10. wasm 版の __VProc_spawn__（2026/11/8）

- `__VProc_spawn__` は、ソースからモジュールを作るのに、ネイティブ版では `conma build`（VM に埋め込んだ ConMa の front end）を別のプロセスで動かす。wasm 版では、vm/src/prim/vprocs.rs の `load_program` のうち、モジュールの文字列を作るところ（`build_module`）を `cfg(not(unix))` で分け、ホストの `host_build_module` に頼む。ファイルがあるかの確認も、wasm 版では `sys::is_dir` で行う。ネイティブ版は変わらない。
- vm-node.js は、同じ wasm からもう 1 つ VM のインスタンスを作り、リポジトリの `vm/frontend.module.se` を `conma build` と同じ引数で最後まで動かして、その標準出力をモジュールにする（ファイルは同じ hostFs で読む）。ブラウザにはこれがないので、`__VProc_spawn__` は `includer failed` になる。
- これで、`CONMA_VM=wasm test/run_all.sh` は、予定どおりの step40 のほかは、ネイティブ版と同じログになった。consh/test は、予定どおりの step17 と、step8（子の出力がソケットに見える）だけが違う。

### 3.11. js のコードの文（2026/11/8）

- 式として読めないコードを、async 関数の本体として動かすようにした（2.1 の 3）。テストは consh/js/test/step4（`return`、`args`、`stdin`、`await`、`console`、`return` のない本体、ジェネレータを返す本体、式が先になること、本体の中の例外と拒否、関数の結果）。

### 3.12. ブラウザ版の pwd（2026/11/8）

- ブラウザの画面の `COMMANDS` に `pwd` を足した。`cat` と同じく、ConSh の組み込みコマンドになる（`CONSH_HOST_BUILTINS` は `COMMANDS` の名前から作るので、ConSh は変えていない）。ConSh が子に渡すカレントディレクトリを出すので、`cd` に従う。仮想ファイルシステムにはシンボリックリンクがないので、`-L` と `-P` は同じになる。ほかのオプションと引数はエラー（`$?` は 1）。

### 3.13. ブラウザ版の ls（2026/11/8）

- ブラウザの画面の `COMMANDS` に `ls` を足した（`ls [-1AadF] [file ...]`）。出力が端末でないときの ls と同じく、1 行に 1 つの名前を、名前の順に出す。ファイルの名前が先、ディレクトリの中身が後で、引数が 2 つ以上なら `名前:` の見出しを付ける。`-a` は `.` と `..` と `.` で始まる名前も、`-A` は `.` で始まる名前だけを足し、`-d` はディレクトリそのものを、`-F` はディレクトリの後に `/` を出す。ない名前は `No such file or directory`（`$?` は 1）。
- 仮想ファイルシステムのディレクトリを JavaScript から読めるように、wasm に `conma_list_dir` を足した（conma.js の `listDir`）。エントリの名前と、ディレクトリかどうかを返す（3.14 で、ファイルがサーバーのものになったので、なくした）。

### 3.14. ブラウザ版のファイルをサーバーから読む（2026/11/8）

- ブラウザ版も、Node.js 版と同じく、ファイルをホストに任せる（`conma_use_host_fs`）。hostFs は `web/serverfs.js` で、`/home/user` をページのディレクトリにし、その下のファイルを開くたびに、ディレクトリを一覧するたびに、サーバーから読む。VM はファイルを同期して開くので、同期の XMLHttpRequest を使う（バイト列のまま読むため `overrideMimeType` で x-user-defined にする）。キャッシュはブラウザに任せる。
    - ディレクトリかどうか：`/` のない名前のディレクトリは `/` のある名前にリダイレクトされる（`python3 -m http.server` などと同じ）ので、HEAD の `responseURL` で見分ける。
    - ディレクトリの一覧：`consh/js/serve.py` が `?consh-ls` に JSON で答える（ディレクトリの名前は `/` で終わる）。ほかのサーバーでは、ディレクトリの一覧の HTML のリンクを読む。`python3 -m http.server` は `index.html` のあるディレクトリには一覧を出さないので、serve.py を足した。serve.py は `Cache-Control: no-cache` も付けるので、ブラウザは毎回サーバーに確かめ（変わっていなければ 304）、ディスクで書き換えたファイルがすぐ見える。
    - 書き込み：サーバーには書けないので、書いたファイルはメモリに置き、同じ名前のサーバーのファイルより先に見せる（ユーザーの選択）。`>>` は、サーバーのファイルの中身に足す。「ファイルを置く」もメモリに置く。`/home/user` の外は、`/`、`/home`、`/dev`、`/dev/null` などのメモリのものだけである。
- serve.py は、`build.sh` が `web/` に作るファイルがないか古いときは、`build.sh` を実行するように言って起動しない（終了コード 1）。`web/conma.js` は `consh/js/conma.js` と中身を比べ、`conma.wasm` は vm/src と consh/js/wasm の、`consh.module.se` は include と consh/include の、ファイルとフォルダの時刻と比べる。git pull で新しい `consh.js` と古い `conma.js` になり、ページが `TypeError: vm.useHostFs is not a function` で止まったためである。
- ページのディレクトリを置く場所（`/home/user`）は、ページの URL の `?home=絶対パス` で変えられる（ConSh の `HOME` とカレントディレクトリもそこになる）。`/home/user` にしたのは、前の仮想ファイルシステムのホームの名残であり、`/dev` などのメモリのものとサーバーのファイルがぶつからないためである。`?home=/` では、`/dev` と `/dev/null` が同じ名前のサーバーのファイルより先に見える（書いたファイルと同じ規則）。ブラウザで、`?home=/`（`ls -F`、`cd ..`、`/dev/null`、書き込み）、`?home=/work/site/`（途中のディレクトリはメモリにある）、絶対パスでない値（`/home/user` になる）を確かめた。
- ファイルを ConSh の組み込みコマンド（`cat < file` など、子の標準入出力）に渡すときは、hostFs の fd（`host_proc_spawn` の kind 1）になるので、ページの `hostProc` は、それを `dup` してプロセスが終わるまで持ち、読み書きする。`cat` と `ls` も hostFs を使い、`HOME_FILES` と `conma_list_dir` はなくした。
- ブラウザで、`ls`（ページのファイルがすべて出る）、`ls -F /`、`cat`、`*.txt`、`>`、`>>`（サーバーのファイルに足す）、`cd` でのサブディレクトリ、エラー（ない名前、ディレクトリを `cat`、ファイルの下、ないディレクトリへの `>`）、`/dev/null`、ディスクでのファイルの書き換えと追加がページを読み直さずに見えること、2 回目からの読み込みが 304 になることを確かめた。

### 3.15. JS の文とシェルの変数の JS の値（2026/11/9）

- 2.5 を作った（段階 8）。`js 変数名=` を JS の値にすること（2.5 の 4 の最後）は、3.17 で作った。
    - 構文解析（consh/include/consh_parser.se）：`Ps_statement` の先頭で `Ps_at_js` が先頭の語を調べ、JS の文なら `Ps_js` が範囲を読んで節 `(jsstmt 位置 コード 代わりのコマンド行 名前 ...)` にする。テキストの中の `function 名前` は、解析の前に `Funcs_scan` で集める。JavaScript が使えるかは、run-src と対話モードの始めに `Js_probe`（`__JS_call__` で `0` を評価する）で調べ、使えるときだけ `Parse_file_with`、`Parse_text_with` に知らせる。ネイティブ版は onError に進むので、今までと同じ構文木になる。
    - 評価（consh/include/consh_jsstmt.se。高速版と参照版の consh_eval.se が include する）：`Eval_jsstmt` が、コードに `//@consh-stmt` の行を付けて `__JS_call__` に渡す。引数は、渡す変数の名前、種類、値の 3 つの LList。ホストが書く `(w 名前 番号 種類 値)` の行で変数を入れ、`(f メッセージ)` なら代わりのコマンド行を実行する。
    - JS の値の変数は、`"js"` に番号を入れた Dict で、`Value_force` がシェルの値を取り出す（3.16 からは、読むたびに `Js_force` で直す）。そのため、変数を読むところは変えていない。シェルで代入し直せば、ふつうの値に置き換わる。
    - ホスト（consh/js/conma.js の `evaluateStatement`）：トップレベルの宣言を代入に書き換え（`rewriteDeclarations`。字句解析は `jsTokens`）、シェルの変数の Proxy を `with` のスコープにして評価する。値は `values` の表に置き、関数の JS の値は、`js` のコード `__consh_value(番号)` で呼ぶ。
- 仕様から変えたところ（2.5 に書いた）：
    - 打ち間違えたコマンドは、メッセージを真似るのでなく、同じテキストをシェルの文として実行する（2.5 の 7）。`nosuch | wc -l` のようなパイプラインも、今までと同じになる。
    - `$x` の変換は、参照するたびでなく、JS の文が終わるたびに行う（2.5 の 4）。参照のたびにホストを同期して呼ぶプリミティブが要らない。（3.16 で、`__JS_value__` を足して、読むたびに直す形に戻した）
    - JS の値の関数は、エラーにせず、`$f` で呼べる関数値にした。循環するオブジェクトは `String()` の文字列にした。
    - 先頭の語は `)` と `}` でも区切り、`/` は名前の部分だけで調べる（2.5 の 2）。`$(true)` の `true)` と `echo a; )` の `)` が JS の文になり、consh/test の step4 と step12 が wasm 版で変わったためである。
- テスト：consh/js/test/step5（Node.js）。case1 は判定、代入と宣言、`$` での変換、`$( )`、case2 は続きの行、ブロック、`await`、出力の規則、関数の中の局所変数、パイプラインの中の `stdin` と `console`、case3 は JS の値の共有と変換のし直し、関数の呼び出し、関数の写し、シェルで代入し直したとき、case4 はエラー（打ち間違えたコマンド、パイプライン、例外、拒否、どちらとしても読めない文）、case5 は対話モード（PS2 の続きの行、今定義されている関数）。
- ネイティブ版の consh/test（高速版と参照版、Rust 版の VM）と consh/js/test は通る。`CONMA_VM=wasm CONSH_IMPL=conma consh/test/run_all.sh` でネイティブ版と違うのは、今までどおり step8 と step17 だけである。ブラウザで、`n = 3`、`n * 2`、`await fetch(...)`、`$add 1 2`、打ち間違えたコマンドを確かめた。
- 名前が PATH にあるコマンドと同じなら、JS の文にならない。macOS には `/opt/X11/bin/x` と `/usr/bin/w` があるので、`x = 1` はシェルの文（外部コマンド `x`）になる。テストでは、そうした名前を使わない。

### 3.16. レビューへの対応（2026/11/9）

consh/js/REVIEW-ja.md（e0b2c58）の指摘に、次のように対応した。

- 1.1（大域変数と同じ名前への代入が大域変数に入る）：代入される名前は、大域変数にあってもシェルの変数にした（2.5 の 5）。`has` をいつも真にすると、`alert()` や `fetch()` が `with` の Proxy を `this` にして呼ばれ、Illegal invocation になるので、代入される名前だけを先に用意する。
- 1.2（`await` の間の更新が失われる）：文が終わったときに書き戻すのは、その文が代入した変数だけにした。
- 1.3（表がふくらむ）、1.4（文ごとに JS の値の変数をすべて直す）：文字列は String、プリミティブは値そのものを番号にして表に入れず、オブジェクトと関数は `$x` で読むたびに `__JS_value__` で直すようにした（2.5 の 4）。文ごとの直し直しはなくなり、`setTimeout` で後から変えたものも `$x` に表れる。VM には `__JS_value__` を足した（Rust 版、wasm 版、Python 版。ネイティブ版は onError に進む。test/step40 の case3）。
- 2.1：打ち間違えたコマンドとしてシェルの文を実行するときは、JS の代入を入れないようにした。レビューの例の `f(x = 1)` は、呼ぶ関数の名前を引数より先に評価するので、もともと代入の前にエラーになる。SyntaxError のときに JS のエラーを出す変数（`CONSH_JS_DEBUG`）は、まだ作っていない（4）。
- 2.2〜2.4：README に、移植できるスクリプトでは `js '...'` を使うこと、`n=1+2` と `n = 1+2` の対照、`if 文; then` の件を書いた。1 の 4 に、JS の文は JavaScript のある版だけと書いた。
- 2.5（console の出力の帰属）：3.5 に書いてあったものを、2.1 の 8 にも書いた。
- 2.6（遅延の LazyList の後始末）：2.2 の 4 に当面の制限として書いてある。4 にも挙げた。
- 3：モジュールの大きさ（今は約 7.0 MB。bin/includer と bin/resolver で作る。レビューの「約 2.2 MB、conma build」は vm/README.md の別のモジュールの数）と、評価のしかた（2.1 の 3、2.2 の 4、1 の 3）を直した。2.5 の 4 の `js 変数名=` には、すでに「まだ作っていない」と書いてあった。
- 4：JS の文のテスト（step5 の case4 など）は、エラーのメッセージが V8（Node.js）のものである。serverfs.js の同期の XMLHttpRequest は、4 に挙げた。
- テスト：consh/js/test/step5 の case3 に、`setTimeout` で後から変えたものが `$o2` に表れること、数と真偽値、`process = 5` がシェルの変数になることを足した。

### 3.17. js 変数名= の JS の値（2026/11/9）

- `js 変数名= コード` の結果を、JS の文と同じく JS の値のまま変数に入れるようにした（2.5 の 4）。`Builtin_js` は、変数名があれば、コードの前に `//@consh-keep` の行を付けて `__JS_call__` に渡す。ホストは、結果が文字列でもイテレータでもなければ、`(r j 番号)` を書き、ConSh は `Js_value` で JS の値の変数（`Js_wrap`）にする。
- 関数の結果は、エラーでなく、`$f` で呼べる値になった。consh/js/test/step1 の case4（`js f= '() => () => 1'` は、外側の関数を呼んだ `() => 1` が入り、`$f` は 1 を出す）と、step5 の case3（`js wv= '[1, 2]'` の要素が数のまま、`js ob= '({k: 1})'` の後の `ob.k + 1`）のログを変えた。

### 3.18. JS の値の表の解放とイテレータの後始末（2026/11/9）

- VM：`__JS_call__` が返す outputStream（ホストに書く側）は、最後の参照が捨てられたときに閉じるようにした（vm/src/value.rs の `StreamObj` の `drop_close` と `Drop`）。ほかの Stream は今までどおり、`__OS_close__` で閉じるまで開いている。
- ConSh：JS の値の変数（`Js_wrap`）は、`"handle"` に、その値を作った呼び出しの outputStream を持つ。JS の文と `js 変数名=` は、JS の値を変数に入れたら、outputStream を閉じずに変数に任せる（`Js_close_result`）。関数の JS の値を呼ぶ関数値も、変数の値（`Js_hold`）を持つ。
- ホスト：値の番号を、作った呼び出しごとに覚え（`owned`）、その呼び出しの outputStream が閉じたら（`host_js_input_end`）、表から消す（`releaseValues`）。変数に入れた遅延の LazyList の outputStream も、捨てられたら閉じるので、読み終わる前に捨てたイテレータの `return()` が呼ばれる（2.2 の 4 の当面の制限の一部）。
- 確かめたこと：オブジェクトの変数をシェルで代入し直すと表から消え、関数の JS の値も消える。読まないまま捨てた遅延の LazyList（`xs=${nat}` の後に両方を代入し直す）は、ホストに閉じたことが届く。一方、`for e in ${nat}; do break; done` のように途中まで読んだものと、関数の局所変数の値は、VM の中の循環に残る。前者は、40 万回の関数の呼び出しで循環の回収を走らせても消えなかった（4）。
- テスト：consh/js/test/step5 の case6（表の大きさ、`__consh_value.count()`）。ホストは、テストのために、表の大きさを返す `__consh_value.count` を置く。

### 3.19. ブラウザ版の VM を Worker で動かす（2026/11/9）

- ブラウザ版の VM を Worker で動かすようにした（REVIEW-ja.md の 4）。serverfs.js の同期の XMLHttpRequest と VM の実行は Worker で行い、画面は止まらない。JavaScript（`js` と JS の文）は、`document` や `window` を使えるように、ページで評価する（ユーザーの選択）。
- conma.js：JavaScript の評価を `JsEngine`（呼び出しを job の番号で扱い、行を `out.line`、終わりを `out.end` で返す）にまとめ、VM（`create`）から分けた。`create` は、`hooks.jsRemote` があれば、`host_js_call`、`host_js_input`、`host_js_input_end`、`host_js_value` をそれに渡し、行は `vm.jsLine`、終わりは `vm.jsEnd` で受け取る。なければ、今までどおり同じスレッドの `JsEngine` を使う（Node.js、ページの中で動かすとき）。
- web/consh-core.js：VM、ファイル、`COMMANDS`、`hostProc`、VM を区切って動かすことを、consh.js から移した。Worker（consh-worker.js）でもページ（consh.js）でも動く。
- web/consh-worker.js：ページとメッセージでつなぐ。js の呼び出しはページに渡し、ページの `JsEngine` が書く行を VM のパイプに書く。読み手がいなくなったら（EPIPE）ページに知らせる。`__JS_value__` は同期なので、ページに尋ねて、SharedArrayBuffer を `Atomics.wait` で待つ（1 MB ずつ受け取るので、大きな値も渡せる）。
- web/consh.js：画面と JsEngine。ページが cross-origin isolated（`crossOriginIsolated`）なら Worker で、そうでなければページの中で consh-core.js を動かす。serve.py は COOP と COEP のヘッダを送る。
- ブラウザ（serve.py）で、Worker で動くこと、JS の文から `document.title` を変えられること、`setTimeout` で後から変えたオブジェクトが `$o` に表れること（`__JS_value__`）、`cat < names.txt | js ...`、`ls | js ...` と `console.log`、イテレータを `for` で読むこと、打ち間違えたコマンドを確かめた。ヘッダのないサーバー（`python3 -m http.server`。`.claude/launch.json` の `consh-web-plain`）では、ページの中で動き、同じく `document.title`、`$o`、`cat < names.txt | js ...` が動くことを確かめた。Node.js の `consh/js/test`（同じスレッドの JsEngine）は通る。

### 3.20. CONSH_JS_DEBUG（2026/11/9）

- JS の文をシェルの文として実行するとき（2.5 の 7）、シェルの変数か環境変数の `CONSH_JS_DEBUG` が空でなければ、JavaScript としてのエラーを `js: SyntaxError: Unexpected identifier 'a' (run as a shell statement)` のように出すようにした（`Js_debug`。REVIEW-ja.md の 2.1）。シェルの文としても解析できない文は、今までどおり、JavaScript のエラーだけを出す。
- テスト：consh/js/test/step5 の case7。

## 4. 未解決の点

- JS の値の関数を、`$f 1 2` だけでなく `f 1 2` でも呼べるようにするか。シェルの関数を JavaScript から呼ぶか（2.5 の 9）。
- JS の値の表の解放と、イテレータの `return()` は、VM が値を捨てたときに行う（3.18）。`for` で途中まで読んだ遅延の LazyList は、VM が捨てず、循環の回収でも消えないので、ホストのイテレータと呼び出しが残る。関数の局所変数の JS の値も、循環の回収まで残る。VM の LazyList と循環の回収（vm/src/gc.rs）を調べる必要がある。
- ブラウザ版は、cross-origin isolated でないサーバーでは、今までどおりページの中で VM を動かすので、ファイルを読む間は画面が止まる（3.19）。
- 結果のオブジェクトを JSON の文字列でなく ConMa の Dict にするか。ConSh にはまだ Dict の値がない（ConSh のレビューの 1、ConMa の 6 のレコードとつながる）。
