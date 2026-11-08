# JavaScript 版 ConSh の計画

JavaScript 版 ConSh は、ConMa の VM を WebAssembly（wasm32）にして JavaScript の処理系の上で ConSh を動かし、組み込みコマンド `js` で JavaScript のコードを呼べるようにしたものである。

この文書は、memo.txt の 2026/11/8 の「JavaScript 版 ConSh の方針」を計画にしたものである。関係するファイルは consh/js に置く（VM 自体の変更は vm/ に入る。2.3 を参照）。

## 1. 決めたこと

次の 3 点は、それぞれ (推奨) の案に決めた（2026/11/8）。

1. **実行環境**
    - (推奨・決定) まずブラウザで動かす。memo の例（`location.href`、`fetch`）はブラウザを想定している。
    - Node.js のホストで外部コマンドを使えるようにするのは後回しにする（段階 7）。OS 層の作り直しが大きい。テストは、Node.js（vm-node.js）か、macOS の JavaScriptCore のシェル（jsc、vm-jsc.js）で動かす（2.4）。
2. **何を wasm にするか**
    - (推奨・決定) ConMa の VM（vm/）を wasm にし、ConMa 版 ConSh（consh/include）を動かす。memo の「先に ConMa の VM を wasm32 向けにビルドできるようにする」に従う。ConMa の VM が OS 以外の環境でも動くことの確認にもなる。
    - Rust 版 ConSh（consh/rust）を wasm にするのは別の案である。速いが、ConMa 言語を使わない。また std::process と std::io::pipe に深く依存している。
3. **JavaScript の処理系**
    - (推奨・決定) wasm を読み込んだホスト（ブラウザ）の処理系をそのまま使う。`js` のコードは、ホストの JavaScript から間接 eval で評価する。
    - QuickJS などを wasm に組み込む案は取らない。ページの DOM や `fetch` が使えなくなるからである。

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
    - 評価は大域スコープで行い（段階 5 からは `new Function("stdin", ...)`。strict でない）、ConSh の変数は見えない。
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
        - コードは `new Function("stdin", "return (" + code + ")")` で評価する（大域スコープで、strict でない）。
    - 段階 6 で、イテレータを足した。結果がイテレータ（配列と文字列以外の反復可能なもの。ジェネレータ、Set、Map も）なら、ホストは `(r t)` を書く。ConSh が `outputStream` に `n` の行を書くたびに、ホストは次の要素を `(t 要素)` で書く。終わりなら `(d)`、例外なら `(e "...")` を書く。要求された分だけ進めるので、終わらないジェネレータも、読む分だけ要素を作る。
        - ConSh が `outputStream` を閉じたら、読み手がいなくなったとして、ホストは呼び出しを止め、イテレータの `return()` を呼ぶ。その間の `console` の出力（ジェネレータの `finally` など）は、ホストの標準出力と標準エラー出力に直接書く。
        - 出力する形では、2 つの Stream を開いた Stream の表（`Open_track`）に入れてから要素を読む。`| head` などで読み手が先に終わってこの VProc が止められると、表から閉じられ、ホストに届く。
        - 変数に入れる形では、要素を読んだときに `n` を書いて受け取る遅延の LazyList にする。最後まで読まれなければ、2 つの Stream は開いたまま、ホストの呼び出しも終わらないまま残る（当面の制限）。
    - ネイティブ版（Python と Rust の VM）の `__JS_call__` は、引数を確かめた後、`onError` に進む（`__JS_call__: JavaScript is not available`）。
5. **モジュールの読み込み**
    - ConSh のモジュール（約 5 MB）は、ネイティブで bin/includer と bin/resolver（または ConMa の front end）で作っておき、静的なファイルとしてページと一緒に置く。
    - スクリプトの構文解析は、Python の consh_parse がないので、ConMa 版の構文解析（consh_parser.se、`--conma-parser` と同じ）を使う。
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
  web/              （段階 3）HTTP で開く（python3 -m http.server --directory consh/js/web など）
    index.html      端末の画面（出力と入力の行、「ファイルを置く」「スクリプトを実行」「中断」）
    consh.js        ConSh の起動（対話モード）、入力、Ctrl-C、ファイル。段階 4 から js の評価
    conma.wasm、conma.js、consh.module.se   build.sh が置く（リポジトリには入れない）
  test/             （段階 4）ConSh の test/stepN と同じ形の js のテスト。stepN/test.sh は、CONMA_VM=wasm で
                    consh/test/common.sh の run を使う。run_all.sh ですべて実行する
```

VM の Rust のコードのうち、ネイティブ版と共通のもの（sys.rs の関数を通すようにした変更、スケジューラ、段階 4 の js のプリミティブ）は vm/src に置く。ConSh の `js` の組み込みコマンドは、consh/include/consh_eval.se と consh/rust/src/eval.rs に置く（2.1 の 10）。

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

- `js [変数名=] コード {引数}` を、ConMa 版の ConSh（consh/include/consh_eval.se の `Builtin_js`）に足した。2.1 の 1〜6、9、10 のとおりに動く。Rust 版の ConSh（consh/rust）の `js` は、同じ順に引数を確かめた後、`js: JavaScript is not available` を出して `$?` を 127 にする。
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

## 4. 未解決の点

- 結果のオブジェクトを JSON の文字列でなく ConMa の Dict にするか。ConSh にはまだ Dict の値がない（ConSh のレビューの 1、ConMa の 6 のレコードとつながる）。
- Rust 版 ConSh も wasm にして、`js` を使えるようにするか（1 の 2）。
