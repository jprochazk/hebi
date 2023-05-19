set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

snap *ARGS:
  cargo insta test --review --delete-unreferenced-snapshots --no-ignore {{ARGS}}

miri *ARGS:
  MIRIFLAGS='-Zmiri-tree-borrows -Zmiri-permissive-provenance' cargo miri {{ARGS}} --no-default-features -F __miri

example *ARGS:
  cargo run --example {{ARGS}}

examples *ARGS:
  cargo xtask examples {{ARGS}}

test:
  cargo test --all-targets
  just examples
