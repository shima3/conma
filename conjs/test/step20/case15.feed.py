# The input of case15.con.js, a line at a time with pauses, so that reading the next line waits
import sys, time
for line in ["one", "three", "five"]:
    sys.stdout.write(line + "\n")
    sys.stdout.flush()
    time.sleep(0.05)
