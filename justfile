set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

@snap *ARGS:
  cargo insta test --review {{ARGS}}

@snap-fresh *ARGS:
  cargo insta test --review --delete-unreferenced-snapshots --no-ignore {{ARGS}}
