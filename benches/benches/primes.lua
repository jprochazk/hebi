local _M = {}

function _M.primes(max_number)
  local prime_mask = {}
  for i = 1,max_number do
    prime_mask[i] = true
  end

  prime_mask[0] = false
  prime_mask[1] = false

  local total_primes_found = 0

  for p = 2, max_number do
    if not prime_mask[p] then goto continue end

    total_primes_found = total_primes_found + 1

    local i = 2 * p
    while i < max_number + 1 do
      prime_mask[i] = false
      i = i + p
    end

    ::continue::
  end

  return total_primes_found
end

return _M
