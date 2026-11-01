d=$1
echo to-file > $d/out
cat < $d/out
echo more >> $d/out
cat $d/out
sh -c 'echo to-err >&2' 2> $d/err
cat $d/err
sh -c 'echo out; echo err >&2' > $d/both 2>&1
cat $d/both
sh -c 'echo out; echo err >&2' 2>&1 > $d/out-only
cat $d/out-only
echo "x" 1>&2 2> $d/err2
cat $d/err2
true > $d/empty
cat $d/empty
cat < no-such-file
echo "status $?"
