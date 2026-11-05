#!/bin/bash
# consh/examples の各デモを ConSh で、比べる Bash 版を /bin/bash で実行し、
# 出力と終了ステータスを NAME.log、NAME.bash.log と比べる
cd ${0%/*}
. ../common.sh
examples="$consh/examples"
unset MYAPP_CACHE
for c in spaces logsummary lazy typo
do
  (cd "$examples" && PYTHONUNBUFFERED=1 "$consh/bin/consh" $c.sh < /dev/null > /tmp/consh.$c.log 2>&1; echo "exit: $?" >> /tmp/consh.$c.log)
  diff $c.log /tmp/consh.$c.log
  (cd "$examples" && /bin/bash $c.bash < /dev/null > /tmp/consh.$c.bash.log 2>&1; echo "exit: $?" >> /tmp/consh.$c.bash.log)
  diff $c.bash.log /tmp/consh.$c.bash.log
done
