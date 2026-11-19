# 関数 loop で、n を 0 まで減らし、acc を数える（40 万回。ConSh 版は 5 億回だが、Bash では約 22 分かかるので小さくした）
# smaller size: 500000000/400000
# （上の行は run.py が読む：ConSh 版の仕事はこの何倍か。Bash の時間にこれを掛けたものを、ほかの版と比べる）
set -u
loop() {
  local n=$1 acc=0
  while (( n != 0 )); do
    (( n -= 1, acc += 1 ))
  done
  r=$acc
}
loop 400000
echo $r
