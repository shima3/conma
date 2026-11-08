# consh/include：ConMa 版 ConSh の高速版

ConMa 版の ConSh には、2 つの版がある。

* **参照版**（`consh/reference`）：仕様の参照実装。読みやすさを優先して書く。
* **高速版**（このフォルダ）：参照版のうち、速くするために書き直したファイルだけを置く。

高速版は、このフォルダ、`consh/reference` の順に include ディレクトリを指定して作る（`--include consh/include --include consh/reference`）。
include は、include ディレクトリを指定の順に探すので（docs/INCLUDER.md）、このフォルダにあるファイルは、どのファイルから include されても、`consh/reference` の同じ名前のファイルの代わりになる。
ここにないファイルは `consh/reference` のものを使う。最初のファイル（`consh_main.se`）も、ここになければ `consh/reference` のものを使う。

`bin/consh` は、`CONSH_IMPL=conma` で高速版、`CONSH_IMPL=reference` で参照版を動かす（既定は Rust 版）。

* 機能を足したり変えたりしたときは、参照版を直し、ここにあるファイルにも同じ変更をして、両方でテストする
  （`CONSH_IMPL=conma` と `CONSH_IMPL=reference` で `consh/test/run_all.sh`）。
* 速くしただけ（動きは変えない）なら、高速版のテストだけでよい。

ここにファイルを書き写すときは、参照版のどの部分をどう速くしたかを、そのファイルの先頭に書く。
