# ConSh の版と動かし方

ConSh には、実装、VM、構文解析、動かす場所の 4 つの選び方があり、その説明は consh/README-ja.md の「実行」、consh/js/README-ja.md、consh/bench/README-ja.md などに分かれている。この文書は、それを 1 か所にまとめたものである。

ここに書いたことは、2026-11-15 に alpha（92a959f）を実際に動かして確かめた。環境は macOS（Apple Silicon）、Python 3.9.6、Node.js v26.13.0、rlwrap 0.48 である。確かめていないことは、最後の節に分けて書いた。

## 4 つの軸

★ は既定。

| 軸 | 指定のしかた | 選択肢 | 内容 |
|---|---|---|---|
| 実装 | 環境変数 `CONSH_IMPL` | `rust` ★ | Rust 版（`consh/rust` の `consh-rs`）。構文解析も Rust 版。`consh/rust` で `cargo build --release` して作っておく。作っていなければ、そう出して ConMa 版（高速版）で動かす |
| | | `conma` | ConMa 版の高速版（`consh/include`） |
| | | `reference` | ConMa 版の参照版。`consh/reference`、標準ライブラリの `include/core`（コア ConMa）と `reference` のファイルを、高速版の同じ名前のファイルの代わりに使う |
| VM（ConMa 版を動かすもの） | 環境変数 `CONMA_VM`（`test/vm.sh`） | `python` ★ | 基準の Python 版の VM |
| | | `rust` | Rust 版の VM（`vm/target/release/conma`。`vm` で `cargo build --release`） |
| | | `transpiled` | ConMa から Rust に変換したコードを組み込んだ Rust 版の VM（`vm/target-transpiled/release/conma`。`bench/build.sh` で作る） |
| | | `wasm` | wasm32 版の VM を Node.js で動かす（`consh/js/vm-wasm.sh`。`consh/js/build.sh` で作る） |
| 構文解析 | オプション | `--conma-parser` ★ | ConMa 版の構文解析（`consh_parser.se`）。ConMa 版の既定で、付けても明示するだけ |
| | | `--python-parser` | Python の `consh_parse --pos` が作った構文木を読む |
| 動かす場所 | 起動のしかた | ターミナル ★ | `consh/bin/consh スクリプト [引数 ...]`、引数なしで対話モード |
| | | ブラウザ | `consh/js/serve.py` で `consh/js/web` を出し、http://localhost:8765/ を開く。対話モードだけ |
| | | Node.js | `CONMA_VM=wasm CONSH_IMPL=conma consh/bin/consh ...` |

* `--python-parser`、`--conma-parser`、`CONSH_CONMA_DATA` は ConMa 版にしかないので、付ければ `CONSH_IMPL` が `rust`（既定）でも ConMa 版（高速版）で動く。
* ConMa 版のモジュールは、Rust 版の VM があれば、その `conma build` で作る。そのため、`CONMA_VM=rust`（か `transpiled`）なら Python を使わない。

## 早見表

| したいこと | コマンド |
|---|---|
| ふつうに動かす（速い） | `consh/bin/consh script.sh` |
| 対話モード | `consh/bin/consh` |
| `(( ))` と `$(( ))` を使う | `CONSH_IMPL=conma CONMA_VM=rust consh/bin/consh script.sh` |
| ConMa 版を Python を使わずに動かす | `CONSH_IMPL=conma CONMA_VM=rust consh/bin/consh script.sh` |
| ConMa 版を基準の VM で動かす | `CONSH_IMPL=conma consh/bin/consh script.sh` |
| 参照版で動かす | `CONSH_IMPL=reference CONMA_VM=rust consh/bin/consh script.sh` |
| `js` を使う（Node.js） | `CONSH_ENABLE_JS=js CONMA_VM=wasm CONSH_IMPL=conma consh/bin/consh script.sh` |
| ブラウザで使う | `consh/js/build.sh` の後に `consh/js/serve.py`、http://localhost:8765/ を開く |
| 構文木を見る | `consh/bin/consh --dump script.sh`（Rust 版）、`CONSH_IMPL=conma CONMA_VM=rust consh/bin/consh --dump script.sh`（ConMa 版） |

