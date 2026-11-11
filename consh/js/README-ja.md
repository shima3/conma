# JavaScript 版 ConSh

ConMa の VM を WebAssembly（wasm32）にビルドし、ブラウザか Node.js の上で ConMa 版の ConSh を動かす。算術評価 `(( ))` と算術置換 `$(( ))` に JavaScript のサブセット（SJS）を書け、組み込みコマンド `js` で JavaScript を呼べる。計画と設計は [PLAN-ja.md](PLAN-ja.md) にある。

**セキュリティの注意**：`js` は、ホストの JavaScript でコードをそのまま評価する（`Function` コンストラクタ。サンドボックスはない）。`(( ))` の SJS は、ConMa に変換して VM の中で動かす（段階 B。ホストの JavaScript に触れない。ブラウザ版も段階 B6 から）。変換したモジュールを作れないとき（`frontend.module.se` がないなど）だけ、`CONSH_ENABLE_JS` が `sjs` か `js` なら、ホストの JavaScript で評価する（段階 A）。SJS の範囲の外のコードはエラーにするが、段階 A に抜け道がないことは証明できない（consh/SAFEJS-PLAN-ja.md の 5）。ConSh のスクリプトや入力を書く人は、そのページ（DOM、Cookie、同じオリジンへのリクエスト）や Node.js のプロセス（ファイル、プロセス、ネットワーク）の権限で、何でもできる。当面は、自分の手元（ローカル）で、信用できるスクリプトだけに使うこと。信用できない入力をそのまま ConSh に渡したり、公開のサーバーで誰でも使えるようにしたりしない。

