# ${-語} は一致した名前の LazyList。一致しなければ空。記号がなければ語のまま
function show (l) { echo "[${l}]"; }
show ${-g/[ab].txt}
show ${-g/*.none}
show ${-g/c.se}
OFS=, show ${-g/*.txt}
echo "${-g/[ab].txt}"
# for 文で回す。一致しなければ1回も回らない
for f in ${-g/*.se}; do echo "for: $f"; done
for f in ${-g/*.none}; do echo "not reached"; done
# 変数を含むパターン。引用符の部分は記号にならない
dir=g
echo ${-$dir/'x['*}
# 外部コマンドには葉ごとの語にし、その葉には glob をもう一度は適用しない
ls ${-g/x?1?.txt}
