#!/bin/bash
# consh/examples の各デモを ConSh で、比べる Bash 版を /bin/bash で実行し、
# 出力と終了ステータスを NAME.log、NAME.bash.log と比べる
cd ${0%/*}
. ../common.sh
examples="$consh/examples"
unset MYAPP_CACHE
for c in spaces logsummary lazy typo
do
  (cd "$examples" && PYTHONUNBUFFERED=1 "$consh/bin/consh" $c.sh < /dev/null > $tmp/$c.log 2>&1; echo "exit: $?" >> $tmp/$c.log)
  diff $c.log $tmp/$c.log
  (cd "$examples" && /bin/bash $c.bash < /dev/null > $tmp/$c.bash.log 2>&1; echo "exit: $?" >> $tmp/$c.bash.log)
  diff $c.bash.log $tmp/$c.bash.log
done
# nbody は Bash 版がない（Bash に小数はない）。100 段で、Node.js で consh/sjs/bench/nbody.safe.js を動かした出力と同じ
(cd "$examples" && "$consh/bin/consh" nbody.sh 100 < /dev/null > $tmp/nbody.log 2>&1; echo "exit: $?" >> $tmp/nbody.log)
diff nbody.log $tmp/nbody.log
