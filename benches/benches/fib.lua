local _M = {}

function _M.fib(n)
  function inner(n)
    if n <= 1 then return n end
    return inner(n-2) + inner(n-1)
  end
  return inner(n)
end

return _M
