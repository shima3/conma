# (( )) の SJS の関数 fib で、素朴な再帰のフィボナッチ数 fib(35)（CPython で約 1 秒）
# bench/fib/fib.py、consh/sjs/bench/fib.safe.js と同じ計算。自分を名前で呼ぶ数だけの関数なので、wasm 版の VM では WebAssembly で動く
(( fib = (n) => {
  if (n === 0) return 0
  if (n === 1) return 1
  return fib(n - 1) + fib(n - 2)
} ))
echo $(( fib(35) ))