そのため、`js` は、既定では使えない（`js: JavaScript is disabled ...` を出して `$?` を 2 にする）。使うときは、次のように有効にする（[「JavaScript を有効にする」](#javascript-を有効にする)）。`js` は `js` と、段階 A の `(( ))` を使えるようにし、`sjs` は段階 A の `(( ))` だけを使えるようにする。

* Node.js 版：環境変数 `CONSH_ENABLE_JS` を `sjs` か `js` にして起動する。
* ブラウザ版：サーバーのページのディレクトリ（`consh/js/web`）に `config.json` を置き、`{"CONSH_ENABLE_JS": "sjs"}`（か `"js"`）と書く。

## 作る

```
rustup target add wasm32-unknown-unknown   # 初めてのときだけ
consh/js/build.sh
```

`build.sh` は、VM を wasm32 でビルドし（consh/js/wasm、約 700 KB）、`web/` に wasm、`conma.js`、ConSh のモジュール（`consh.module.se`、bin/includer と bin/resolver で作る。約 7.0 MB）を置く。

## ブラウザで使う

`consh/js/web` を `serve.py` で開く。

```
consh/js/serve.py          # http://localhost:8765/（ポートは引数で変えられる）
```

http://localhost:8765/ を開くと、ConSh の対話モードが動く（`.claude/launch.json` の `consh-web` と同じ）。`serve.py` は `python3 -m http.server` と同じくファイルを出すほか、ディレクトリの一覧（`?consh-ls`）を返し、ブラウザにファイルを毎回確かめさせる（`Cache-Control: no-cache`）。`build.sh` が `web/` に作るファイル（`conma.js`、`conma.wasm`、`consh.module.se`、`frontend.module.se`）がないか、ソースより古いとき（`git pull` の後など。これらはリポジトリにないので pull では変わらない）は、`build.sh` を実行するように言って起動しない。ほかの Web サーバーでも動くが、`index.html` のあるディレクトリ（ページのディレクトリ）の一覧が取れないので、`ls` や `*` にはそのディレクトリのファイルが出ない。

* VM、ファイル、`cat` などの組み込みコマンドは Worker（`web/consh-worker.js`、`web/consh-core.js`）で動き、ページは画面と JavaScript（`js` と `(( ))` の SJS）を受け持つ。そのため、ファイルを読む間や長い処理の間も画面は固まらず、JavaScript からは今までどおり `document` や `window` が使える。Worker に JS の値を同期して渡すために SharedArrayBuffer を使うので、ページが cross-origin isolated である必要がある。`serve.py` は、そのためのヘッダ（`Cross-Origin-Opener-Policy: same-origin`、`Cross-Origin-Embedder-Policy: require-corp`）を送る。そのため、ページから別のオリジンのものを no-cors で読む（`<img>`、`<script>`、`fetch` の `mode: "no-cors"`）と、相手が `Cross-Origin-Resource-Policy` を付けて出していなければ止められる。CORS で読む `fetch`（既定）は、相手が CORS を許していれば動く。
* このヘッダを送らないサーバーでは、ConSh はページの中で動く（Worker を使う前と同じ。ファイルを読む間は画面が止まる）。最初の案内に、どちらで動いているかが出る。

* 外部コマンドはない。JavaScript で作った `cat`、`pwd`、`ls`（`web/consh-core.js` の `COMMANDS`）が、ConSh の組み込みコマンドになっている（環境変数 `CONSH_HOST_BUILTINS`、ConSh の PLAN-ja.md の 1.2）。`cat` は、引数のファイルか標準入力を、標準出力に書く。`pwd` は、カレントディレクトリを出す（`-L` と `-P` は同じ）。`ls [-1AadF] [file ...]` は、名前を 1 行に 1 つ出す。`builtin cat` でも呼べ、`command cat` などの外部コマンドは `No such file or directory`（`$?` は 127）になる。関数、組み込みコマンド、パイプ、`$( )`、`&` の関数のジョブ、ファイルへのリダイレクトは使える。
* `cat` などが端末から読んでいる間は、入力欄の行はそのコマンドに渡り、Ctrl+D はそのコマンドの入力だけを終える（`cat > hoge.txt` で書いて Ctrl+D、ConSh は続く）。Ctrl+C はそのコマンドを止める（`$?` は 130）。ConSh 自身が入力を読んでいるときの Ctrl+D は、ネイティブ版と同じく ConSh の標準入力を終えるので、ConSh も終わる。
* ファイルはサーバーのもの（`web/serverfs.js`）。カレントディレクトリの `/home/user` がページのディレクトリ（`consh/js/web`）で、その下のファイルとディレクトリは、開くたび、一覧するたびにサーバーから読む（キャッシュはブラウザに任せる）。ディスクのファイルを書き換えれば、ページを読み直さなくても見える。
* 書いたファイル（`> file`、`>> file`）と「ファイルを置く」で置いたファイルは、メモリに置き、同じ名前のサーバーのファイルより先に見える（サーバーには書かない。ページを閉じると消える）。`/home/user` の外は、`/dev/null` などのメモリのものだけがある。
* ページのディレクトリを置く場所は、URL の `?home=絶対パス` で変えられる（ConSh の `HOME` とカレントディレクトリもそこになる）。`http://localhost:8765/?home=/` なら `/` がページのディレクトリになり、`/dev` と `/dev/null` は、同じ名前のサーバーのファイルより先に見える。絶対パスでなければ `/home/user` になる。
* 「スクリプトを実行」は、スクリプトの各行を入力として渡す（ConMa 版には `source` がない）。
* Ctrl-C か「中断」で、動いているコマンドを止める。

## JavaScript を有効にする

`js` と、段階 A で動かす `(( ))`（モジュールを作れないとき）は、ホストの JavaScript を使うので、既定では使えない（上のセキュリティの注意）。段階 B で動かす `(( ))`（Node.js 版、ブラウザ版、ネイティブの VM の ConMa 版）は、ホストの JavaScript に触れないので、`CONSH_ENABLE_JS` によらず使える。ConSh は起動したときに一度だけ、何を使ってよいかを、`CONSH_ENABLE_JS` の値で決める（consh/include/consh_jsstmt.se の `Js_state`）。

| `CONSH_ENABLE_JS` | 段階 A の `(( ))` と `$(( ))`（モジュールを作れないとき） | `js` |
|---|---|---|
| `sjs` | 使える | 使えない |
| `js` | 使える | 使える |
| ないか、ほかの値 | 使えない | 使えない |

* **Node.js 版**：VM のプロセスの環境変数 `CONSH_ENABLE_JS` で決める。起動した後にシェルで `export CONSH_ENABLE_JS=js` としても、今の ConSh は変わらない（シェルの `export` は、起動するコマンドの環境だけを変える）。ただし、環境変数なので、その ConSh から起動した別の `consh` には引き継がれる。

  ```
  CONSH_ENABLE_JS=sjs CONMA_VM=wasm CONSH_IMPL=conma consh/bin/consh script.sh
  ```

* **ブラウザ版**：ページは、起動するときにサーバーの `config.json`（ページと同じディレクトリ）を読み、`"CONSH_ENABLE_JS"` の値（文字列）を、VM の環境変数 `CONSH_ENABLE_JS` にする。ファイルがない、読めない、その値がないときは、どれも使えない。`config.json` はリポジトリに入れない（`.gitignore`）ので、使うサーバーで作る。

  ```
  echo '{"CONSH_ENABLE_JS": "sjs"}' > consh/js/web/config.json
  ```

  URL のパラメータで有効にしないのは、他人が作ったリンクを開くだけで有効になってしまうからである。ページの前にいる本人は、開発者ツールでいつでも JavaScript を動かせるので、この設定は本人ではなく、他人のリンクやスクリプトから守るためのものである。
* JavaScript のない版（ネイティブの VM と Rust 版の ConSh）では、今までどおり `js: JavaScript is not available`（`$?` は 127）になる。`(( ))` は、ネイティブの VM の ConMa 版では段階 B で動き、Rust 版の ConSh にはない。
* この設定で守れるのは、主にブラウザ版である。Node.js 版の ConSh は、もともと外部コマンドを起動でき、ファイルも読み書きできるので、`js` を止めても、スクリプトにできることはほとんど減らない。Node.js 版では、信用できないスクリプトを ConSh で動かさないこと。

## Node.js で使う

`test/vm.sh` は、`CONMA_VM=wasm` のとき wasm 版の VM を Node.js で動かす（`vm-wasm.sh`、`vm-node.js`）。ConSh も同じく動かせる。

```
CONMA_VM=wasm CONSH_IMPL=conma consh/bin/consh script.sh
```

引数なしで起動すると対話モードになる。`(( ))` は段階 B で動くので、そのまま使える。`js` を使うときは `CONSH_ENABLE_JS=js` を付ける（上の「JavaScript を有効にする」）。

VM は、ネイティブ版と同じく、実際のファイルを読み書きし、外部コマンドを起動する。子の標準入出力のパイプは、Node.js が UNIX ドメインソケットでつなぐので、`test -p` などではパイプに見えない（PLAN-ja.md の 3.8）。

## (( )) と $(( ))：SJS

算術評価 `(( SJS式 ))` と算術置換 `$(( SJS式 ))` に、JavaScript のサブセット（SJS）の式を書ける（consh/SAFEJS-PLAN-ja.md の 2）。Node.js 版（とネイティブの VM の ConMa 版）では、式を ConMa に変換して VM の中で動かす（段階 B4。consh/sjs/README-ja.md）。ブラウザ版も、段階 B6 から同じく段階 B で動かす（ページの Worker の中で、もう 1 つの VM に ConMa のフロントエンド `frontend.module.se` を動かして、モジュールを作る。`web/consh-core.js` の `buildModule`）。モジュールを作れないときだけ、`CONSH_ENABLE_JS` が `sjs` か `js` なら、ホストの JavaScript で評価する（段階 A。SAFEJS-PLAN-ja.md の 4.1）。どちらも同じ結果になるようにしてある（段階 B で違うところは consh/sjs/README-ja.md の「段階 A との違い」）。下の組み込みの説明は段階 A のもので、段階 B の組み込みは consh/sjs/README-ja.md にある。

```
(( n = 1 + 2 ))                # SJS変数 n に 3。何も出さない。$? は値の真偽（真なら 0、偽なら 1、エラーなら 2）
if (( n > 2 )); then echo big; fi
echo "n is $(( n ))"           # 算術置換。ほかの語要素とは文字列にして連結する（"..." の中でも使える）
v=$(( [n, n * 2] ))            # 算術置換だけの語は、JS の値のまま変数に入れる
for x in $v; do echo $x; done  # $ で読むと、js の値と同じく文字列かリストになる
(( sum = (a) => { let s = 0; for (const x of a) s += x; return s } ))
echo $(( sum([1, 2, 3]) ))     # 6
```

* `((` の後と `))` の前には空白（改行も）が要る。SJS式の終わりは JavaScript の字句で決めるので、文字列の中の ` ))` では終わらない。閉じるまで次の行も読む（対話モードでは PS2 を出す）。
* SJS式の中の代入は、シェルの変数ではなく **SJS変数**を変える。SJS変数は、シェルの変数と同じ名前でも別のもので、シェルの変数と同じく、関数、パイプラインの要素、`$( )`、`&` では写しになる（シェルの関数の中で `(( n = 5 ))` としても、外の `n` は変わらない）。SJS変数の値をシェルの変数に入れるときは `v=$(( n ))` と書く。
* 大域の名前は、SJS変数と、SJS の組み込み（`Math`、`JSON`、`Number`、`String`、`Boolean`、`Array`、`Object` の `keys` など、`parseInt`、`parseFloat`、`isNaN`、`isFinite`、`console`、`undefined`、`NaN`、`Infinity`）だけである。組み込みには代入できず（`TypeError`）、組み込みはホストのものの凍結した写しなので、変えてもホストには届かない。ホストの名前（`process`、`document`、`fetch` など）は `ReferenceError` になる。
* 関数の本体のブロックには、`let`、`const`、`if`、`for`、`while`、`return`、`try` なども書ける（`(( ))` の中なので ConSh の構文と衝突しない）。
* 次は SJS の範囲の外として、評価する前にエラーにする（`sjs: not in SJS: ...`、`$?` は 2）：`this`、`new`、`class`、`import`、`eval`、`arguments`、`async`、`await`、`yield`、`globalThis`、`Function` などの名前、`__` で始まる名前、`.constructor`、`.prototype`、`.__proto__` などのプロパティ、正規表現、`==` と `!=`。計算したキー `a[k]` は、評価のときに、`constructor` などのキーを `TypeError` にする。
* **演算の型**：`+` は数どうしだけを足す。文字列をつなぐときは、テンプレート文字列（`` `${a}${b}` ``）か `[a, b].join("")` を使う。`-`、`*`、`/`、`%`、`**`、ビット演算（`&`、`|`、`^`、`~`、`<<`、`>>`、`>>>`）、単項の `-` と `+`、`++` と `--`、複合の代入（`+=` など）も、数でなければ `TypeError` になる。大小の比較（`<` など）は、数どうしか文字列どうしだけ。JavaScript の意味は変えず、紛らわしい組み合わせをエラーにするだけなので、SJS でエラーにならずに動いたコードは、JavaScript でも同じ結果になる。シェルから渡した引数は文字列なので、数として使うときは `Number(x)` と書く（`$名前` は、演算子の隣なら自動で数になる）。
* `console.log` は標準出力に出る（リダイレクトできる）。
* 文の `(( ))` は値を出さない（`$?` だけ）。ただし**対話モード**では、コマンド行の文がそのまま `(( ))` なら、Node.js の REPL のように値を標準エラーに出す（SJS関数の戻り値と同じ形。`undefined` と `null` は出さない。`(( 1+2 )) 2> (n)` で `n` は 3）。ループ、関数、`if` の条件の中の `(( ))` は出さない。スクリプトでは出さない。Rust 版の ConSh（`CONSH_IMPL` の既定）には `(( ))` がないので、`CONSH_IMPL=conma` で使う
* 段階 B では、ジェネレータと `async`/`await` も使える（consh/sjs/README-ja.md の段階 B5a、B5b）。段階 A では範囲の外。
* 文の `(( ))` でエラーになれば、メッセージを出して `$?` を 2 にし、次の文に進む。語の中の `$(( ))` でエラーになれば、変数が未定義のときと同じく、その文を止め（スクリプトなら終了ステータス 2 で終える。対話モードならそのコマンド行を止める）、`$?` を 2 にする。
* **`$名前` でシェル変数を読む**：SJS式の中の `$n` は、シェル変数 `n` の値である（読むだけ。代入と `let $n` の宣言はエラー）。String は文字列、リストは配列、JS の値はそのまま。定義されていなければエラー（局所変数になければ環境変数も見る）。

  ```
  n=3
  echo $(( $n + 1 ))             # 4：演算子の被演算子の $n は数にする
  z=007
  echo $(( `${$z}` ))            # 007：ほかの所では値のまま
  (( k = $n ))                   # SJS変数 k は文字列 "3"
  ```

  算術（`+`、`-`、`*`、`/`、`%`、`**`、単項の `-` と `+`）と大小の比較（`<`、`<=`、`>`、`>=`）のすぐ左か右の `$n`（`($n)` のように括弧で囲んでもよい）と、数のリテラルと `===`、`!==` で比べる `$n` は、数にする。数の書き方（`3`、`-2.5`、`1e3`）でなければ、`NaN` にせず `not a number` のエラーにする。真偽（`!$n`、`$n ? a : b`）は数にしないので、`n=0` の `$n` は真である。SJS の関数の中の `$n` は、呼び出したときのシェル変数を読む。
* **SJS の関数をコマンドとして呼ぶ**：先頭の語の値が SJS の関数（`$f` か `$(( ... ))`）なら、後の語を引数として呼ぶ。戻り値は**標準エラー**に出し（`undefined` と `null` は何も出さない）、`$?` はエラーなら 2、そうでなければ戻り値の真偽（真なら 0、偽なら 1。`undefined` を返す関数は 1）。

  ```
  add=$(( (a, b) => Number(a) + Number(b) ))
  $add 1 2 2> (c)                # c は 3
  $add 1 2 2>&1 | cat            # 標準出力に出すときは 2>&1
  $(( (x) => { m = x } )) hello  # シェルの値を SJS変数 m に入れる（何も出さない）
  if $isBig 50 2>/dev/null; then ...; fi
  ```

  引数はシェルの関数と同じく展開し、String は文字列、リスト（`${xs}`）は配列になる（`$xs` は要素を別々の引数にする）。シェル変数の JS の値は、引数では文字列かリストになるので、値のまま渡すときは `(( f($v) ))` のように `$名前` を使う。先頭の語の値が、SJS が作った関数でない値なら、`not a function` のエラー（`$?` は 2。語の中の `$(( ))` のエラーと同じく、スクリプトは終わる）。
* SJS の関数の中の大域の名前は、**呼び出したときの** SJS変数を読み書きする（動的なスコープ）。シェルの関数の中で呼べば、その関数の中の写しを読む。SJS変数の写しは浅く、配列やオブジェクトの中身は写さない（シェルの関数の中で `(( o.a = 1 ))` とすると、外の `o` の中身も変わる）。
* `async`、`await`、ジェネレータ（`yield`）は、今は SJS の範囲の外である（`await` の後で、別の評価の SJS変数を見てしまうため）。
* 制限：`typeof` を定義されていない名前に使うと、`"undefined"` ではなく `ReferenceError` になる。

## js

```
js [変数名=] コード {引数}
```

* コードを JavaScript の式として評価する。値が関数なら引数を渡して呼び、Promise なら解決するまで待つ（その間、ほかの VProc は動く）。
* 式として読めないコード（`let x = 1; return x + 1` など）は、async 関数の本体として動かし、`return` の値を結果にする。`await` を書け、引数は配列 `args` で見える。
* 最初の語が「識別子 `=`」だけ（スペースの前に `=`）なら、結果をその局所変数に入れる。そうでなければ、結果を標準出力に出す。変数には、文字列とイテレータのほかは JavaScript の値のまま入り（PLAN-ja.md の 2.5 の 4）、`$` で読むたびに下の規則で文字列やリストにする。
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

## JS の文（消した）

先頭の語がコマンドでない文を JavaScript とする「JS の文」（PLAN-ja.md の 2.5）は、段階 A3 で消した（PLAN-ja.md の 3.26）。代わりに、計算は `(( ))` と `$(( ))` の SJS で、ページの `document` などを使うときは `js '...'` で書く。

```
n = 1 + 2                      # 前は JS の文。今はコマンド n を探す（見つからない）
(( n = 1 + 2 ))                # SJS変数 n
js 'document.title = "x"'      # DOM など、ホストの JavaScript を使うとき
```

## テスト

```
consh/js/test/run_all.sh                   # js と SJS のテスト（Node.js。各 test.sh が CONSH_ENABLE_JS を js か sjs にする）
CONMA_VM=wasm test/run_all.sh              # ConMa のテストを wasm 版で
CONMA_VM=wasm CONSH_IMPL=conma consh/test/run_all.sh
```

wasm 版で ConMa と ConSh のテストを流すと、外部コマンド、プロセス、`__VProc_spawn__`、シンボリックリンクを使うケースだけがネイティブ版と違う（PLAN-ja.md の 3.1〜3.4）。
