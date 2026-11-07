"""fib: the naive recursive Fibonacci function, a benchmark of calls.

    python3 fib.py N

Prints fib(N). fib.se computes it in the same way."""

import sys


def fib(n):
    if n == 0:
        return 0
    if n == 1:
        return 1
    return fib(n - 1) + fib(n - 2)


def main():
    print(fib(int(sys.argv[1])))


if __name__ == "__main__":
    main()
