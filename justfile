set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

@snap *ARGS:
  RUST_BACKTRACE=full cargo insta test --review --no-ignore --delete-unreferenced-snapshots {{ARGS}}
