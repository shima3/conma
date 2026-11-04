# & で起動したパイプラインを wait で待つ。$! は起動した VProc の番号。起動する前の $! は空文字列
echo "before=[$!]"
sleep 1 &
if [ -n "$!" ]; then echo bang-set; fi
wait $!
echo "wait status $?"
sh -c 'exit 3' &
id=$!
wait $id
echo "status $?"
# すでに待ったジョブは 127
wait $id
echo "again $?"
# 引数がなければ、すべてのジョブを待ち、$? は 0
sh -c 'exit 4' &
sh -c 'exit 5' &
wait
echo "all $?"
