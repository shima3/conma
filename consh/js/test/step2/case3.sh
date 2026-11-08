# 2つ以上の js が同時に終わっていないとき、どちらの console.log か決められない出力は、ホストの標準出力に直接書く。
# 1つだけなら、それの出力にする
js 'new Promise(r => setTimeout(() => { console.log("A (two calls)"); r("a") }, 50))' > (ra) &
js 'new Promise(r => setTimeout(() => { console.log("B (one call)"); r("b") }, 150))' &
wait
echo "ra=$ra"
