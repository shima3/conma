# パイプラインも & で起動できる
echo piped | cat &
wait
yes | head -1 &
wait $!
echo "status $?"
