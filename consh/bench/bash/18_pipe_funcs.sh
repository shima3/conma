# 関数どうしのパイプ gen | inc | cnt で 7 万 5000 要素を渡す（$(stdin) は $(cat)）
set -u
gen() { local i; for i in $(seq 1 75000); do echo $i; done; }
inc() { local x; for x in $(cat); do echo x$x; done; }
cnt() { local n=0 x; for x in $(cat); do n=$x; done; echo $n; }
gen | inc | cnt
