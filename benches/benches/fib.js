function fib_rec(n) {
  if (n <= 1) {
    return n;
  } else {
    return fib_rec(n - 2) + fib_rec(n - 1);
  }
}

module.exports = { fib_rec };

