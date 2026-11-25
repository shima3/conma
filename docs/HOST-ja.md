# ホストの関数を足す

ConMa の VM を動かすプログラム（ホスト）は、ConMa や ConJS のプログラムから呼べる関数を足せる。ブラウザの canvas に描く、Rust のライブラリを使う、アプリの中のデータを読む、といったことを、言語に手を入れずに、そのプラットフォームでできるようにするための口である。この文書は、使う人のために、呼び方と足し方を 1 か所にまとめたもの（段階 P7。設計は [dev/PLATFORM-PLAN-ja.md](dev/PLATFORM-PLAN-ja.md) の 3.2 節、3.4 節、4.9 節）。

## 1. 呼ぶ側

ホストの関数は、`"canvas.line_to"` のような名前で呼ぶ。関数には 2 種類ある。

* **すぐ返る関数**：呼ぶと、その場で値が返る。
* **待つ関数**：結果が届くまで、呼んだ処理だけが待つ。待つ間、ほかの処理（ConMa の VProc、ConJS の `spawn` した処理）は動く。

関数の集まりには、**機能の名前**（`"canvas"`、`"demo"` など）がある。ホストによって、ある関数とない関数があるので、プログラムは、使う前に機能があるかを確かめ、なければ別の道をとる。

| | ConJS | ConMa |
|---|---|---|
| すぐ返る関数 | `host.call(name, ...args)` | `__Host_call__ onError name args ,(result)` |
| 待つ関数 | `host.wait(name, ...args)` | `Host_wait onError name args ,(result)`（include/platform/host.se） |
| 機能があるか | `host.available(feature)` | `__Feature_available__ feature ,(Boolean)` |

ConJS の例（conjs/examples/host_demo.con.js の一部）：

```js
if (host.available("demo")) {
  console.log(host.call("demo.add", 1, 2, 3.5));             // 6.5
  const slow = spawn(() => host.wait("demo.delay", 300, "slow"));
  console.log(host.wait("demo.delay", 50, "fast"));          // この処理だけが待つ
  console.log(join(slow));
  console.log(host.call("demo.info", "abc", ["x", "y"]));    // { name: 'abc', count: 2, short: true }
}
```

ホストにない関数、関数の失敗、渡せない値は、ConJS では `Error`（渡せない値は `TypeError`）を投げ、ConMa では `onError` に行く。メッセージは、ホストが返したもの。ConMa のプリミティブの詳しいことは [PRIMITIVES.md](PRIMITIVES.md) の `__Host_call__`、`__Host_start__`、`__Host_result__`。

## 2. 渡せる値

| ConMa | ConJS | JavaScript（wasm 版のホスト） | Rust（`HostValue`） |
|---|---|---|---|
| Number | 数 | number（-0、NaN、無限大も） | `Num(f64)` |
| String | 文字列 | string | `Str(String)` |
| Boolean | 真偽 | boolean | `Bool(bool)` |
| Dict（キーは String） | オブジェクト（自分のプロパティだけ） | ふつうのオブジェクト | `Dict(Vec<(String, HostValue)>)` |
| LList（null は空の LList） | 配列 | 配列 | `List(Vec<HostValue>)` |

* 真偽と Dict は、段階 P7 から渡せる。
* ConJS のオブジェクトは、自分のプロパティ（`Object.keys` と同じ）だけを渡す。クラスのインスタンスも、メソッドは渡らない。値が関数、`null`、`undefined` のプロパティがあると渡せない（`TypeError`。JavaScript の `JSON.stringify` は `undefined` のプロパティを飛ばすが、ConJS は飛ばさない）。
* 自分を含む値（`o.self = o` のようなオブジェクトや配列、ConMa の Dict）は渡せない（ConJS では `TypeError`、ConMa では `onError`）。同じ値を 2 か所に持つだけなら渡せ、ホストには 2 つの別の値として届く。
* JavaScript のホストが返せるオブジェクトは、ふつうのオブジェクト（`{ ... }`）だけ。`Date` などは、関数のエラーになる。
* ConMa では、null と空の LList は同じ値なので、ホストが返した空の配列と `null` と `undefined` は区別できない（ConJS では、どれも `undefined` になる）。
* 結果の大きさに限りはない（Rust 版、transpiled VM、Python 版の VM は、段階 P7 の前は 8 KB までだった）。

## 3. 足す側

### 3.1. ブラウザ（wasm 版の VM）

`ConMaVM.create` の hooks に、`hostFunctions`（名前から関数）と `hostFeatures`（機能の名前の配列）を渡す（vm/wasm/conma.js の先頭のコメント）。

```js
const vm = ConMaVM.create(wasmModule, write, {
  hostFunctions: {
    "demo.add": (...args) => args.reduce((s, a) => s + a, 0),                        // すぐ返る
    "demo.delay": (ms, text) => new Promise((r) => setTimeout(() => r(text), ms)),   // 待つ
  },
  hostFeatures: ["demo"],
});
```

