#!/bin/bash
# Tests of test/selfhost_compare.py with the outputs of the Python tools for sample.se
# and the same outputs in the ConMa format (conma/*.txt)
cd ${0%/*}
bin=../../bin
compare="python3 ../selfhost_compare.py"
tmp=/tmp/selfhost2
# Absolute paths in the includer and resolver outputs are written as <dir>
dir="$(pwd -P)"
$bin/lexer sample.se > $tmp.lexer.tsv
$bin/comment_remover < $tmp.lexer.tsv > $tmp.comment_remover.tsv
$bin/parser --file sample.se $tmp.comment_remover.tsv > $tmp.parser.se
$bin/includer sample.se | sed -e "s|$dir|<dir>|g" > $tmp.includer.se
$bin/includer sample.se | $bin/resolver | sed -e "s|$dir|<dir>|g" > $tmp.resolver.se

# run label command...: prints the label, the output and the exit code
run() {
  echo "== $1"
  shift
  "$@" 2>&1
  echo "exit: $?"
}

# changed file sed-script: a copy of file changed by sed-script
changed() {
  sed -e "$2" "$1" > $tmp.changed.txt
  echo $tmp.changed.txt
}

# case1: the token outputs are equal
case1() {
  run lexer $compare tokens $tmp.lexer.tsv conma/lexer.txt
  run comment_remover $compare tokens $tmp.comment_remover.tsv conma/comment_remover.txt
  run stdin sh -c "$compare tokens - conma/lexer.txt < $tmp.lexer.tsv"
}

# case2: the S-expression outputs are equal, whatever their layout
case2() {
  run parser $compare ast $tmp.parser.se conma/parser.txt
  run includer $compare ast $tmp.includer.se conma/includer.txt
  run resolver $compare ast $tmp.resolver.se conma/resolver.txt
}

# case3: token differences
case3() {
  run value $compare tokens $tmp.lexer.tsv $(changed conma/lexer.txt 's/SYMBOL "define"/SYMBOL "defun"/')
  run escape $compare tokens $tmp.lexer.tsv $(changed conma/lexer.txt 's/"\\"inc.se\\""/"inc.se"/')
  run column $compare tokens $tmp.lexer.tsv $(changed conma/lexer.txt 's/(6 9) SYMBOL/(6 10) SYMBOL/')
  run kind $compare tokens $tmp.lexer.tsv $(changed conma/lexer.txt 's/LPAREN/RPAREN/')
  run missing $compare tokens $tmp.lexer.tsv $(changed conma/lexer.txt '$d')
  run extra $compare tokens $tmp.lexer.tsv $(changed conma/lexer.txt '$p')
}

# case4: S-expression differences
case4() {
  run string $compare ast $tmp.parser.se $(changed conma/parser.txt 's/"a\\"b/"a\\"B/')
  run atom $compare ast $tmp.resolver.se $(changed conma/resolver.txt 's/"h" (G 2)/"h" (L 2)/')
  run missing-child $compare ast $tmp.parser.se $(changed conma/parser.txt 's/ (LCont (9 6))//')
  run extra-child $compare ast $tmp.parser.se $(changed conma/parser.txt 's/(OList (9 5)/(OList (9 5) (Null (9 5))/')
  run missing-item $compare ast $tmp.includer.se $(changed conma/includer.txt '$d')
  run extra-item $compare ast $tmp.includer.se $(changed conma/includer.txt '$p')
}

# case5: outputs that cannot be read, and a wrong command line
case5() {
  run not-a-token $compare tokens $tmp.lexer.tsv $(changed conma/lexer.txt '1s/Token/Tok/')
  run bad-tsv $compare tokens $(changed $tmp.lexer.tsv '1s/^0\t0\t//') conma/lexer.txt
  run unbalanced $compare ast $tmp.parser.se $(changed conma/parser.txt 's/)$//')
  run extra-paren $compare ast $tmp.parser.se $(changed conma/parser.txt 's/$/)/')
  run no-file $compare ast $tmp.parser.se none.txt
  run bad-mode sh -c "$compare lisp $tmp.parser.se conma/parser.txt 2>&1 | head -1; exit \${PIPESTATUS[0]}"
}

for c in case1 case2 case3 case4 case5
do
  $c > /tmp/$c.log
  diff $c.log /tmp/$c.log
done
rm -f $tmp.*
