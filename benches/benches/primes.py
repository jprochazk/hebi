def primes(max_number):
    prime_mask = [True] * (max_number + 1)

    prime_mask[0] = False
    prime_mask[1] = False

    total_primes_found = 0
    for p in range(2, max_number + 1):
        if not prime_mask[p]:
            continue

        total_primes_found += 1

        i = 2 * p
        while i < max_number + 1:
            prime_mask[i] = False
            i += p

    return total_primes_found