* 関数は、値を返すか、例外を投げる。例外のメッセージが、呼んだ側のエラーになる。
* **Promise を返す関数**は、待つ関数になる。`host.wait`（`__Host_start__`）で呼ぶ。`host.call` で呼ぶとエラー。
* 見本：vm/wasm/host/canvas.js（機能 `"canvas"`）、vm/wasm/host/input.js（機能 `"input"`）と、それを使うページ（ConMa の vm/wasm/web と、ConJS の conjs/web。2026-11-23 に分けた）。
* どのホストにも、`time.now`、`time.sleep`、`http.fetch`（`fetch` があれば）がある。同じ名前を `hostFunctions` に入れると、そちらが使われる。

### 3.2. Node.js（wasm 版の VM）

環境変数 `CONMA_HOST` に、`hostFunctions` と `hostFeatures` を exports する CommonJS のモジュールを渡す（vm/wasm/vm-node.js）。

```sh
CONMA_HOST=conjs/examples/host_demo.host.js conjs/conjs conjs/examples/host_demo.con.js
```

関数の書き方は、ブラウザと同じ（conjs/examples/host_demo.host.js）。待つ関数の Promise がいつまでも決まらず、Node.js にほかにすることがなくなると、`vm-node: every VProc waits for host calls that never finish: N` を出して、終了ステータス 2 で終える。

関数の中では、Node.js のモジュールも npm のライブラリも `require` で使える。次の例は、Node.js に入っている `node:crypto` の `createHash` を呼ぶ（npm から何も入れずに動く。npm のライブラリは、`npm install` した後、同じく `require` する）。

```js
// hash.host.js
const { createHash } = require("node:crypto");
module.exports = {
  hostFunctions: {
    "hash.sha256": (text) => createHash("sha256").update(text).digest("hex"),
  },
  hostFeatures: ["hash"],
};
```

```js
// hash.con.js：CONMA_HOST=hash.host.js conjs/conjs hash.con.js
if (host.available("hash")) console.log(host.call("hash.sha256", "abc")); // ba7816bf...
```

**async/await のコードを任せる**（段階 P10、2026-11-25 から）：ConJS は `async`、`await`、`Promise` を受け付けない（待つ呼び出しは、その場で待つ。conjs/README-ja.md）。async/await を使う JavaScript のコード（Node.js の `fs.promises` や、Promise を返す npm のライブラリ）は、ホストの関数にする。async 関数は Promise を返すので、そのまま待つ関数になり、ConJS から `host.wait` で呼べる。待つのは呼んだ処理だけで、ほかの処理は動き続ける。ConJS を動かすのは wasm 版の VM だけなので（docs/dev/PLATFORM-PLAN-ja.md の 4.13 節）、Rust で書き直すことはない。

2026-11-26 からは、同じことを、ConJS のファイルの中にも書ける（docs/dev/CONJS-HOST-PLAN.md の 34 節。conjs/README-ja.md の「言語」）。ファイルの一番外の `async function` は JavaScript で、変換器がそれをホストの関数にし、ConJS のコードは `host.wait` を書かずに、ふつうの呼び出しとして呼ぶ。`CONMA_HOST` のファイルは、いくつものプログラムで使うホストのコードと、JavaScript でない（Rust の）ホストの関数のために残る。

```js
// files.con.js：conjs/conjs files.con.js
async function countLines(name) {                    // ホストの JavaScript で動く
  const fs = require("node:fs/promises");          // require は、このファイルの場所から探す
  const text = await fs.readFile(name, "utf8");
  return text.split("\n").filter((line) => line !== "").length;
}
const t = spawn(() => countLines("files.con.js"));  // 待つ呼び出し。ほかの処理は動き続ける
console.log(join(t));
```

```js
// files.host.js
const fs = require("node:fs/promises");
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
module.exports = {
  hostFunctions: {
    "files.countLines": async (name) => {
      const text = await fs.readFile(name, "utf8");
      return text.split("\n").filter((line) => line !== "").length;
    },
    "files.later": async (ms, text) => {
      await delay(ms);
      return text;
    },
  },
  hostFeatures: ["files"],
};
```

```js
// files.con.js：CONMA_HOST=files.host.js conjs/conjs files.con.js
const slow = spawn(() => host.wait("files.later", 300, "slow"));   // 別の処理で待つ
console.log(host.wait("files.later", 10, "fast"));                  // fast（slow より先）
console.log(host.wait("files.countLines", "files.con.js"));
console.log(join(slow));                                            // slow
```

渡せるのは値だけで（2 節）、関数は渡せない。そのため、ホストの async のコードから ConJS の関数を呼ぶ形（受け手を渡す形）は書けない。入力のように何度も届くものは、input.js のように、ホストが列にためて、ConJS が待つ関数で 1 つずつ取りに行く形にする。テストは conjs/test の step21（case1.host.js）。

