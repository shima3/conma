[English](QUICKSTART.md) | **日本語**

# ConJS クイックスタート

ConJS を自分のパソコンで動かすまでの手順です。必要なツールを入れ、GitHub からソースを取ってきてビルドし、ConJS で書いた小さな Web サーバを Node.js の上で動かして、そのコードを読むところまで進みます。かかる時間は、ツールのダウンロードを含めて 30 分から 1 時間くらいです。

ConJS は、JavaScript のサブセット（一部分）です。JavaScript では `await` を付けて書く「待つ処理」を、ふつうの関数呼び出しとして書けます。言語の詳しいことは、この後の[チュートリアル](TUTORIAL-ja.md)で説明します。

## 0. 用意するもの

* Windows 10 か 11、または macOS のパソコン
* インターネットにつながっていること（ツールとソースのダウンロードに使います）

この文書では、コマンドを打つ画面（ターミナル）を使います。

* **macOS**：「ターミナル」（アプリケーション → ユーティリティ）を使います。
* **Windows**：次の 1 で入れる Git for Windows に付いてくる **Git Bash** を使います。PowerShell やコマンドプロンプトではなく、Git Bash で打ってください（この文書のコマンドは bash 用です）。

コマンドは、`$` の後ろの部分を打って Enter を押します（`$` は打ちません）。

## 1. ツールを入れる

使うツールは 3 つです。

| ツール | 何に使うか |
|---|---|
| Git | GitHub からソースを取ってくる |
| Node.js（22 以降） | ConJS のプログラムを動かす（wasm 版の VM を動かす）、Web サーバ |
| Rust | ConJS を動かす仕組み（ConMa の VM）をビルドする |

### macOS

1. **Git**：ターミナルで次を打ちます。開発者ツールを入れるか聞かれたら「インストール」を押します。すでに入っていれば、何も起きません。
   ```
   $ xcode-select --install
   ```
