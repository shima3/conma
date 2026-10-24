#!/bin/bash
# Compare the ConMa includer (include/includer.se) with bin/includer.
# For each run, the exit codes, the result of selfhost_compare.py (when both succeed),
# and the error messages of both sides (sorted, absolute paths written as <dir>) are printed.
cd ${0%/*}
if ! ../resolver.sh test.se 2> /tmp/selfhost6.err
then cat /tmp/selfhost6.err; exit 1
fi
dir="$(pwd -P)"
# include source include-dir...
include() {
  echo "== $*"
  local source=$1
  shift
  local opts=""
  for d in "$@"; do opts="$opts --include $d"; done
  ../../bin/includer $opts $source > /tmp/selfhost6.py.txt 2> /tmp/selfhost6.py.err
  py=$?
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py --main /tmp/test.module.se $source "$@" > /tmp/selfhost6.conma.txt 2> /tmp/selfhost6.conma.err
  echo "exit: python $py, conma $?"
  if [ $py = 0 ]
  then
    python3 ../selfhost_compare.py ast /tmp/selfhost6.py.txt /tmp/selfhost6.conma.txt
    echo "compare: $?"
    echo "-- files"
    grep -o '^(Program ([0-9]* [0-9]* "[^"]*"' /tmp/selfhost6.conma.txt | sed -e 's/^(Program ([0-9]* [0-9]* "//' -e 's/"$//' -e "s|$dir|<dir>|g"
  fi
  echo "-- python stderr"
  sed -e "s|$dir|<dir>|g" /tmp/selfhost6.py.err | LC_ALL=C sort
  echo "-- conma stderr"
  sed -e "s|$dir|<dir>|g" /tmp/selfhost6.conma.err | LC_ALL=C sort
}
# link.se is a symbolic link to a.se, work/abs*.se include files by absolute paths
setup() {
  ln -sf a.se link.se
  rm -rf work
  mkdir work
  echo "(include \"$dir/lib/lib.se\")" > work/abs.se
  echo "(include \"$dir/sub/../lib/./c2.se\")" >> work/abs.se
  echo "(include \"$dir/none.se\")" > work/abs_missing.se
}
cleanup() {
  rm -rf work link.se
}
# case1: breadth-first order; duplicates by name, by "../" and by a symbolic link;
# the directory of the including file is searched before the include directories,
# which are searched in order
case1() {
  include main.se lib lib2
}
# case2: include directories in the other order; a file that includes itself
case2() {
  include main.se lib2 lib
  include self.se
}
# case3: absolute paths in include statements
case3() {
  include work/abs.se
}
# case4: errors: an included file not found, a syntax error, a missing source file,
# a missing include directory, an absolute path that does not exist
case4() {
  include missing.se lib
  include syntax.se lib
  include none.se
  include main.se nodir
  include work/abs_missing.se
}
# case5: lexical errors in an included file; a real source with the include directory
case5() {
  include lexerr.se lib
  include ../step14/test.se ../../include
}
setup
for c in case1 case2 case3 case4 case5
do
  $c > /tmp/$c.log 2>&1
  diff $c.log /tmp/$c.log
done
cleanup
rm -f /tmp/selfhost6.*
