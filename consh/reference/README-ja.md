# consh/reference：ConMa 版 ConSh の参照版のうち、高速版と違うファイル

ConMa 版の ConSh の参照版（仕様の参照実装）は、このフォルダのファイルと、`consh/include` のそれ以外のファイルからなる。
ここには、高速版（`consh/include`）で速くするために書き直したファイルの、書き直す前のもの（読みやすさを優先したもの）を置く。
`bin/consh` は、`CONSH_IMPL=reference` のとき、このフォルダを `consh/include` より先に探す（`consh/include/README-ja.md`）。
標準ライブラリも、リポジトリのルートの `reference`（`reference/README-ja.md`）を `include` より先に探す。

* 機能を足したり変えたりしたときは、`consh/include` のファイルと、ここにある同じ名前のファイルの両方を直し、両方でテストする。
* ここにあるファイルは、高速版の書き直しを調べるときの比べる相手にもなる。
