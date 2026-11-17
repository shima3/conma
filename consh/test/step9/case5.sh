# 最適化版は Arg_max の結果を環境変数ごとに覚え、export ARG_MAX で使わなくなる（Env_export の版）。
# 同じ文を、export の前と後で実行すると、後では新しい ARG_MAX を使う。
# printf(7) + '%s\n'(5) + tryN-k(7) = 19 バイト
function try (n) { for k in 1 2; do printf '%s\n' "try$n-$k"; done; }
try 1
export ARG_MAX=19
try 2
# 関数の中の export も、外の文に効く（同じ環境変数）
function setmax (m) { export ARG_MAX=$m; }
setmax 100
try 3
# $( ) の中の export は外に効かない（環境変数の写し）
v=$(export ARG_MAX=18; echo inner)
echo "$v"
try 4
# 前置の変数定義は、その文だけ
ARG_MAX=18 printf '%s\n' "$(echo too-long-for-18)"
