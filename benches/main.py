import sys
from benches.fib import fib  # noqa: F401
from benches.primes import primes  # noqa: F401


def format_iter(iter):
    units = ["s", "ms", "μs", "ns", "ps"]
    unit = 0

    while iter < 1:
        iter *= 1000
        unit += 1

    return f"{round(iter, ndigits=3)} {units[unit]}/iter"


def run_bench(str, globals, N=10000):
    iter = timeit(str, globals=globals, number=N) / float(N)
    return f"{str}: {format_iter(iter)}"


if __name__ == "__main__":
    from timeit import timeit

    if "fib" in sys.argv[1:]:
        print(run_bench("fib(15)", globals=locals()))
        print(run_bench("fib(20)", globals=locals()))
    if "primes" in sys.argv[1:]:
        print(run_bench("primes(1000000)", globals=locals(), N=100))
