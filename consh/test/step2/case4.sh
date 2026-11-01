nosuchcommand a b
echo "status $?"
sh -c 'kill -TERM $$'
echo "status $?"
sh -c 'kill -TERM $$'
