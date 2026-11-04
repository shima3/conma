# 関数や組み込みコマンドとのパイプは、今までどおりチャネルでつなぐ
function up () { for line in $(stdin); do echo "<$line>"; done; }
seq 3 | up | tr '<>' '[]'
echo a b | cat | up
# 先頭の語が変数なら、外部コマンドでも直接はつながない（結果は同じ）
c=cat
seq 2 | $c | wc -l | tr -d ' '
