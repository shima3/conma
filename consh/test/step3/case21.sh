false; for x in a; do false; done; echo "for $?"
for x in a; do false; break; done; echo "break $?"
