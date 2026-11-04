# 関数と組み込みコマンドの引数、for 文の語にも適用する。一致した名前ごとに1つの引数
function count (. rest) { for x in ${rest}; do echo "arg: $x"; done; }
count g/[ab].txt
for f in g/*.se; do echo "for: $f"; done
# 変数の値の * も、引用符で囲まなければ適用する（Bash と同じ）。変数定義の右辺には適用しない
p=g/*.se
echo "$p" $p
# 値 (語 ...) にも適用する
a=(g/[ab].txt)
echo ${a}
