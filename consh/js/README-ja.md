# JavaScript 版 ConSh

ConMa の VM を WebAssembly（wasm32）にビルドし、ブラウザか Node.js の上で ConMa 版の ConSh を動かす。組み込みコマンド `js` で JavaScript を呼べる。計画と設計は [PLAN-ja.md](PLAN-ja.md) にある。

## 作る

```
rustup target add wasm32-unknown-unknown   # 初めてのときだけ
consh/js/build.sh
```

`build.sh` は、VM を wasm32 でビルドし（consh/js/wasm、約 700 KB）、`web/` に wasm、`conma.js`、ConSh のモジュール（`consh.module.se`、約 5.8 MB）を置く。

## ブラウザで使う

`consh/js/web` を `serve.py` で開く。

```
consh/js/serve.py          # http://localhost:8765/（ポートは引数で変えられる）
```

http://localhost:8765/ を開くと、ConSh の対話モードが動く（`.claude/launch.json` の `consh-web` と同じ）。`serve.py` は `python3 -m http.server` と同じくファイルを出すほか、ディレクトリの一覧（`?consh-ls`）を返し、ブラウザにファイルを毎回確かめさせる（`Cache-Control: no-cache`）。`build.sh` が `web/` に作るファイル（`conma.js`、`conma.wasm`、`consh.module.se`）がないか、ソースより古いとき（`git pull` の後など。これらはリポジトリにないので pull では変わらない）は、`build.sh` を実行するように言って起動しない。ほかの Web サーバーでも動くが、`index.html` のあるディレクトリ（ページのディレクトリ）の一覧が取れないので、`ls` や `*` にはそのディレクトリのファイルが出ない。

* 外部コマンドはない。JavaScript で作った `cat`、`pwd`、`ls`（`web/consh.js` の `COMMANDS`）が、ConSh の組み込みコマンドになっている（環境変数 `CONSH_HOST_BUILTINS`、ConSh の PLAN-ja.md の 1.2）。`cat` は、引数のファイルか標準入力を、標準出力に書く。`pwd` は、カレントディレクトリを出す（`-L` と `-P` は同じ）。`ls [-1AadF] [file ...]` は、名前を 1 行に 1 つ出す。`builtin cat` でも呼べ、`command cat` などの外部コマンドは `No such file or directory`（`$?` は 127）になる。関数、組み込みコマンド、パイプ、`$( )`、`&` の関数のジョブ、ファイルへのリダイレクトは使える。
* `cat` などが端末から読んでいる間は、入力欄の行はそのコマンドに渡り、Ctrl+D はそのコマンドの入力だけを終える（`cat > hoge.txt` で書いて Ctrl+D、ConSh は続く）。Ctrl+C はそのコマンドを止める（`$?` は 130）。ConSh 自身が入力を読んでいるときの Ctrl+D は、ネイティブ版と同じく ConSh の標準入力を終えるので、ConSh も終わる。
* ファイルはサーバーのもの（`web/serverfs.js`）。カレントディレクトリの `/home/user` がページのディレクトリ（`consh/js/web`）で、その下のファイルとディレクトリは、開くたび、一覧するたびにサーバーから読む（キャッシュはブラウザに任せる）。ディスクのファイルを書き換えれば、ページを読み直さなくても見える。
* 書いたファイル（`> file`、`>> file`）と「ファイルを置く」で置いたファイルは、メモリに置き、同じ名前のサーバーのファイルより先に見える（サーバーには書かない。ページを閉じると消える）。`/home/user` の外は、`/dev/null` などのメモリのものだけがある。
* ページのディレクトリを置く場所は、URL の `?home=絶対パス` で変えられる（ConSh の `HOME` とカレントディレクトリもそこになる）。`http://localhost:8765/?home=/` なら `/` がページのディレクトリになり、`/dev` と `/dev/null` は、同じ名前のサーバーのファイルより先に見える。絶対パスでなければ `/home/user` になる。
* 「スクリプトを実行」は、スクリプトの各行を入力として渡す（ConMa 版には `source` がない）。
* Ctrl-C か「中断」で、動いているコマンドを止める。

## Node.js で使う

`test/vm.sh` は、`CONMA_VM=wasm` のとき wasm 版の VM を Node.js で動かす（`vm-wasm.sh`、`vm-node.js`）。ConSh も同じく動かせる。

```
CONMA_VM=wasm CONSH_IMPL=conma consh/bin/consh script.sh
```

VM は、ネイティブ版と同じく、実際のファイルを読み書きし、外部コマンドを起動する。子の標準入出力のパイプは、Node.js が UNIX ドメインソケットでつなぐので、`test -p` などではパイプに見えない（PLAN-ja.md の 3.8）。

## js

```
js [変数名=] コード {引数}
```

