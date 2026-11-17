# if defined i; then n=1; else n=2; fi を 40 万回
for i in $(seq 1 400000); do if defined i; then n=1; else n=2; fi; done
echo $n
