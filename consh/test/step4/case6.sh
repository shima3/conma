# 文の終わりに、$( ) が終わるか、チャネルが一杯で書き手が待つまで待つ。
# 出力の少ない $( ) の副作用は、次の文より前に終わる
rm -rf /tmp/consh.step4.dd
x=$(mkdir /tmp/consh.step4.dd); y=$(mkdir /tmp/consh.step4.dd/d2)
ls /tmp/consh.step4.dd
rm -rf /tmp/consh.step4.dd
# 出力の多い $( ) は、一杯になったところで次の文に進むので、終わらないコマンドでも進む
x=$(yes); echo assigned
