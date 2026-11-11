# エラーは $? を 2 にする
(( 1 + "a" ))
echo "status $?"
(( nosuch ))
echo "status $?"
(( null.x ))
echo "status $?"
(( this ))
echo "status $?"
(( a == 1 ))
echo "status $?"
(( ({})["construct" + "or"] ))
echo "status $?"
(( (() => { throw "thrown" })() ))
echo "status $?"
(( $undefinedvar ))
echo "status $?"
s=abc
(( $s + 1 ))
echo "status $?"
(( 2 ** 0.3 ))
echo "status $?"
