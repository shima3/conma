FOO=$(echo v) f a | g b & h > (a . rest) 2>&1
function f (a) { echo $a; return 1 }
