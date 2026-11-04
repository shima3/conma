d=$1
function out () {
  echo one
  sh -c 'echo two; echo three'
  printf 'no newline'
  printf 'crlf\r\n'
}
# 関数の出力が端末やファイルのままなら、中の外部コマンドはバイト列をそのまま書く（段階 10）
out
out > $d/file
cat $d/file
# 外部コマンドに渡すパイプは OS のパイプなので、中の外部コマンドはバイト列をそのまま書く（段階 15。以前は行ごとの要素にしていた）
out | cat
function errs () { sh -c 'echo to-err >&2'; echo to-out }
errs 2>&1
errs 2> $d/err
echo "err file:"
cat $d/err
sh -c 'echo x; echo y >&2' 2>&1
