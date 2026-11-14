# 数の関数の wasm 版（consh/SJS-WASM-PLAN-ja.md の段階 4）：(( )) の「名前 = 関数式」で、数だけを扱う関数は、wasm 版の VM では
# WebAssembly で動く。ほかの VM では ConMa 版で動き、結果は同じ。後の (( )) が名前や Math のメンバーを書き換えると、呼ぶときに
# 確かめて ConMa 版で動かす
(( fib = (n) => n < 2 ? n : fib(n - 1) + fib(n - 2) ))
echo "fib: $(( fib(20) ))"
(( count = (n) => { let acc = 0; while (n !== 0) { n -= 1; acc += 1 } return acc } ))
echo "count: $(( count(100000) ))"
(( hyp = function (a, b) { return Math.sqrt(a * a + b * b) } ))
echo "hyp: $(( hyp(3, 4) ))"
# 2 つの数の関数を 1 つの (( )) で作り、一方が他方を呼ぶ
(( even = (n) => n === 0 ? 1 : odd(n - 1), odd = (n) => n === 0 ? 0 : even(n - 1) ))
echo "even: $(( even(10) )) $(( even(7) ))"
# 数でない引数、足りない引数は ConMa 版で動く（=== は数でない値とも比べられる）
(( isZero = (x) => x === 0 ? 1 : 0 ))
echo "isZero: $(( isZero(0) )) $(( isZero(-0) )) $(( isZero("0") )) $(( isZero() ))"
# シェル変数の値は文字列なので、+$名前 で数にして渡す
n=12
echo "shell: $(( fib(+$n) ))"
# Math のメンバーを書き換えると、Math を使う数の関数は ConMa 版で動き、書き換えた関数を呼ぶ
(( Math.sqrt = (x) => -1 ))
echo "hyp after Math.sqrt: $(( hyp(3, 4) ))"
# 名前を書き換えると、それを呼ぶ数の関数は ConMa 版で動き、新しい関数を呼ぶ
(( oldFib = fib ))
(( fib = (n) => 100 ))
echo "fib replaced: $(( fib(20) )) $(( oldFib(20) ))"
