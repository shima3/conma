# if defined i; then n=1; else n=2; fi を 5000 回
for i in $(seq 1 5000); do if defined i; then n=1; else n=2; fi; done
echo $n