## 版ごとの機能の差

関数、パイプ、外部コマンド、`$( )` を使うスクリプトで確かめた。

| | Rust 版 | ConMa 版（Python、Rust、変換版の VM） | ConMa 版（wasm、Node.js） | ConMa 版（wasm、ブラウザ） |
|---|---|---|---|---|
| 関数、パイプ、`$( )` | 動く | 動く | 動く | 動く（外部コマンドはなく、`cat`、`pwd`、`ls` は組み込み） |
| `(( ))` と `$(( ))`（SJS） | 構文エラー（`expected a command, but found '('`） | 動く | 動く | 動く |
| `js` | `js: JavaScript is not available`、`$?` は 127 | 同じ（127） | 既定では `js: JavaScript is disabled ...`、`$?` は 2。`CONSH_ENABLE_JS=js` で動く | 既定では使えない（`$?` は 2） |
| 対話モード | 動く | 動く | — | 動く |
| `--dump` | Rust 版の構文木（`(( ))` は構文エラー） | ConMa 版の構文木 | — | — |

* 参照版（`CONSH_IMPL=reference`）も、高速版と同じ結果になった（Python 版と Rust 版の VM）。
* `(( n = 6 * 7 ))` の `n` は SJS の変数で、`$(( n ))` で読む。`$n` では読めない（`undefined variable: n`）。シェル変数は、`(( ))` の中で `$k` として読める。
* 対話モードでは、`(( ))` が式の値を出す（`(( n = 6 * 7 ))` で `42`）。スクリプトでは出さない。

## 必要なもの

| もの | 要るとき | 確かめたこと |
|---|---|---|
| Rust | Rust 版の VM（`vm`）、wasm 版、変換版の VM | Rust 1.75 でビルドできた（`vm/Cargo.toml` の `rust-version = "1.75"`） |
| Rust | ConSh の Rust 版（`consh/rust`） | Rust 1.87 でビルドでき、1.86 では断られた（`rust-version`） |
| `wasm32-unknown-unknown` | wasm 版（`rustup target add wasm32-unknown-unknown`） | `consh/js/build.sh` で作れた |
| Python 3 | Python 版の VM、`bin/` の道具、`--python-parser`、`consh/js/serve.py` | 3.9.6 で動いた。これより古い版は試していない |
| Node.js | wasm 版を Node.js で動かすとき | v26.13.0 で動いた。これより古い版は試していない |
| rlwrap | 対話モードで、行の編集と履歴を使うとき（なくても動く） | 0.48 |

## そのほかの環境変数

| 環境変数 | 意味 |
|---|---|
| `CONSH_ENABLE_JS` | `js` なら `js` コマンドと、段階 A の `(( ))` を使える。`sjs` なら段階 A の `(( ))` だけ。Node.js 版で使う（ブラウザ版は `config.json`） |
| `CONSH_CONMA_DATA` | 空でなければ、データの組み込み（`__Dict_copy__` など）の代わりに ConMa 版（`include/data.se`）を使う。モジュールは `consh.data.module.se` として別に作る |
| `CONSH_NO_RLWRAP` | 空でなければ、対話モードで rlwrap を使わない |
| `CONSH_MODULE_DIR` | ConSh のモジュールを作るディレクトリ |
| `CONMA_CACHE_DIR` | モジュールを置くディレクトリ（`bin/cache-dir`）。既定は `${XDG_CACHE_HOME:-$HOME/.cache}/conma/<リポジトリのパスのチェックサム>` |

`CONSH_CONMA_DATA` を付けると、キャッシュのディレクトリに `consh.data.module.se` ができることを確かめた。

## ここで確かめていないこと

* ブラウザ版で `config.json` に `{"CONSH_ENABLE_JS": "js"}` を書いて `js` を使うこと（consh/js/README-ja.md に書かれている）。
* 標準入力が端末のときの rlwrap（行の編集、履歴、Tab の補完）。対話モードは、標準入力をパイプにして確かめた。
* `CONSH_ENABLE_JS=sjs` での段階 A の `(( ))`（モジュールを作れないときだけ使う）。
* Linux と Windows。
