# (( )) の SJS の関数 loop で、n を 0 まで減らし、acc を数える（3000 万回。CPython で約 1 秒）
# bench/loop/loop.py、consh/sjs/bench/loop.safe.js と同じ計算。数だけを扱うので、wasm 版の VM では WebAssembly で動く
(( loop = (n) => {
  let acc = 0
  while (n !== 0) {
    n -= 1
    acc += 1
  }
  return acc
} ))
echo $(( loop(30000000) ))
