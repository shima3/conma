# レビュー 8：kill で止めたジョブのパイプの fd は閉じ、SIGTERM を送った外部コマンドはゾンビにならない
function f () { stdin > /dev/null; }
function fds () { lsof -p $$ 2> /dev/null | grep -c PIPE; }
before=$(fds)
for i in 1 2 3
do
  /bin/sleep 31 | f &
  j=$!
  sleep 0.5
  kill $j
  wait $j
done
sleep 1
after=$(fds)
if test "$before" = "$after"; then echo "pipes: no leak"; else echo "pipes: $before -> $after"; fi
zombies=$(ps -axo ppid=,stat= | awk -v p=$$ '$1 == p && $2 ~ /Z/' | wc -l | tr -d ' ')
echo "zombies: $zombies"
