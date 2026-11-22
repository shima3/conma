#!/bin/bash
# GitHub Actions の CI（.github/workflows/ci.yml）で suite を 1 つ流し、既知の違いのほかに違いがあれば失敗にする。
# 使い方：.github/ci-suite.sh "既知の違いのテストの名前（空白で区切る。なければ空）" コマンド 引数...
# suite は、テストごとに「== 名前」の行を出し、違いがあれば、その下に diff を出す（docs/dev/TESTS-ja.md の「流し方」）。
# 既知の違いは docs/dev/TESTS-ja.md の「既知の違い」のもの。出なくても失敗にはしない（混んでいるときだけ出るものがあるため）。
known=" $1 "
shift
out="$(mktemp)" || exit 1
trap 'rm -f "$out"' EXIT
# GitHub Actions の runner は SIGPIPE を無視させたままコマンドを始め、bash はそれを元に戻せない。そのままだと、
# `yes | head -1` の yes が「Broken pipe」を出して終わるなど、端末で流すときと違うので、Python で元に戻してから流す
python3 -c 'import os, signal, sys; signal.signal(signal.SIGPIPE, signal.SIG_DFL); os.execvp(sys.argv[1], sys.argv[1:])' "$@" 2>&1 | tee "$out"
status=${PIPESTATUS[0]}
# 違いの行があるテストの名前（最初の「==」の前に出た行は「(前)」）
names="$(awk '/^== /{n=substr($0,4); next} {print (n=="" ? "(前)" : n)}' "$out" | sort -u)"
bad=""
for n in $names; do
  case "$known" in
    *" $n "*) echo "::notice::既知の違い：$n" ;;
    *) bad="$bad $n" ;;
  esac
done
if [ -n "$bad" ]; then
  echo "::error::既知でない違い：$bad"
  exit 1
fi
if [ "$status" -ne 0 ]; then
  echo "::error::終了ステータス $status"
  exit 1
fi
