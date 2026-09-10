# specdrift

Takes a snapshot of your machine's specs, then tells you what changed later.

```
specdrift snapshot -o baseline.json   # record what this machine looks like
specdrift diff baseline.json          # compare now against that record
```

Exit codes: `0` no drift, `1` drift found, `2` error — so it works as a CI check.

## Building

```
cargo build --release
cargo test
```

## Status

Work in progress. Built as a hands-on way to learn CI with GitHub Actions.
