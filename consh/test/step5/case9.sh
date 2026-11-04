# 後から加わった鎖も読まれる：& のジョブが書き続けていても、関数の後の echo の出力が読み手に届く
function g () { yes & echo hi; }
g | grep -m1 hi
echo "status=$?"
