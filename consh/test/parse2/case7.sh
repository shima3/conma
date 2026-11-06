# 関数値、reset、shift
f=function (a b . rest) { echo "$a"; }
g=function(){ echo x; } h=1
k=function ( . r ) {
  echo $r
}
reset { echo start; shift k (v) { $k 1; }; echo "v=$v"; } > (out)
reset{ x; } | cat
f; reset { shift k { echo $k; }; }
shift k { echo $k; }
x=${y-function (a) { echo d; }}
# キーワードにならない
reset
reset x
shift x
shift
x=function
echo function reset shift
