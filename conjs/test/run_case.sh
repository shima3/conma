#!/bin/bash
# ConJS のテストの 1 つのステップ（docs/dev/PLATFORM-PLAN-ja.md の 4.2 節）。2026-11-20 から、言語キットのテストの型
# bin/lang-test（4.3 節）を呼ぶだけにした。ステップのディレクトリの case*.con.js を変換して動かし、出力と終了ステータスを
# case*.log と比べる。VM は wasm 版（conjs/lang.conf の vm。段階 P10）
#   run_case.sh ステップのディレクトリ
# ステップに copy.txt があれば、一時ディレクトリに写してから動かす（ファイルを書くステップ。同じワークツリーで、いくつかの
# テストを同時に流しても混ざらない）。server.py もあれば、写した所で HTTP のサーバとして立て（ポートは port.txt）、
# 終わったら止める（4.4 節、段階 P5。step9 の fetch）
here="$(cd "$(dirname "$0")"; pwd)"
step="$1"
if [ ! -f "$here/$step/copy.txt" ]; then
  exec "$here/../../bin/lang-test" "$here/.." "$here/$step"
fi
tmp="$(mktemp -d "${TMPDIR:-/tmp}/conjs-test.XXXXXX")" || exit 1
work="$tmp/$step"
mkdir "$work" && cp "$here/$step"/case* "$work/" || { rm -rf "$tmp"; exit 1; }
if [ -f "$here/$step/server.py" ]; then
  # サーバは、標準入力（fd 3 の先のパイプ）が閉じたら終わる
  exec 3> >(exec python3 "$here/$step/server.py" "$work/port.txt")
  for i in $(seq 1 100); do [ -s "$work/port.txt" ] && break; sleep 0.1; done
  [ -s "$work/port.txt" ] || { echo "run_case.sh: $step: the server did not start" >&2; exec 3>&-; rm -rf "$tmp"; exit 1; }
fi
"$here/../../bin/lang-test" "$here/.." "$work" | sed "s|$tmp/||g"
exec 3>&- 2>/dev/null
rm -rf "$tmp"
