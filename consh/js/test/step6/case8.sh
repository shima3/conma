# 先頭の語の SJS の値が関数でなければエラー（$? は 2。スクリプトは終わる）
v=$(( 3 ))
$v a
echo "not reached"
