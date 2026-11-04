# レビュー 4：先頭の語が glob で関数の名前になる要素は、OS のパイプでつながない
d=$1
function foo () { /bin/echo one; /bin/echo two; echo three; }
touch $d/foo
cd $d
f?o | cat
