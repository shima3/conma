# エラーは $? を 2 にする。SJS の範囲の外のコードもエラーにする（consh/SAFEJS-PLAN-ja.md の 3.2）
(( undefinedName + 1 ))
echo "status $?"
(( 1 + ))
echo "status $?"
(( this ))
echo "status $?"
(( new Array(2) ))
echo "status $?"
(( globalThis ))
echo "status $?"
(( process ))
echo "status $?"
(( (() => 1).constructor ))
echo "status $?"
(( []["constructor"] ))
echo "status $?"
(( k = "constr" + "uctor" ))
(( ({})[k] ))
echo "status $?"
(( a = {}; a.__proto__ ))
echo "status $?"
(( /x/.test("x") ))
echo "status $?"
(( 1 == 1 ))
echo "status $?"
(( Math = 1 ))
echo "status $?"
(( eval ))
echo "status $?"
# 段階 A では async と await も範囲の外
(( af = async () => 1 ))
echo "status $?"
