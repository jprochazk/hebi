local socket = require("socket")
local fib = require("./benches/fib")
local primes = require("./benches/primes")

local UNITS = {"s", "ms", "μs", "ns", "ps"}
function format_iter(iter)
  local unit = 1

  while iter < 1 do
    iter = iter * 1000
    unit = unit + 1
  end
  
  return string.format("%.3f %s/iter", iter, UNITS[unit])
end

function run_bench(name, f, N)
  local N = N or 10000

  local total = 0
  for i = 1, N do
    local start = socket.gettime()
    f()
    total = total + (socket.gettime() - start)
  end

  return string.format("%s: %s", name, format_iter(total / N))
end


local run = {}
for _, v in ipairs(arg) do
  run[v] = true
end

local ran_something = false
if run["fib"] then
  ran_something = true
  print(run_bench("fib(15)", function() fib.fib(15) end))
  print(run_bench("fib(20)", function() fib.fib(20) end))
end

if run["primes"] then
  ran_something = true
  print(run_bench("primes(1000000)", function() primes.primes(1000000) end, 1000))
end

if not ran_something then
  print("no bench specified. options: fib, primes")
end
