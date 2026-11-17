# 関数どうしのパイプ gen | inc | cnt で 7 万 5000 要素を渡す（要素を 1 つずつ受け渡す）
function gen () { for i in $(seq 1 75000); do echo $i; done; }
function inc () { for x in $(stdin); do echo x$x; done; }
function cnt () { n=0; for x in $(stdin); do n=$x; done; echo $n; }
gen | inc | cnt
