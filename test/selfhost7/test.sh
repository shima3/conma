#!/bin/bash
# Compare the ConMa resolver (include/resolver.se), after the ConMa includer, with
# bin/includer | bin/resolver. For each run, the exit codes and the result of
# selfhost_compare.py are printed, and the ConMa Module with the test directory written as <dir>.
cd ${0%/*}
if ! ../resolver.sh test.se 2> /tmp/selfhost7.err
then cat /tmp/selfhost7.err; exit 1
fi
dir="$(cd ..; pwd -P)"
# resolve source include-dir...
resolve() {
  echo "== $*"
  local source=$1
  shift
  local opts=""
  for d in "$@"; do opts="$opts --include $d"; done
  ../../bin/includer $opts $source 2> /dev/null | ../../bin/resolver > /tmp/selfhost7.py.txt
  py=$?
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py --main /tmp/test.module.se $source "$@" > /tmp/selfhost7.conma.txt 2> /dev/null
  echo "exit: python $py, conma $?"
  python3 ../selfhost_compare.py ast /tmp/selfhost7.py.txt /tmp/selfhost7.conma.txt
  echo "compare: $?"
}
# case1: de Bruijn indexes: several parameters, nested functions, shadowing,
# the same name twice in a Head, an LCont and a FuncExp, a free variable
case1() {
  resolve scope.se
  sed -e "s|$dir|<dir>|g" /tmp/selfhost7.conma.txt
}
# case2: global numbers: a name defined twice, a reference before the definition,
# __NULL__ as a global and as a parameter, an SInfo without Strings, an empty Body
case2() {
  resolve globals.se
  sed -e "s|$dir|<dir>|g" /tmp/selfhost7.conma.txt
}
# case3: an empty file: a Module with only F
case3() {
  resolve empty.se
  sed -e "s|$dir|<dir>|g" /tmp/selfhost7.conma.txt
}
# case4: several files: the includes of selfhost6 (link.se is made as in selfhost6/test.sh)
case4() {
  ln -sf a.se ../selfhost6/link.se
  resolve ../selfhost6/main.se ../selfhost6/lib ../selfhost6/lib2
  rm -f ../selfhost6/link.se
}
# case5: a real program with the standard library
case5() {
  resolve ../step14/test.se ../../include
}
for c in case1 case2 case3 case4 case5
do
  $c > /tmp/$c.log 2>&1
  diff $c.log /tmp/$c.log
done
rm -f /tmp/selfhost7.*
