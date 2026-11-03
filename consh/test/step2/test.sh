#!/bin/bash
cd ${0%/*}
. ../common.sh
# expandN.sh: 語の展開を consh_parse --expand と比べる。
# words は引数を (words ...) の形で出す外部コマンドで、consh_parse --expand の (cmd ...) の中の (words ...) と同じになるはず
for f in expand*.sh
do
  c=${f%.sh}
  args=()
  if [ -e $c.args ]; then
    while IFS= read -r line; do args+=("$line"); done < $c.args
  fi
  python3 "$consh/bin/consh_parse" --expand $f ${args[@]+"${args[@]}"} | sed -n 's/^(cmd .*\((words .*\))$/\1/p' > /tmp/consh.$c.ref
  PATH="$PWD:$PATH" "$consh/bin/consh" $f ${args[@]+"${args[@]}"} > /tmp/consh.$c.out 2>&1
  diff /tmp/consh.$c.ref /tmp/consh.$c.out
done
run case1 case1.sh < /dev/null
d=$(mktemp -d /tmp/consh.step2.XXXXXX)
run case2 case2.sh $d < /dev/null
rm -rf $d
CONSH_T=outer run case3 case3.sh < /dev/null
run case4 case4.sh < /dev/null
for c in case5 case6 case7 case8 case9 case10 case11 case12 case13
do
  run $c $c.sh < /dev/null
done
run case14 case14.sh a b < /dev/null
for c in case15 case16 case17 case18
do
  run $c $c.sh < /dev/null
done
