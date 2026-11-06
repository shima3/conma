# test/ の test.sh で共通に使う変数と関数。各 test.sh の最初に source する:
#   . "$(dirname "$0")/../common.sh"
# テストのディレクトリに移り、一時ディレクトリ $tmp を作る（テストが終わると消す）。
# モジュールとログは $tmp に作るので、別のステップのテストを同時に実行しても壊しあわない
test="$(cd "${BASH_SOURCE[0]%/*}"; pwd)"
root="${test%/*}"
cd "$(dirname "$0")" || exit 1
tmp="$(mktemp -d /tmp/conma-test.XXXXXX)" || exit 1
trap 'rm -rf "$tmp"' EXIT

# resolve file.se...: 各ファイルから $tmp/file.module.se を作る。失敗したらテストを止める
resolve() {
  local f
  for f in "$@"
  do
    "$test/resolver.sh" "$f" "$tmp" || exit 1
  done
}
