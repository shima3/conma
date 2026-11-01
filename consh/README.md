# ConSh

ConSh は ConMa 言語で動作するシェルである。仕様と設計は [PLAN-ja.md](PLAN-ja.md) にある。

## 構成

```
consh/
├── PLAN-ja.md           仕様と設計
├── AST.md               consh_parse が出力する構文木の形式（英語）
├── bin/
│   ├── consh            ConSh を起動する（consh_parse --pos で構文木を作り、ConMa 版に渡す）
│   └── consh_parse      Python の参照実装：構文解析と語の展開の結果をテキストで出す
├── include/
│   ├── consh_main.se    main：ConSh の入口
│   ├── consh_ast.se     consh_parse の構文木の読み込みと書き戻し
│   ├── consh_parser.se  構文解析（当面は consh_parse を使うので空）
│   └── consh_eval.se    実行
└── test/
    ├── run_all.sh       すべての test.sh を実行する
    ├── common.sh        各 test.sh が共通に使うパスと関数
    ├── parse1/          文法のテスト：caseN.sh を consh_parse に渡し、caseN.log と比べる
    ├── parse2/          構文木の位置のテスト：consh_parse --pos に渡す
    ├── expand1/         語の展開のテスト：consh_parse --expand に渡す。caseN.args の各行が引数になる
    └── stepN/           ConMa 版のテスト：test.sh, caseN.log。step1 は構文木の読み込み
```

* ConSh のために ConMa に足すプリミティブやライブラリは、`consh/` ではなく、リポジトリのルートの `docs/`、`include/`、`test/` に置く。
* `include/` のファイルには、ルートの `include/` のファイルと名前がぶつからないよう、`consh_` を付ける。
* 大きくなったら、`consh_lexer.se`、`consh_expand.se`、`consh_builtin.se` などに分ける。

## 実行

```
bin/consh [--dump] スクリプト [引数 ...]
```

ConMa 版は構文解析をせず、Python の `consh_parse --pos` が作った構文木のファイルを読む（形式は [AST.md](AST.md)）。
`--dump` を付けると、実行せず、ConMa 版が読んだ構文木を S 式に書き戻して出す。
ConMa のモジュールは `/tmp/consh.module.se` に作り、`.se` のソースが変わったときだけ作り直す。

構文木と展開の結果を見るには、Python の参照実装を使う。構文木の形式は [AST.md](AST.md)、展開の結果の形式は `bin/consh_parse` の先頭に書いてある。`--pos` を付けると、構文木に行と桁が入る。

```
bin/consh_parse [--pos] スクリプト
bin/consh_parse --expand スクリプト [引数 ...]
```

## テスト

```
test/run_all.sh
```

diff の出力がなければ成功である。
