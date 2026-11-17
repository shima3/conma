# リストの先頭を除いて自分を呼ぶ関数 down（深さ 500）を 5 回。戻ると、先頭と下の結果をつないで出す
# 残りは $rest で 1 つずつの引数として渡す（${rest} はリスト全体を 1 つの引数にするので、再帰しない）
function down (x . rest) {
  for y in ${rest}; do down $rest > (r); echo $x $r; return 0; done
  echo $x
}
xs=$(seq 1 500)
for i in $(seq 1 5); do down $xs > (r); done
echo $r
