# レビュー 7：[...] の中の、クォートした文字とエスケープした文字は、その文字そのもの（Bash と同じ）。*/. と */.. も一致する
d=$1
mkdir $d/b7
cd $d/b7
touch a b c ./- ']'
mkdir sub
echo [a\-c]
echo [a"-"c]
echo ["!"a]
echo [!a]
echo */. */..
