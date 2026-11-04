# & の直接つないだパイプライン。kill で最後の要素を止めると、前の要素は SIGPIPE で止まる
yes | cat | sleep 30 &
j=$!
kill $j
wait $j
echo "killed=$?"
seq 3 | wc -l | tr -d ' ' &
wait $!
echo "status=$?"
