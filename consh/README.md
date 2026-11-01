# ConSh

ConSh は ConMa 言語で動作するシェルである。仕様と設計は [PLAN-ja.md](PLAN-ja.md) にある。

## 構成

```
consh/
├── PLAN-ja.md           仕様と設計
├── bin/
│   ├── consh            ConMa 版を起動する（includer → resolver → vproc_run.py --main）
│   └── consh_parse      Python の参照実装：構文解析と語の展開の結果をテキストで出す
├── include/
│   ├── consh_main.se    main：ConSh の入口
│   ├── consh_parser.se  構文解析
│   └── consh_eval.se    実行
└── test/
    ├── run_all.sh       すべての test.sh を実行する
    ├── common.sh        各 test.sh が共通に使うパスと関数
    ├── parse1/          文法のテスト：caseN.sh を consh_parse に渡し、caseN.log と比べる
    ├── expand1/         語の展開のテスト：consh_parse --expand に渡す。caseN.args の各行が引数になる
    └── stepN/           ConMa 版の実行のテスト：test.sh, caseN.log
```

* ConSh のために ConMa に足すプリミティブやライブラリは、`consh/` ではなく、リポジトリのルートの `docs/`、`include/`、`test/` に置く。
* `include/` のファイルには、ルートの `include/` のファイルと名前がぶつからないよう、`consh_` を付ける。
* 大きくなったら、`consh_lexer.se`、`consh_expand.se`、`consh_builtin.se` などに分ける。

## 実行

```
bin/consh [引数 ...]
```

構文木と展開の結果を見るには、Python の参照実装を使う。出力の形式は `bin/consh_parse` の先頭に書いてある。

```
bin/consh_parse スクリプト
bin/consh_parse --expand スクリプト [引数 ...]
```

## テスト

```
test/run_all.sh
```

diff の出力がなければ成功である。
