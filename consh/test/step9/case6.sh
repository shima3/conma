# 覚えた結果を使った後でも、export ARG_MAX で小さくすると、次の外部コマンドで超える
function try (n) { for k in 1 2; do printf '%s\n' "try$n-$k"; done; }
try 1
export ARG_MAX=19
try 2
export ARG_MAX=18
try 3
echo not reached
