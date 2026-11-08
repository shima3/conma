# Promise は解決するまで待つ。待つ間、ほかの VProc は動く
js 'Promise.resolve("resolved")'
js w= 'new Promise(r => setTimeout(() => r("waited"), 50))'
echo "w=$w"
function ticks () { for i in 1 2 3; do echo "tick $i"; done }
js 'new Promise(r => setTimeout(() => r("late"), 300))' &
ticks
wait
js 'Promise.reject(new RangeError("no"))'
echo "status $?"
