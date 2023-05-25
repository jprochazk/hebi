const process = require("process");
const Benchmark = require("benchmark");
const { fib_rec } = require("./benches/fib.js");
const { primes } = require("./benches/primes.js");

const UNITS = ["s", "ms", "μs", "ns", "ps"];
function hz_to_iter(hz) {
  let n_per_iter = 1 / hz;
  let unit = 0;

  while (n_per_iter < 1) {
    n_per_iter *= 1000;
    unit += 1;
  }

  return `${n_per_iter.toFixed(3)} ${UNITS[unit]}/iter`;
}

function create_suite() {
  return new Benchmark.Suite({
    onCycle(event) {
      const name = event.target.name;
      const iter = hz_to_iter(event.target.hz);
      const samples = event.target.stats.sample.length;
      console.log(`${name}: ${iter} (${samples} samples)`);
    },
  });
}

const FIB = "fib";
const PRIMES = "primes";

function main(benches) {
  if (benches.size === 0) {
    console.warn(`no benches provided. options: ${[FIB, PRIMES].join(", ")}`);
    process.exit(0);
  }

  if (benches.has("fib")) {
    const suite = create_suite();
    suite.add("fib_rec(15)", function () {
      return fib_rec(15);
    });
    suite.add("fib_rec(20)", function () {
      return fib_rec(20);
    });
    suite.run();
  }

  if (benches.has("primes")) {
    const suite = create_suite();
    suite.add("primes(1000000)", function () {
      return primes(1_000_000);
    });
    suite.run();
  }
}

main(new Set(process.argv.slice(2)));

