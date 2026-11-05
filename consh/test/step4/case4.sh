# $( ) の中から外のループへの continue はエラー。
# 値は読まない（読む文と、$( ) の中のエラーのどちらが先かは決まらないので）
for x in a b; do
  for y in a; do z=$(echo 1; continue x; echo 2); echo "never $z"; done
done
