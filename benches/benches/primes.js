function primes(max_number) {
  const prime_mask = Array(max_number).fill(true);

  prime_mask[0] = false;
  prime_mask[1] = false;

  let total_primes_found = 0;

  for (let p = 2; p <= max_number; p += 1) {
    if (!prime_mask[p]) continue;

    total_primes_found += 1;

    let i = 2 * p;
    while (i < max_number + 1) {
      prime_mask[i] = false;
      i += p;
    }
  }

  return total_primes_found;
}

module.exports = { primes };

