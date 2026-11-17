# 先頭の語が決まっている単純文（最適化版は、変換のときに組み込みコマンドか外部コマンドかを決める。Cmd_plain_code）：
# 同じ文を、関数を定義する前と後で実行すると、後では関数を呼ぶ（組み込みコマンドの名前でも、外部コマンドの名前でも）
function run_twice () { for k in 1 2; do echo "e$k"; printf '%s\n' "p$k"; if [ $k = 1 ]; then
  function echo (. a) { builtin echo "my echo: $a"; }
  function printf (f . a) { command printf "my printf: $f" $a; builtin echo; }
fi; done; }
run_twice
# 関数の中で定義した関数は、その呼び出しの中だけ。外では組み込みコマンドの echo に戻る
echo after
function inner () { function cat () { builtin echo "my cat"; }; cat; }
inner
echo x | cat
# builtin と command は今までどおり
builtin echo b; command echo c
# 関数の引数の数の誤りと、ない外部コマンド
function two (a b) { echo "$a $b"; }
two 1 2
two 1
