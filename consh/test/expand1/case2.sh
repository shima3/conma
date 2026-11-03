empty=$(x=1) e= s=""
echo $empty a$empty "$empty"
echo $e a$e "$e"
echo $s a$s "$s"
echo "" '' ${undefined-} a${undefined-d}
