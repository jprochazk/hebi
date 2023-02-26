set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

@cli *ARGS:
  cargo run --example cli {{ARGS}}

@snap *ARGS:
  cargo insta test --workspace --review --delete-unreferenced-snapshots --no-ignore {{ARGS}}

#@miri *ARGS:
#  MIRIFLAGS="-Zmiri-disable-isolation" cargo miri --no-default-features {{ARGS}}

@time build:
  cargo clean
  cargo +nightly build --all --timings --release