* コードを JavaScript の式として評価する。値が関数なら引数を渡して呼び、Promise なら解決するまで待つ（その間、ほかの VProc は動く）。
* 式として読めないコード（`let x = 1; return x + 1` など）は、async 関数の本体として動かし、`return` の値を結果にする。`await` を書け、引数は配列 `args` で見える。
* 最初の語が「識別子 `=`」だけ（スペースの前に `=`）なら、結果をその局所変数に入れる。そうでなければ、結果を標準出力に出す。
* 引数の String は文字列、LazyList（`${list}`）は配列として渡す。
* 結果の文字列は値 1 つ、配列は要素ごとの値（変数には LazyList）、`undefined` と `null` は何も出さない（変数には空文字列）。そのほかの値は `String()` か `JSON.stringify` で文字列にする。
* 結果がイテレータ（ジェネレータ、Set、Map など、配列と文字列以外の反復可能なもの）なら、要素ごとの値にする。要素は読まれる分だけ作るので、終わらないジェネレータも `| 関数` や `> (a b)` で使える。変数に入れると、読んだときに要素を作る遅延の LazyList になる（`${nat}` で 1 つずつたどる）。
* 例外のときは `js: 名前: メッセージ` を出して `$?` を 1 にする。
* コードの中では、`stdin` が標準入力の要素（文字列）の非同期のイテレータになる。`stdin.lines()` は全部の配列の Promise。`stdin` を読まないコードは、標準入力を読まない。
* `console.log`（`info`、`debug`）は標準出力に、`console.error`（`warn`）は標準エラー出力に、1 回で 1 つの要素を出す。変数名を書いたときは、標準出力には `console.log` の出力だけが出る。

```
js '1 + 2'                                          # 3
js sum= '(a, b) => Number(a) + Number(b)' $x $y     # sum に入れる
js '["a", "b"]' > (first . rest)
js text= 'fetch("data.txt").then(r => r.text())'    # ブラウザ
cmd | js 'async () => { let n = 0; for await (const l of stdin) n++; return n }'
js nat= '(function* () { let i = 0; while (true) yield i++ })()'     # 遅延の LazyList
```

ネイティブの VM（Python 版、Rust 版）と Rust 版の ConSh では、`js` は `js: JavaScript is not available` を出して `$?` を 127 にする。

## JS の文

JavaScript のある版（ブラウザと Node.js の wasm 版）では、先頭の語がコマンド（予約語、組み込みコマンド、関数、PATH にある外部コマンド）でない文を、行全体の JavaScript のコードとして評価する（PLAN-ja.md の 2.5）。

```
n = 3                          # シェルの変数 n に、JavaScript の数 3
n * 2                          # 6
echo $n                        # 3（$ では文字列やリストにする）
let o = {a: 1, b: [1, 2]}      # let、const、var の宣言もシェルの変数になる
echo $o                        # {"a":1,"b":[1,2]}
xs = [1, 2, 3].map(i => i * n)
for x in $xs; do echo $x; done
name=world                     # 名前= で始まる文は、今までどおりシェルの変数定義（String）
console.log(`hello ${name}`)   # シェルの変数も JavaScript の変数として見える
add = (a, b) => Number(a) + Number(b)
$add 1 2                       # JavaScript の関数は $ を付けて呼ぶ
r = await fetch("names.txt").then(r => r.text())
```

* 括弧や文字列が閉じていなければ、次の行も続きとして読む（対話モードでは PS2 を出す）。
* JS の文の中の `;`、`|`、`>` は JavaScript のもので、パイプやリダイレクトにはならない。シェルの関数の本体では、JS の文を 1 行に 1 つ書く。
* 式の値は `js` と同じく標準出力に出す。代入と宣言は何も出さない。
* JavaScript の `if`、`for`、`function` は、文の始めではシェルの構文になる。`{ for (...) ... }` のようにブロックにするか、`let f = function () {...}` と書く。`"..."`、`/.../`、`./` で始まる文もシェルの文である。
* JavaScript としてコンパイルできないか、先頭の名前が定義されていなければ、打ち間違えたコマンドとみて、シェルの文として実行する（`nosuchcommand a b` は、今までどおりコマンドが見つからない）。
* 判定はスクリプトを解析するときに行う。PATH にあるコマンドと同じ名前（macOS の `x`、`w` など）の変数は、文の始めでは JS にならない。

## テスト

```
consh/js/test/run_all.sh                   # js のテスト（Node.js）
CONMA_VM=wasm test/run_all.sh              # ConMa のテストを wasm 版で
CONMA_VM=wasm CONSH_IMPL=conma consh/test/run_all.sh
```

wasm 版で ConMa と ConSh のテストを流すと、外部コマンド、プロセス、`__VProc_spawn__`、シンボリックリンクを使うケースだけがネイティブ版と違う（PLAN-ja.md の 3.1〜3.4）。
