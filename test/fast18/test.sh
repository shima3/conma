#!/bin/bash
# ConJS の端末の入力の、キーに分ける所（conjs/include/conjs_term.se の Conjs_term_keys。docs/dev/PLATFORM-PLAN-ja.md の
# 4.7 節、段階 P6a）。ESC の並びが 2 回の読みに分かれて届く場合は、conjs/test の標準入力のファイルでは作れないので、
# ここで直に確かめる。どの VM でも同じログ
. "$(dirname "$0")/../common.sh"
"$root/bin/includer" --include "$root/conjs/include" --include "$root/include/ext" --include "$root/include" test.se \
  | sed -e 's/"\([A-Za-z]:\)\{0,1\}\/.*\/conma/"conma/' > $tmp/test.ast.se && CONMA_EXT=1 "$root/bin/resolver" < $tmp/test.ast.se > $tmp/test.module.se || exit 1
PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se main < /dev/null > $tmp/out.log 2>&1
echo "exit: $?" >> $tmp/out.log
diff test.log $tmp/out.log
