function f (a b . rest) {
  echo "a=$a b=$b"
  echo "rest:" $rest
  echo "rest quoted: [$rest]"
  return 3
  echo not reached
}
f 1 2 3 "4 5"
echo "status $?"
f only
function g () { echo "no params" }
g extra args are dropped
function h (. all) { echo all: $all }
h x y
function last () { false }
last
echo "last: $?"
function empty () { }
false
empty
echo "empty: $?"
function assign_last () { false; x=1 }
assign_last
echo "assign last: $?"
