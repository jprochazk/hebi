set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

test *ARGS:
  cargo insta test --review --delete-unreferenced-snapshots --no-ignore {{ARGS}}

miri *ARGS:
  MIRIFLAGS='-Zmiri-tree-borrows' cargo miri {{ARGS}}
