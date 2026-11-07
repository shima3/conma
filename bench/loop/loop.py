"""loop: a loop that counts, a benchmark of the simplest iteration.

    python3 loop.py N

Counts N down to 0 and an accumulator up from 0, and prints the
accumulator (N). loop.se does the same with a function that calls itself."""

import sys


def loop(n):
    acc = 0
    while n != 0:
        n -= 1
        acc += 1
    return acc


def main():
    print(loop(int(sys.argv[1])))


if __name__ == "__main__":
    main()
