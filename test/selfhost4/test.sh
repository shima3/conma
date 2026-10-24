#!/bin/bash
# Compare the ConMa comment_remover (include/comment_remover.se) with bin/comment_remover.
# Each case prints the result of selfhost_compare.py and the difference of the error messages.
# The ConMa lexer and comment_remover run together, so their error messages are mixed:
# both sides are sorted before they are compared.
cd ${0%/*}
if ! ../resolver.sh test.se 2> /tmp/selfhost4.err
then cat /tmp/selfhost4.err; exit 1
fi
remove() {
  echo "== $1"
  (../../bin/lexer $1 | ../../bin/comment_remover > /tmp/selfhost4.py.txt) 2>&1 | LC_ALL=C sort > /tmp/selfhost4.py.err
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py --main /tmp/test.module.se $1 > /tmp/selfhost4.conma.txt 2> /tmp/selfhost4.conma.err
  echo "exit: $?"
  python3 ../selfhost_compare.py tokens /tmp/selfhost4.py.txt /tmp/selfhost4.conma.txt
  echo "compare: $?"
  LC_ALL=C sort /tmp/selfhost4.conma.err | diff /tmp/selfhost4.py.err -
  cat /tmp/selfhost4.py.err
}
# case1: S-expression comments and line comments
case1() {
  remove sexp.se
}
# case2: block comments, an unmatched |#
case2() {
  remove block.se
}
# case3: the end of the source: an S-expression comment not closed, #; at the end,
# a block comment not closed
case3() {
  remove sexp_open.se
  remove sexp_end.se
  remove ../selfhost3/open.se
}
# case4: other sources
case4() {
  remove ../selfhost2/sample.se
  remove ../selfhost3/edge.se
  remove ../../include/util.se
}
# case5: BLOCK_COMMENT_TEXT and BLOCK_COMMENT_END outside a block comment,
# made by hand because the lexer does not give them
case5() {
  echo "== tokens"
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py --main /tmp/test.module.se =tokens 2>&1
  echo "exit: $?"
}
for c in case1 case2 case3 case4 case5
do
  $c > /tmp/$c.log 2>&1
  diff $c.log /tmp/$c.log
done
rm -f /tmp/selfhost4.*