vm-node.js は、`CONMA_HOST` がなくても、ネットワークのサーバの関数（機能 `"server"`、vm/wasm/host/server.js。段階 P6b）を渡す。ConJS では `http.listen`（conjs/README-ja.md）で使う。`CONMA_HOST` に同じ名前の関数があれば、そちらが使われる。

### 3.3. Rust 版の VM と transpiled VM

ConMa のプログラムのための口である。ConJS は、段階 P10（2026-11-25）から wasm 版の VM だけで動かすので、ここで足した関数を ConJS から使うことは目標にしない（`CONMA_VM=rust` を付ければ、今も動く。test/host-api.sh）。

VM を組み込む Rust のプログラムを書き、vm/ のクレート `conma` をライブラリとして使う。`Host::new()` に関数を登録して、`conma::run_main_with` で動かす。引数は `conma` のコマンドと同じ（`-m モジュール 引数 ...`、`build`、`run` など）なので、できたプログラムは、関数を足した `conma` として使える。

```rust
use conma::{Host, HostValue};

#[global_allocator]
static GLOBAL: conma::Pool = conma::Pool::new();   // VM のアロケータ（ないと遅い）

fn main() {
    let mut host = Host::new();                     // test.* と time.* は、はじめから入っている
    host.function("demo.add", |args| {              // すぐ返る関数
        let mut sum = 0.0;
        for a in args {
            match a {
                HostValue::Num(x) => sum += x,
                _ => return Err("demo.add: expected Numbers".to_string()),
            }
        }
        Ok(HostValue::Num(sum))
    });
    host.waiting("demo.delay", |args, done| {       // 待つ関数：done で後から結果を返す
        let ms = match args { [HostValue::Num(ms), ..] => *ms, _ => 0.0 };
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs_f64(ms / 1000.0));
            done.finish(Ok(HostValue::Str("late".into())));
        });
    });
    host.feature("demo");
    std::process::exit(conma::run_main_with(std::env::args().skip(1), host));
}
```

* **`Host::function(name, f)`**：すぐ返る関数。`f` は `Fn(&[HostValue]) -> Result<HostValue, String>`。VM のスレッドで動き、その間 VM は止まるので、時間のかかる仕事は待つ関数にする。
* **`Host::waiting(name, f)`**：待つ関数。`f` は `Fn(&[HostValue], Done)`。`f` 自身はすぐ戻り、仕事を別のスレッドなどで進めて、終わったら `done.finish(Ok(値))` か `done.finish(Err(メッセージ))` を 1 回呼ぶ。`Done` は、どのスレッドに渡してもよい（`Send`）。`finish` を呼ばずに `Done` を捨てると、呼んだ側は `名前: the host dropped the call` のエラーを受け取る。VM が終わった後の `finish` は、何もしない。
* **`Host::feature(name)`**：`host.available(name)` を真にする。
* 関数は `Send + 'static` の閉包。同じ名前を 2 度登録すると、後のものになる。
* `Done` を持ったまま、いつまでも `finish` も捨てることもしないと、呼んだ処理は待ち続ける（VM には見つけられない）。
* Cargo.toml では、`conma = { path = ".../vm" }` を依存にする。**transpiled VM** を使うときは、vm/conma2rust/build.sh と同じく、変換したコードのファイルを環境変数 `CONMA_TRANSPILED` に入れて作る。
* 見本：vm/examples/host_demo.rs（`cargo build --release --example host_demo`）。`bin/lang` で ConJS のプログラムを、この実行ファイルで動かすには、`CONMA_RUNNER` にその道を入れる：

```sh
CONMA_RUNNER=vm/target/release/examples/host_demo bin/lang conjs conjs/examples/host_demo.con.js
```

### 3.4. Python 版の VM

Python 版の VM（テストの基準の VM）には、関数を足す口はない。ConJS は、段階 P10 から、Python 版の VM では動かさない。どの VM にもある `test.*` と `time.*` だけを持つ。

## 4. どの VM にもある関数

テストのための機能 `"test"`（`test.add`、`test.echo`、`test.fail`、仮の時計で待つ `test.later`）と、機能 `"time"`（`time.now`、`time.sleep`）。wasm 版の VM のホストには、`fetch` があれば `http.fetch`（機能 `"fetch"`）もあり、Node.js で動かすとき（vm-node.js）は、ネットワークのサーバの `server.listen`、`server.accept`、`server.respond`、`server.close`（機能 `"server"`）もある。詳しいことは [PRIMITIVES.md](PRIMITIVES.md) の `__Host_call__` と `__Host_start__`。

## 5. テスト

test/host-api.sh が、vm/examples/host_demo.rs を作り、conjs/examples/host_demo.con.js を Rust 版の VM と、同じ関数を JavaScript で書いた conjs/examples/host_demo.host.js を使う wasm 版の VM で動かして、同じログ（test/host-api/demo.log）になることを確かめる。