2. **Node.js**：<https://nodejs.org/> から LTS 版（22 以降）のインストーラをダウンロードして入れます。
3. **Rust**：ターミナルで次を打ち、聞かれたら Enter（既定のまま）を押します。
   ```
   $ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
   終わったら、ターミナルを一度閉じて開き直します（`cargo` コマンドを使えるようにするため）。

### Windows

1. **Git**：<https://git-scm.com/> から Git for Windows のインストーラをダウンロードして入れます。途中の設定は既定のままで構いません。これで Git Bash も入ります。
2. **Node.js**：<https://nodejs.org/> から LTS 版（22 以降）のインストーラをダウンロードして入れます。
3. **Rust**：<https://rustup.rs/> から `rustup-init.exe` をダウンロードして動かします。Visual Studio の C++ のビルドツールを入れるか聞かれたら、入れてください（Rust が Windows でプログラムを作るのに使います）。そのほかは既定のままで構いません。

終わったら、Git Bash を開き直します。

### 入ったか確かめる（共通）

ターミナル（Windows は Git Bash）で、次の 3 つを打ちます。どれも版の番号が出れば大丈夫です。

```
$ git --version
$ node --version
$ cargo --version
```

`node --version` は `v22` 以上である必要があります。`command not found` と出たら、ターミナルを開き直してから、もう一度打ってみてください。

最後に、Rust に WebAssembly 用の部品を足します（Node.js で ConJS を動かすのに使います）。

```
$ rustup target add wasm32-unknown-unknown
```

## 2. GitHub からソースを取ってくる（クローン）

作業用のフォルダに移ってから、ソースを取ってきます。ここでは、ホームフォルダの下にそのまま置きます。

```
$ cd ~
$ git clone -c core.autocrlf=false https://github.com/shima3/conma.git
$ cd conma
```

`-c core.autocrlf=false` は、Windows で改行の形を変えずに取ってくるための設定です（変えてしまうと、スクリプトが動きません）。macOS では付けても付けなくても同じです。

これで `conma` というフォルダができ、その中に移りました。この後のコマンドは、すべてこの `conma` フォルダの中で打ちます。ConJS の部分は `conjs` フォルダにあります。

## 3. ビルドする

ConJS のプログラムは、ConMa という言語に変換してから、ConMa の VM（仮想機械）で動かします。その VM を 2 つビルドします。

```
$ (cd vm && cargo build --release)
$ vm/wasm/build.sh
```

* 1 つ目は、パソコンの上で直接動く VM（Rust 版の VM）です。ConJS の変換にも使います。
* 2 つ目は、WebAssembly の VM（wasm 版の VM）です。Node.js の上で動き、Web サーバの見本はこちらで動かします。

初めてのビルドは、パソコンによって数十秒から数分かかります。`Finished` と出れば成功です。

## 4. 最初のプログラムを動かす

`hello.con.js` というファイルを作ります。エディタ（VS Code など）で作っても、次のコマンドで作っても構いません。

```
$ echo 'console.log("こんにちは、ConJS");' > hello.con.js
```

ConJS のファイルの名前は `.con.js` で終わるようにします（この文書とチュートリアルで作るファイルは、`git status` に「Untracked」と出ますが、害はありません。いらなくなったら消して構いません）。動かします。

```
$ conjs/conjs hello.con.js
こんにちは、ConJS
```

初めて動かすときは、変換の準備（変換器とランタイムを作ること）に、数秒から 30 秒くらいかかります（パソコンによっては、もっとかかります）。止まったように見えても、Ctrl-C を押さずに待ってください。2 回目からは、すぐに動きます。

## 5. Web サーバを動かす

ConJS で書いた小さな Web サーバの見本（`conjs/examples/counter_server.con.js`）を、Node.js の上で動かします。Web サーバは Node.js の上でだけ動くので、`CONMA_VM=wasm` を付けて、wasm 版の VM を使います。

```
$ CONMA_VM=wasm conjs/conjs conjs/examples/counter_server.con.js
listening on http://127.0.0.1:8000/
```

`listening on ...` と出たら、サーバが動いています。ブラウザで、次のアドレスを開いてみてください。

| アドレス | 答え |
|---|---|
| <http://127.0.0.1:8000/count> | 開くたびに 1、2、3 と数が増える |
| <http://127.0.0.1:8000/slow> | 0.2 秒待ってから `slow` と答える（その間も、ほかの要求に答える） |
| <http://127.0.0.1:8000/file?name=README.md> | `conma` フォルダの README.md の行の数を答える |
| <http://127.0.0.1:8000/stop> | `bye` と答えて、サーバが止まる |

サーバを止めるには、`/stop` を開くか、ターミナルで Ctrl-C を押します。

8000 番が使われていて `EADDRINUSE` と出たときは、後ろに別の番号を付けます（`... counter_server.con.js 8001`。アドレスも `8001` にします）。

## 6. サンプルのコードを読む

サーバのコードを、エディタで開いてみましょう（`conjs/examples/counter_server.con.js`）。ターミナルで見るなら、次のようにします。

```
$ less conjs/examples/counter_server.con.js
```

（`less` は q で終わります。）

いちばん大事なところは、要求を受け取るループです。

```js
function serve(server, dir) {
  for (;;) {
    const req = server.accept();          // 次の要求が来るまで待つ
    if (req === undefined) return;        // サーバが閉じられた
    spawn(() => handle(server, dir, req)); // 要求ごとに、新しい処理で答える
  }
}
```

`server.accept()` は、次の要求が来るまで待つ呼び出しです。JavaScript なら `await` が要るところですが、ConJS では、ふつうの関数呼び出しとして書けます。待っている間も、`spawn` で始めたほかの処理（ほかの要求への答え）は動き続けます。`/slow` の答えの中の `sleep(200)` も同じで、その処理だけが 0.2 秒止まります。

同じサーバを、ふつうの Node.js の JavaScript で書いたものが `conjs/examples/counter_server.node.js` にあります。こちらは、Node.js でそのまま動きます。

```
$ node conjs/examples/counter_server.node.js
```

2 つを並べて読むと、ConJS では `async` と `await` を書かずに済むことが分かります。

## 困ったとき

| 出たメッセージ | どうするか |
|---|---|
| `cargo: command not found` | ターミナルを開き直す。それでも出るときは、1 の Rust を入れ直す |
| `vm-wasm.sh: ... is not built` | 3 の `vm/wasm/build.sh` を動かす |
| `vm/wasm/build.sh` が、`wasm32-unknown-unknown` が入っていないというエラーで止まる | 1 の最後の `rustup target add wasm32-unknown-unknown` を打つ |
| 何も出ない。または、ファイルを開くアプリを聞かれる（Windows の PowerShell） | PowerShell で打っている。Git Bash を開いて打ち直す（0） |
| `'conjs' は、内部コマンドまたは外部コマンド、操作可能なプログラムまたはバッチ ファイルとして認識されていません。`（Windows のコマンド プロンプト） | コマンド プロンプトで打っている。Git Bash を開いて打ち直す（0） |
| `$'\r': command not found`（Windows） | 改行の形が変わってクローンされている。`conma` フォルダを消して、2 の `-c core.autocrlf=false` を付けてクローンし直す |
| `EADDRINUSE` | 8000 番が使われている。5 のとおり、別の番号を付ける |
| `http.listen: this platform has no server` | `CONMA_VM=wasm` を付け忘れている |

## 次に読むもの

* [チュートリアル](TUTORIAL-ja.md)：ConJS で書ける JavaScript の書き方と、JavaScript と違うところ
* [ConJS の README](README-ja.md)：ConJS の言語とライブラリのすべて
* ブラウザで動く ConJS の見本（ゲーム「星ひろい」など）：<https://shima3.github.io/conma/conjs/>
