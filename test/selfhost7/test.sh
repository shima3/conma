#!/bin/bash
# Compare the ConMa resolver (include/resolver.se), after the ConMa includer, with
# bin/includer | bin/resolver --warn-arity. For each run, the exit codes, the result of
# selfhost_compare.py and the difference of the warnings (standard error) are printed,
# and the warnings and the ConMa Module with the test directory written as <dir>.
. "$(dirname "$0")/../common.sh"
resolve test.se
dir="$(cd ..; pwd -P)"
# resolve source include-dir...
resolve() {
  echo "== $*"
  local source=$1
  shift
  local opts=""
  for d in "$@"; do opts="$opts --include $d"; done
  ../../bin/includer $opts $source 2> /dev/null | ../../bin/resolver --warn-arity > $tmp/selfhost7.py.txt 2> $tmp/selfhost7.py.err
  py=$?
  PYTHONUNBUFFERED=1 ../vm.sh --main $tmp/test.module.se $source "$@" > $tmp/selfhost7.conma.txt 2> $tmp/selfhost7.conma.err
  echo "exit: python $py, conma $?"
  python3 ../selfhost_compare.py ast $tmp/selfhost7.py.txt $tmp/selfhost7.conma.txt
  echo "compare: $?"
  diff $tmp/selfhost7.py.err $tmp/selfhost7.conma.err
  sed -e "s|$dir|<dir>|g" $tmp/selfhost7.py.err
}
# case1: de Bruijn indexes: several parameters, nested functions, shadowing,
# the same name twice in a Head, an LCont and a FuncExp, a free variable; variadic Heads
case1() {
  resolve scope.se
  sed -e "s|$dir|<dir>|g" $tmp/selfhost7.conma.txt
  resolve variadic.se
  sed -e "s|$dir|<dir>|g" $tmp/selfhost7.conma.txt
}
# case2: global numbers: a name defined twice, a reference before the definition,
# __NULL__ as a global, as a parameter and as the name of a Definition, an SInfo without Strings, an empty Body
case2() {
  resolve globals.se
  sed -e "s|$dir|<dir>|g" $tmp/selfhost7.conma.txt
}
# case3: an empty file: a Module with only F
case3() {
  resolve empty.se
  sed -e "s|$dir|<dir>|g" $tmp/selfhost7.conma.txt
}
# case4: several files: the includes of selfhost6 (with its link.se, a symbolic link to a.se)
case4() {
  resolve ../selfhost6/main.se ../selfhost6/lib ../selfhost6/lib2
}
# case5: a real program with the standard library
case5() {
  resolve ../step14/test.se ../../include
}
# case6: warnings for too few arguments to a global function: with an SInfo from the file name,
# with an explicit SInfo; none for the same number, more arguments, no arguments,
# a variadic function, a local variable with the name of a global function, a name defined twice
case6() {
  resolve arity.se
}
for c in case1 case2 case3 case4 case5 case6
do
  $c > $tmp/$c.log 2>&1
  diff $c.log $tmp/$c.log
done
