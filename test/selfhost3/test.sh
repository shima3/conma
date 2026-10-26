#!/bin/bash
# Compare the ConMa lexer (include/lexer.se) with bin/lexer.
# Each case prints the result of selfhost_compare.py and the difference of the error messages.
cd ${0%/*}
if ! ../resolver.sh test.se 2> /tmp/selfhost3.err
then cat /tmp/selfhost3.err; exit 1
fi
lex() {
  echo "== $1"
  ../../bin/lexer $1 > /tmp/selfhost3.py.txt 2> /tmp/selfhost3.py.err
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py --main /tmp/test.module.se $1 > /tmp/selfhost3.conma.txt 2> /tmp/selfhost3.conma.err
  echo "exit: $?"
  python3 ../selfhost_compare.py tokens /tmp/selfhost3.py.txt /tmp/selfhost3.conma.txt
  echo "compare: $?"
  diff /tmp/selfhost3.py.err /tmp/selfhost3.conma.err
}
# case1: comments, strings, include and Japanese text; CRLF line endings
case1() {
  lex ../selfhost2/sample.se
  lex ../selfhost2/inc.se
}
# case2: edge cases: |#, unexpected characters, escapes, unterminated strings,
# lone CR, nested block comments over several lines, no newline at the end
case2() {
  lex edge.se
}
# case3: an unterminated block comment and an empty file
case3() {
  lex open.se
  lex empty.se
}
# case4: real sources
case4() {
  lex ../../include/util.se
  lex ../step14/test.se
}
# case5: a backslash just before a newline in a String. bin/lexer takes the newline
# into the String and stops counting that line; the ConMa lexer ends the String there
case5() {
  lex backslash.se
}
# case6: messages for unexpected characters show them as Python's repr does:
# ' in double quotes, a full-width space, a no-break space and control characters escaped
case6() {
  lex chars.se
}
for c in case1 case2 case3 case4 case5 case6
do
  $c > /tmp/$c.log 2>&1
  diff $c.log /tmp/$c.log
done
rm -f /tmp/selfhost3.*
