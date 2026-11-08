#!/bin/bash
# Compare the ConMa includer (include/includer.se) with bin/includer.
# For each run, the exit codes, the result of selfhost_compare.py (when both succeed),
# and the error messages of both sides (sorted, absolute paths written as <tmp> in the temporary directory,
# <dir> in the test directory and <root> in the repository) are printed.
. "$(dirname "$0")/../common.sh"
resolve test.se
dir="$(pwd -P)"
tmpreal="$(cd "$tmp"; pwd -P)"
# The absolute paths of the temporary directory, the test directory and the repository
paths() {
  sed -e "s|$tmpreal|<tmp>|g" -e "s|$tmp|<tmp>|g" -e "s|$dir|<dir>|g" -e "s|$root|<root>|g"
}
# include source include-dir...
include() {
  echo "== $*" | paths
  local source=$1
  shift
  local opts=""
  for d in "$@"; do opts="$opts --include $d"; done
  ../../bin/includer $opts $source > $tmp/selfhost6.py.txt 2> $tmp/selfhost6.py.err
  py=$?
  PYTHONUNBUFFERED=1 ../vm.sh --main $tmp/test.module.se $source "$@" > $tmp/selfhost6.conma.txt 2> $tmp/selfhost6.conma.err
  echo "exit: python $py, conma $?"
  if [ $py = 0 ]
  then
    python3 ../selfhost_compare.py ast $tmp/selfhost6.py.txt $tmp/selfhost6.conma.txt
    echo "compare: $?"
    echo "-- files"
    grep -o '^(Program ([0-9]* [0-9]* "[^"]*"' $tmp/selfhost6.conma.txt | sed -e 's/^(Program ([0-9]* [0-9]* "//' -e 's/"$//' | paths
  fi
  echo "-- python stderr"
  paths < $tmp/selfhost6.py.err | LC_ALL=C sort
  echo "-- conma stderr"
  paths < $tmp/selfhost6.conma.err | LC_ALL=C sort
}
# link.se is a symbolic link to a.se (in the repository); $tmp/work/abs*.se include files by absolute paths
setup() {
  mkdir $tmp/work
  echo "(include \"$dir/lib/lib.se\")" > $tmp/work/abs.se
  echo "(include \"$dir/sub/../lib/./c2.se\")" >> $tmp/work/abs.se
  echo "(include \"$dir/none.se\")" > $tmp/work/abs_missing.se
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
  include $tmp/work/abs.se
}
# case4: errors: an included file not found, a syntax error, a missing source file,
# a missing include directory, an absolute path that does not exist
case4() {
  include missing.se lib
  include syntax.se lib
  include none.se
  include main.se nodir
  include $tmp/work/abs_missing.se
}
# case5: lexical errors in an included file; a real source with the include directory
case5() {
  include lexerr.se lib
  include ../step14/test.se ../../include
}
setup
for c in case1 case2 case3 case4 case5
do
  $c > $tmp/$c.log 2>&1
  diff $c.log $tmp/$c.log
done
