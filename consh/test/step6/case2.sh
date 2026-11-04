# 変数の数だけ受け取った後に書くと、コマンドを止める。$? は 0
yes > (y1 y2)
echo "y1=$y1 y2=$y2 status=$?"
function inf () { for x in $(yes); do echo "inf $x"; done; }
inf > (i)
echo "i=$i status=$?"
