#!/bin/bash
# The transpiled code of vm/conma2rust (vm/README.md, "Transpiled code"): case1.se runs inlined calls,
# primitives called from fused chains through the OList, and __Dict_lookup_or__ whose onMissing is a function.
# case2.se runs the channel primitives (docs/dev/CHAN-PLAN-ja.md, C4): __Chan_put__ and __Chan_take__ done in
# place when they would not wait, and left to the VM when they would (the cases of test/step45, one after another).
# case3.se runs the escape continuations of __Cont_get__ that fused chains do not make (STATIC_CONT).
# case4.se runs the array primitives (docs/dev/ARRAY-PLAN-ja.md, A4), four of them called directly (the cases of
# test/step47, one after another).
# case5.se sets a Ref and sets it back in one run of transpiled code (d_ref_set, done in place), and a VProc
# waiting for a change of the Ref wakes, as on the VM.
# bench/build.sh builds its transpiled code into the transpiled VM, which CONMA_VM=transpiled then tests;
# on the other VMs, it checks the results the transpiled code must give. The module is made as
# bench/module.sh makes it, so that it has the text whose transpiled code was built.
. "$(dirname "$0")/../common.sh"
for c in case1 case2 case3 case4 case5
do
  "$root/bench/module.sh" "$PWD/$c.se" $tmp/$c.module.se || exit 1
  { "$root/bin/vm" --main $tmp/$c.module.se; echo "exit: $?"; } > $tmp/$c.log 2>&1
  diff $c.log $tmp/$c.log
done
