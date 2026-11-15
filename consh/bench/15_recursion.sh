# リストの先頭を除いて自分を呼ぶ関数 down（深さ 1000）を 1000 回。戻ると、先頭と下の結果をつないで出す
function down (x . rest) {
  for y in ${rest}; do down ${rest} > (r); echo $x$r; return 0; done
  echo $x
}
xs=$(seq 1 1000)
for i in $(seq 1 1000); do down ${xs} > (r); done
echo $r
