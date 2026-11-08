# reference：標準ライブラリの参照版のうち、include と違うファイル

`include` は ConMa の標準ライブラリで、ふだんはそれだけを使う。速くするために書き直したファイルは、書き直す前のもの（読みやすさを優先したもの）をここに残す。
参照版の標準ライブラリは、このフォルダを `include` より先に探して作る（`--include reference --include include`。include は、include ディレクトリを指定の順に探す。docs/INCLUDER.md）。

* `util.se`：行の読み込み（`__LazyList_lines__` と `__Stream_lines__`）を、`include/util.se` は、読んだ塊を 1024 文字ずつの断片に切り、断片の中を位置で進めて行を取り出す。ここにあるものは、行を取り出すたびに塊の残り全体を写す。

参照版で test/ を走らせるには、環境変数 `CONMA_REFERENCE` を空でない値にする（`test/resolver.sh` がこのフォルダを先に探す）。このとき、util.se そのものに依るログを持つ 3 つ（test/step20 の case15 はエラーの位置が `reference/util.se` の行になり、test/step21 の case5 と case6 は数える時点のステップがずれる）は、期待されるログと違う。ConSh の参照版（`CONSH_IMPL=reference`）も、このフォルダを先に探す。
機能を足したり変えたりしたときは、`include` のファイルと、ここにある同じ名前のファイルの両方を直し、両方でテストする。
