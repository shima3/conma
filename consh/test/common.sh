# ConSh のテストで共通に使うパスと関数。各 test.sh から source する
consh="$(cd ${BASH_SOURCE[0]%/*}/..; pwd)"
root="${consh%/*}"

# caseN.sh を consh_parse に渡し、出力と終了ステータスを caseN.log と比べる。
# 引数はオプションとして consh_parse に渡す。caseN.args があれば、その各行をスクリプトの引数にする
parse_cases() {
  local f c args
  for f in case*.sh
  do
    [ -e "$f" ] || continue
    c=${f%.sh}
    args=()
    if [ -e $c.args ]; then
      while IFS= read -r line; do args+=("$line"); done < $c.args
    fi
    python3 "$consh/bin/consh_parse" "$@" "$f" ${args[@]+"${args[@]}"} > /tmp/consh.$c.log 2>&1
    echo "exit: $?" >> /tmp/consh.$c.log
    diff $c.log /tmp/consh.$c.log
  done
}

# ConSh の ConMa 版を実行し、出力と終了ステータスを caseN.log と比べる
run() {
  local c=$1; shift
  PYTHONUNBUFFERED=1 "$consh/bin/consh" "$@" > /tmp/consh.$c.log 2>&1
  echo "exit: $?" >> /tmp/consh.$c.log
  diff $c.log /tmp/consh.$c.log
}
