function show_env () { echo "local view: $V"; sh -c 'echo "child: $V"' }
V=prefix show_env
echo "after prefix: [${V-}]"
function exp () { export W=inside }
exp
sh -c 'echo "W=$W"'
function change () { echo "in: $L"; export L=changed; echo "after export: $L" }
L=layer change
echo "outside: $L"
x=5
function px () { echo "x=$x" }
x=1 px
