#!/bin/bash
# Compare the ConMa parser (include/parser.se) with bin/parser, after the lexer and the comment remover.
# Each file is parsed with a file name (bin/parser --file) and without one.
# For each, the exit codes, the result of selfhost_compare.py and the difference of the error messages
# are printed, and the error message itself.
cd ${0%/*}
if ! ../resolver.sh test.se 2> /tmp/selfhost5.err
then cat /tmp/selfhost5.err; exit 1
fi
parse() {
  echo "== $1"
  for mode in file nofile
  do
    if [ $mode = file ]; then opt="--file $1"; else opt=""; fi
    ../../bin/lexer $1 2> /dev/null | ../../bin/comment_remover 2> /dev/null | ../../bin/parser $opt > /tmp/selfhost5.py.txt 2> /tmp/selfhost5.py.err
    py=$?
    PYTHONUNBUFFERED=1 python3 ../vproc_run.py --main /tmp/test.module.se $mode $1 > /tmp/selfhost5.conma.txt 2> /tmp/selfhost5.conma.err
    echo "$mode: exit python $py, conma $?"
    if [ $py = 0 ]
    then
      python3 ../selfhost_compare.py ast /tmp/selfhost5.py.txt /tmp/selfhost5.conma.txt
      echo "compare: $?"
    fi
    diff /tmp/selfhost5.py.err /tmp/selfhost5.conma.err
    cat /tmp/selfhost5.py.err
  done
}
# case1: sources with comments, includes and nested functions
case1() {
  parse ../selfhost2/sample.se
  parse ../selfhost2/inc.se
  parse ../../include/util.se
}
# case2: Body: explicit SInfo with and without Strings, an empty Body, only an LCont,
# a FuncExp as Operator and as the last operand, nested functions, an Includer after a Definition;
# variadic Heads in a Definition, a FuncExp and an LCont
case2() {
  parse body.se
  parse variadic.se
}
# case3: no statements: an empty file and a file with only a comment
case3() {
  parse empty.se
  parse comment_only.se
}
# case4: syntax errors at the top level
case4() {
  parse err_toplevel.se
  parse err_keyword.se
  parse err_after_paren.se
  parse err_empty_paren.se
  parse err_include.se
}
# case5: syntax errors in a Definition; a value with ' and " in the message
case5() {
  parse err_comma.se
  parse err_param.se
  parse err_operator.se
  parse err_end.se
}
# case6: an onError that returns a value: the value is the result of __Parser_main__,
# also for an error deep in the recursion
case6() {
  echo "== recover"
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py --main /tmp/test.module.se recover err_end.se err_toplevel.se empty.se 2>&1
  echo "exit: $?"
}
for c in case1 case2 case3 case4 case5 case6
do
  $c > /tmp/$c.log 2>&1
  diff $c.log /tmp/$c.log
done
rm -f /tmp/selfhost5.*
