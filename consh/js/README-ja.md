# JavaScript 版 ConSh

ConMa の VM を WebAssembly（wasm32）にビルドし、ブラウザか Node.js の上で ConMa 版の ConSh を動かす。組み込みコマンド `js` で JavaScript を呼べる。計画と設計は [PLAN-ja.md](PLAN-ja.md) にある。

## 作る

```
rustup target add wasm32-unknown-unknown   # 初めてのときだけ
consh/js/build.sh
```

`build.sh` は、VM を wasm32 でビルドし（consh/js/wasm、約 700 KB）、`web/` に wasm、`conma.js`、ConSh のモジュール（`consh.module.se`、約 5.8 MB）を置く。

## ブラウザで使う

`consh/js/web` を HTTP で開く。

```
python3 -m http.server 8765 --bind 127.0.0.1 --directory consh/js/web
```

http://localhost:8765/ を開くと、ConSh の対話モードが動く（`.claude/launch.json` の `consh-web` と同じ）。

* 外部コマンドはない。関数、組み込みコマンド、パイプ、`$( )`、`&` の関数のジョブ、ファイルへのリダイレクトは使える。
* カレントディレクトリは `/home/user` で、ファイルはメモリ上の仮想ファイルシステムにある（ページを閉じると消える）。「ファイルを置く」で手元のファイルをそこに置ける。
* 「スクリプトを実行」は、スクリプトの各行を入力として渡す（ConMa 版には `source` がない）。
* Ctrl-C か「中断」で、動いているコマンドを止める。

## Node.js で使う

`test/vm.sh` は、`CONMA_VM=wasm` のとき wasm 版の VM を Node.js で動かす（`vm-wasm.sh`、`vm-node.js`）。ConSh も同じく動かせる。

```
CONMA_VM=wasm CONSH_IMPL=conma consh/bin/consh script.sh
```

VM は実際のファイルシステムを読めるが、書いたファイルはメモリの中にだけある。外部コマンドはない。

## js

```
js [変数名=] コード {引数}
```

* コードを JavaScript の式として評価する。値が関数なら引数を渡して呼び、Promise なら解決するまで待つ（その間、ほかの VProc は動く）。
* 最初の語が「識別子 `=`」だけ（スペースの前に `=`）なら、結果をその局所変数に入れる。そうでなければ、結果を標準出力に出す。
* 引数の String は文字列、LazyList（`${list}`）は配列として渡す。
* 結果の文字列は値 1 つ、配列は要素ごとの値（変数には LazyList）、`undefined` と `null` は何も出さない（変数には空文字列）。そのほかの値は `String()` か `JSON.stringify` で文字列にする。
* 例外のときは `js: 名前: メッセージ` を出して `$?` を 1 にする。

```
js '1 + 2'                                          # 3
js sum= '(a, b) => Number(a) + Number(b)' $x $y     # sum に入れる
js '["a", "b"]' > (first . rest)
js text= 'fetch("data.txt").then(r => r.text())'    # ブラウザ
```

ネイティブの VM（Python 版、Rust 版）と Rust 版の ConSh では、`js` は `js: JavaScript is not available` を出して `$?` を 127 にする。

## テスト

```
consh/js/test/run_all.sh                   # js のテスト（Node.js）
CONMA_VM=wasm test/run_all.sh              # ConMa のテストを wasm 版で
CONMA_VM=wasm CONSH_IMPL=conma consh/test/run_all.sh
```

wasm 版で ConMa と ConSh のテストを流すと、外部コマンド、プロセス、`__VProc_spawn__`、シンボリックリンクを使うケースだけがネイティブ版と違う（PLAN-ja.md の 3.1〜3.4）。
