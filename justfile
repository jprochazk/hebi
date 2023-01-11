set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

@snap *ARGS:
  cargo insta test --review {{ARGS}}

@snap-fresh *ARGS:
  cargo insta test --review --delete-unreferenced-snapshots --no-ignore {{ARGS}}

@miri *ARGS:
  INSTA_FORCE_PASS="true" MIRIFLAGS="-Zmiri-disable-isolation" cargo miri {{ARGS}}
