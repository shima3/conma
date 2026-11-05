# 文の終わりに待つ：出力のない $( ) は終わるまで待ち、出力の多いものや終わらないものは、16 行を読んだら待たない
d=$(mktemp -d /tmp/consh.step16.XXXXXX)
x=$(mkdir $d/sub)
ls -d $d/sub > /dev/null
echo "mkdir done $?"
big=$(seq 1 100000)
n=0
for i in ${big}; do n=$i; done
echo "last $n"
inf=$(yes)
for i in ${inf}; do echo "first $i"; break; done
rm -rf $d
echo "end"
