# test/ の test.sh で共通に使う変数と関数。各 test.sh の最初に source する:
#   . "$(dirname "$0")/../common.sh"
# テストのディレクトリに移り、一時ディレクトリ $tmp を作る（テストが終わると消す）。
# モジュールとログは $tmp に作るので、別のステップのテストを同時に実行しても壊しあわない
test="$(cd "${BASH_SOURCE[0]%/*}"; pwd)"
root="${test%/*}"
cd "$(dirname "$0")" || exit 1
tmp="$(mktemp -d /tmp/conma-test.XXXXXX)" || exit 1
trap 'rm -rf "$tmp"' EXIT

# resolve file.se...: 各ファイルから $tmp/file.module.se を作る。失敗したらテストを止める。
# resolver の警告（標準エラー出力）は、file.resolve.log があればそれと比べ、なければそのまま出す
resolve() {
  local f base
  for f in "$@"
  do
    base="$(basename "${f%.se}")"
    "$test/resolver.sh" "$f" "$tmp" 2> "$tmp/$base.resolve.log" || { cat "$tmp/$base.resolve.log" >&2; exit 1; }
    if [ -e "$base.resolve.log" ]
    then diff "$base.resolve.log" "$tmp/$base.resolve.log"
    else cat "$tmp/$base.resolve.log"
    fi
  done
}
