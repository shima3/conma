# console：console.log、info、debug は標準出力に、console.error、warn は標準エラー出力に、1回で1つの要素を出す
js '(console.log("log", 1), console.info("info"), console.debug("debug"), console.error("error"), console.warn("warn"), "result")'
# 文字列でない引数は JSON.stringify（できなければ String）にして、スペースでつなぐ
js '(console.log({a: [1, "x"]}, null, undefined, true, () => 1), 0)'
# 変数に入れるときは、標準出力には console.log だけが出る
js v= '(console.log("only the log"), "the value")'
echo "v=$v"
# 結果より前に出る。Promise を待つ間の出力も
js 'new Promise(r => setTimeout(() => { console.log("in a timer"); r("resolved") }, 20))'
# リダイレクトと変数リストにも行く
js 'console.error("to e1"), console.log("to o1")' 2> (e1) > (o1)
echo "e1=$e1 o1=$o1"
# 例外の前の出力も出る
js '(console.log("then throw"), nosuch)'
echo "status $?"
