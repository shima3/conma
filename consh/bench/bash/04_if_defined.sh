# if defined i; then n=1; else n=2; fi を 5000 回（defined i は [ -n "${i+x}" ]）
set -u
for i in $(seq 1 5000); do if [ -n "${i+x}" ]; then n=1; else n=2; fi; done
echo $n
