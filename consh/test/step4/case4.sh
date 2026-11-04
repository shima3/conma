for x in a b; do
  for y in $(echo 1; continue x; echo 2); do echo $y; done
done
