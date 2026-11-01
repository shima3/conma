x=1
function show () { echo "x=$x" }
x=2
show
echo "top x=$x"
function local () { y=inside; echo "in: $y" }
y=outside
local
echo "out: $y"
function caller () { later; function nested () { echo nested }; nested }
function later () { echo "defined later" }
caller
nested
function f () { echo first }
f
function f () { echo second }
f
function a (n) { echo "a $n"; b$n }
function b1 () { a 2 }
function b2 () { echo "end" }
a 1
