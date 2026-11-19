# 関数 fib で、素朴な再帰のフィボナッチ数 fib(23)（ConSh 版は fib(39) だが、Bash では約 32 分かかるので小さくした）
# smaller size: 204668309/92735
# （上の行は run.py が読む：ConSh 版の仕事はこの何倍か。fib(n) の呼び出しは 2 fib(n+1) − 1 回で、fib(39) は
# 204668309 回、fib(23) は 92735 回。Bash の時間にこれを掛けたものを、ほかの版と比べる）
# 結果は、r=$(fib …) にすると fork の時間を測ることになるので、大域変数 r で返す
set -u
fib() {
  local n=$1 a
  if (( n == 0 )); then r=0; return; fi
  if (( n == 1 )); then r=1; return; fi
  fib $(( n - 1 )); a=$r
  fib $(( n - 2 )); r=$(( a + r ))
}
fib 23
echo $r
