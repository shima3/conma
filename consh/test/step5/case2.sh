# 読み手が先に終わると、書き手は次に書いたときに止まる
yes | head -2
echo "status $?"
function inf () { for x in $(yes); do echo "inf $x"; done; }
inf | head -2
# $( ) は遅延なので、終わらないコマンドでも、使う分だけ読む
for x in $(yes); do echo "for $x"; break; done
x=$(yes); echo assigned
