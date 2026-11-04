# 片側だけが外部コマンドのパイプは OS のパイプでつなぐ。要素の切れ目と文字列化は、チャネルのときと同じ
function f () { cat; }
function first () { stdin > (x . rest); echo got $x; }
function nums () { for i in 1 2 3; do echo n$i; done; }
function lists () { l=(a b c); stdin < (l); stdin > (y); echo y=$y; }
function forever () { for i in $(yes); do echo $i; done; }
# 外部コマンド → 関数（関数の中の外部コマンドには Stream をそのまま渡す）
printf 'a\nb\r\nc\n' | f
# 外部コマンド → 関数の stdin（改行を除いた行が要素になる。読み手が先に終われば、書き手は SIGPIPE で止まる）
yes | first
function three () { stdin > (a b c); echo $a-$b-$c; }
seq 3 | three
# 関数 → 外部コマンド
nums | cat
nums | sort -r
# LazyList の要素は、パイプラインの文脈の OFS で文字列化する
echo x | lists | cat
function withofs () { OFS=:; echo x | lists | cat; }
withofs
# 読み手が先に終われば、関数の書き手を止める（中の $( ) も止まる）
forever | head -2
echo status $?
# 3段：外部 → 関数 → 外部、関数 → 外部 → 関数
seq 5 | f | tail -1
nums | cat | first
# パイプの書き手の if 文と for 文
if true; then echo if-out; fi | cat
for w in p q; do echo $w; done | tr a-z A-Z
# & のジョブ
function nc () { nums | cat; }
nc > (j1 j2 . jr) &
wait
echo $j1 $j2 ${jr}
# 外部コマンドの終了ステータス
true | false
echo $?
nums | false
echo $?
