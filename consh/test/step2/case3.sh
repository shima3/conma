echo "env: $CONSH_T"
CONSH_T=local
echo "local: $CONSH_T"
sh -c 'echo "child: $CONSH_T"'
CONSH_T=prefix sh -c 'echo "prefix: $CONSH_T"'
echo "after prefix: $CONSH_T"
export CONSH_T
sh -c 'echo "exported: $CONSH_T"'
CONSH_T=changed
sh -c 'echo "env stays: $CONSH_T"'
export CONSH_U=only-env
echo "only env: $CONSH_U"
sh -c 'echo "child: $CONSH_U"'
