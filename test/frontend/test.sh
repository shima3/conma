#!/bin/bash
# The module of the ConMa frontend in the repository (vm/frontend.module.se, made by bin/update-frontend),
# and conma build and conma run of the Rust VM, which embeds it. The modules are of fast ConMa (include/fast
# before include/, as bin/update-frontend and conma build make them).
. "$(dirname "$0")/../common.sh"
seed="$root/vm/frontend.module.se"
rootreal="$(cd "$root"; pwd -P)"
relative() { sed -e "s|\"$rootreal/|\"conma/|g"; }
# case1: the frontend in the repository makes itself the same, byte for byte (on the VM of CONMA_VM)
(cd "$root" && PYTHONUNBUFFERED=1 test/vm.sh --main -- "$seed" --include include/fast --include include include/frontend.se) | relative > $tmp/self.se
cmp -s $tmp/self.se "$seed"
echo "fixed point: $?" > $tmp/case1.log
diff case1.log $tmp/case1.log
# case2: the Python frontend (the reference) makes the same module (layout ignored); if not, include/ was
# changed without running bin/update-frontend
"$root/bin/includer" --include "$root/include/fast" --include "$root/include" "$root/include/frontend.se" | relative | "$root/bin/resolver" > $tmp/python.se
python3 ../selfhost_compare.py ast $tmp/python.se "$seed" > $tmp/case2.log 2>&1
echo "compare: $?" >> $tmp/case2.log
diff case2.log $tmp/case2.log
# case3: conma build makes the same module as bin/includer and bin/resolver, and conma run runs a program
# without them (the Rust VM, vm/target/release/conma, built with the frontend in the repository)
conma="$root/vm/target/release/conma"
{
  "$conma" build hello.se > $tmp/built.se
  echo "build: $?"
  "$root/bin/includer" --include "$root/include/fast" --include "$root/include" hello.se | "$root/bin/resolver" > $tmp/ref.se
  python3 ../selfhost_compare.py ast $tmp/ref.se $tmp/built.se
  echo "compare: $?"
  "$conma" run hello.se a "b c"
  echo "run: $?"
  "$conma" run missing.se 2>&1
  echo "run: $?"
} > $tmp/case3.log 2>&1
diff case3.log $tmp/case3.log
# case4: the order in which include file names are searched: the include directories in the order given,
# then the directory of the including file (docs/INCLUDER.md), with the Python frontend and conma run
{
  "$root/bin/includer" --include order/fast --include order/reference --include "$root/include" order/main.se \
    | "$root/bin/resolver" > $tmp/order.se
  echo "build: $?"
  "$conma" --main $tmp/order.se
  "$conma" run --include order/fast --include order/reference order/main.se
  echo "run: $?"
  "$conma" run --include order/reference order/main.se
} > $tmp/case4.log 2>&1
diff case4.log $tmp/case4.log
