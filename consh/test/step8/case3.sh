# 起動できない外部コマンド：読み手がなければ書き手は SIGPIPE で止まり、書き手がなければ読み手は EOF を読む
nosuch | wc -l | tr -d ' '
echo "status=$?"
yes | nosuch2
echo "status=$?"
