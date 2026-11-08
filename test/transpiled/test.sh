#!/bin/bash
# The transpiled code of vm/conma2rust (vm/README.md, "Transpiled code"): case1.se runs inlined calls,
# primitives called from fused chains through the OList, and __Dict_lookup_or__ whose onMissing is a function.
# bench/build.sh builds its transpiled code into the transpiled VM, which CONMA_VM=transpiled then tests;
# on the other VMs, it checks the results the transpiled code must give. The module is made as
# bench/module.sh makes it, so that it has the text whose transpiled code was built.
. "$(dirname "$0")/../common.sh"
for c in case1
do
  "$root/bench/module.sh" "$PWD/$c.se" $tmp/$c.module.se || exit 1
  { "$root/test/vm.sh" --main $tmp/$c.module.se; echo "exit: $?"; } > $tmp/$c.log 2>&1
  diff $c.log $tmp/$c.log
done
