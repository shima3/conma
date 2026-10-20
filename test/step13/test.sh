#!/bin/bash
cd ${0%/*}
for f in test.se args.se exit.se
do
  if ! ../resolver.sh $f
  then exit 1
  fi
done
# Standard input of each case
input() {
  case $1 in
    case2|case7) echo input.txt ;;
    case3) echo lines.txt ;;
    *) echo /dev/null ;;
  esac
}
for c in case1 case2 case3 case4 case5 case6 case7
do
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py /tmp/test.module.se $c < $(input $c) > /tmp/$c.log 2>&1
  echo "exit: $?" >> /tmp/$c.log
  diff $c.log /tmp/$c.log
done
# case8 to case10 start programs with --main
PYTHONUNBUFFERED=1 python3 ../vproc_run.py --main /tmp/args.module.se a "b c" 日本語 < lines.txt > /tmp/case8.log 2>&1
echo "exit: $?" >> /tmp/case8.log
diff case8.log /tmp/case8.log
PYTHONUNBUFFERED=1 python3 ../vproc_run.py --main /tmp/args.module.se < /dev/null > /tmp/case9.log 2>&1
echo "exit: $?" >> /tmp/case9.log
diff case9.log /tmp/case9.log
PYTHONUNBUFFERED=1 python3 ../vproc_run.py --main /tmp/exit.module.se < /dev/null > /tmp/case10.log 2>&1
echo "exit: $?" >> /tmp/case10.log
diff case10.log /tmp/case10.log
