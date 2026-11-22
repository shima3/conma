# レビュー 8：kill で止めたジョブのパイプの fd は閉じ、SIGTERM を送った外部コマンドはゾンビにならない
# パイプの数は、lsof がファイルに書いた一覧から数える。lsof | grep で数えると、ConSh がそのパイプラインを起動している
# 途中のパイプの端（どこまで準備したかは、機械の速さで変わる）も数に入る（GitHub Actions の macos-latest で、
# 違いのない 4 本に対して 5、11、12 などになった。2026-11-22）。SIGCHLD で起こすためのパイプは最初の外部コマンドを
# 待つときにできるので、前もって 1 つ動かしておく
d=$1
function f () { stdin > /dev/null; }
/usr/bin/true
lsof -p $$ > $d/case6.before 2> /dev/null
before=$(grep -c PIPE $d/case6.before)
for i in 1 2 3
do
  /bin/sleep 31 | f &
  j=$!
  sleep 0.5
  kill $j
  wait $j
done
sleep 1
lsof -p $$ > $d/case6.after 2> /dev/null
after=$(grep -c PIPE $d/case6.after)
if test "$before" = "$after"; then echo "pipes: no leak"; else echo "pipes: $before -> $after"; fi
zombies=$(ps -axo ppid=,stat= | awk -v p=$$ '$1 == p && $2 ~ /Z/' | wc -l | tr -d ' ')
echo "zombies: $zombies"
