# reference：標準ライブラリの参照版のうち、include と違うファイル

`include` は ConMa の標準ライブラリで、ふだんはそれだけを使う。速くするために書き直したファイルは、書き直す前のもの（読みやすさを優先したもの）をここに残す。
参照版の標準ライブラリは、このフォルダを `include` より先に探して作る（`--include reference --include include`。include は、include ディレクトリを指定の順に探す。docs/INCLUDER.md）。

* `util.se`：行の読み込み（`__LazyList_lines__` と `__Stream_lines__`）を、`include/util.se` は、読んだ塊を 1024 文字ずつの断片に切り、断片の中を位置で進めて行を取り出す。ここにあるものは、行を取り出すたびに塊の残り全体を写す。

* `ast.se`、`lexer.se`、`parser.se`、`resolver.se`、`comment_remover.se`、`token.se`、`includer.se`（ConMa 版のフロントエンド、`conma build`）：`include` の方は、次のように機械的に書き換えた。動きは変えていない（作るモジュールは、Python のツールと同じ。test/frontend）。
    * 条件が組み込みか `__and__`、`__or__`、`__not__` 1 つの関数（`__if__ (,() __is_null__ x) ...`）を、先に計算して渡す形（`__is_null__ x ,(test__) __if__ test__ ...`）にした。
    * `__AST_get_kind__`、`__AST_get_pos__`、`__AST_get_children__` の呼び出しを、中の `__LList_uncons__` に書き換えた（変数 `ast_kind__`、`ast_rest__`、`ast_pos__`、`ast_children__` を使う）。
    * `lexer.se` の `__Lexer_symbol_end__` は、ブロックコメントの終わり `|#` を、1 文字目が `|` のときだけ確かめる。

  `conma build` で ConSh のモジュール（約 7,700 関数）を作るのは、1 億 520 万ステップ、3.58 秒から、7,240 万ステップ、2.78 秒になった（Rust 版の VM。Python の `bin/includer` と `bin/resolver` は 1.32 秒）。フロントエンドのモジュール `vm/frontend.module.se` は、`include` のファイルで作る。

参照版で test/ を走らせるには、環境変数 `CONMA_REFERENCE` を空でない値にする（`test/resolver.sh` がこのフォルダを先に探す）。このとき、util.se そのものに依るログを持つ 4 つ（test/step20 の case15 と test/step18 の case12 はエラーの位置が `reference/util.se` の行になり、test/step21 の case5 と case6 は数える時点のステップがずれる）は、期待されるログと違う。ConSh の参照版（`CONSH_IMPL=reference`）も、このフォルダを先に探す。
機能を足したり変えたりしたときは、`include` のファイルと、ここにある同じ名前のファイルの両方を直し、両方でテストする。
