#!/bin/bash
# 印刷した SeqClosure の変数の名前（test.se）。どの VM でも同じログになる。ただし、数の定数を畳み込む VM（Rust、wasm、
# 変換版。edd5945）では、畳み込んだ変数は LVEnv にないので、case4 は case4.fold.log と比べる（Rust VM は、
# CONMA_NO_FOLD=1 なら case4.log と同じになることも確かめる）。このコードを入れた変換版の VM では、case4 の畳み込んだ
# 変数を Null として出す（もう読まない変数）ので違う。vm/target-transpiled には入っていない
. "$(dirname "$0")/../common.sh"
resolve test.se
check() {
  local c=$1 log=$1.log
  shift
  [ -f $c.fold.log ] && [ "${CONMA_VM:-python}" != python ] && [ -z "$CONMA_NO_FOLD" ] && log=$c.fold.log
  env "$@" PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $log $tmp/$c.log || echo "$c $*"
}
for c in case1 case2 case3 case4
do
  check $c
done
[ "$CONMA_VM" = rust ] && CONMA_NO_FOLD=1 check case4 CONMA_NO_FOLD=1
exit 0
