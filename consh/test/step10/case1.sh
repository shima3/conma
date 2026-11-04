# 関数の出力が端末のままなら、中の外部コマンドに端末（シェルの標準出力）をそのまま渡す
function f () { sh -c '[ -p /dev/fd/1 ] && echo pipe || echo not-pipe'; printf 'raw'; echo; }
f
# 入れ子の関数も同じ
function g () { f; }
g
# 組み込みコマンドの出力との順序は保たれる
function h () { echo 1; printf '2\n'; echo 3; sh -c 'echo 4'; echo 5; }
h
