# stdin：標準入力の要素を、文字列にして順に渡す非同期のイテレータと、全部の配列の Promise（lines()）
function gen () { echo one; echo two; echo "three four" }
gen | js 'async () => { let n = 0; for await (const l of stdin) n++; return n }'
gen > (a b c)
js all= 'stdin.lines()' < (a b c)
for e in $all; do echo "all: $e"; done
# LazyList の要素は1つの文字列にする（OFS）
inner=(x y)
list=(p ${inner})
js 'stdin.lines()' < (list)
js 'stdin.lines()' < /dev/null
echo "empty: $?"
# 途中までしか読まなければ、残りは次のコマンドが読む（読み出し位置は組み込みコマンド stdin と同じ）
function firstTwo () {
  js 'async () => { const got = []; for await (const l of stdin) { got.push(l); if (got.length === 2) break } return got.join("+") }'
  stdin > (rest)
  echo "rest: $rest"
}
gen | firstTwo
# stdin を読まないコードは、標準入力を読まない
function noRead () { js '"no read"'; stdin }
gen | noRead
# 外部コマンドの出力（ここでは、ファイル）も行ごとの要素になる
echo "line 1" > in.txt
echo "line 2" >> in.txt
js '(async () => (await stdin.lines()).map(s => s.toUpperCase()))()' < in.txt
