set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

@snap *ARGS:
  cargo insta test --review --no-ignore --delete-unreferenced-snapshots {{ARGS}}
