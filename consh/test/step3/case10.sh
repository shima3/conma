# if、elif、else と $?
if true; then echo a; else echo b; fi
if false; then echo a; else echo b; fi
if false; then echo a; elif true; then echo c; fi
if false; then echo a; elif false; then echo c; else echo "else $?"; fi
if false; then echo a; fi
echo "no branch $?"
false
if then fi
echo "empty condition $?"
false
if true; then fi
echo "empty body $?"
if true; then false; fi
echo "body $?"
if true
then
  echo multi   # comment
else
  echo no
fi
if true; then if false; then echo x; else echo nested; fi; fi
