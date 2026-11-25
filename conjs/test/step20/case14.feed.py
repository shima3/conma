# The input of case14.con.js, a line at a time with pauses, so that reading the next line waits
import sys, time
for line in ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"]:
    sys.stdout.write(line + "\n")
    sys.stdout.flush()
    time.sleep(0.05)
