function f (a b . rest) { echo $a; return 0 }
function g() { echo 1
  echo 2
}
function h (.args) {
  function inner () { echo $x }
  inner
}
