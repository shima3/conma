d=$1
function out () {
  echo one
  sh -c 'echo two; echo three'
  printf 'no newline'
  printf 'crlf\r\n'
}
out
out > $d/file
cat $d/file
function errs () { sh -c 'echo to-err >&2'; echo to-out }
errs 2>&1
errs 2> $d/err
echo "err file:"
cat $d/err
sh -c 'echo x; echo y >&2' 2>&1
