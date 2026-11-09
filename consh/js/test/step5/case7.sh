# CONSH_JS_DEBUG：JS の文をシェルの文として実行するとき、JavaScript としてのエラーも出す
console.lg("x" a)
CONSH_JS_DEBUG=1
console.lg("x" a)
nosuchcommand a b
CONSH_JS_DEBUG=
nosuchcommand a b
