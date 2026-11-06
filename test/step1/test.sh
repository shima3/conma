#!/bin/bash
. "$(dirname "$0")/../common.sh"
resolve test.se
PYTHONUNBUFFERED=1 ../vm.sh --trace $tmp/test.module.se case1 2>&1 | sed -e "s|$tmp/|<tmp>/|g" > $tmp/case1.log
diff case1.log $tmp/case1.log
PYTHONUNBUFFERED=1 ../vm.sh --trace $tmp/test.module.se case2 2>&1 | sed -e "s|$tmp/|<tmp>/|g" > $tmp/case2.log
diff case2.log $tmp/case2.log
PYTHONUNBUFFERED=1 ../vm.sh --trace $tmp/test.module.se case3 2>&1 | sed -e "s|$tmp/|<tmp>/|g" > $tmp/case3.log
diff case3.log $tmp/case3.log
