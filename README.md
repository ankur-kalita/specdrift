# specdrift

[![CI](https://github.com/ankur-kalita/specdrift/actions/workflows/ci.yml/badge.svg)](https://github.com/ankur-kalita/specdrift/actions/workflows/ci.yml)

Takes a snapshot of your machine's specs, then tells you what changed later.

```
$ specdrift snapshot -o baseline.json
saved 16 facts to baseline.json

$ specdrift diff baseline.json --all
  ~ memory.available    6533578752 -> 6417760256
  ~ system.uptime_secs  249418 -> 249984
2 changes since 2026-09-10T08:50:11Z
```

Facts are classified **stable** (CPU model, total RAM, disk size) or
**volatile** (free memory, uptime, clock speed). Only stable facts are compared
by default — otherwise free memory alone would report drift every single run.
Pass `--all` to include the volatile ones.

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | No drift |
| `1` | Drift detected |
| `2` | Error — file missing or malformed |

Because of these, `specdrift` works as a CI step: it turns a pipeline red when
the machine changed.

## Install

Download a binary for your system from the
[latest release](https://github.com/ankur-kalita/specdrift/releases/latest).

## Building

```
cargo build --release
cargo test
```

## How it's put together

| File | Purity | Responsibility |
|------|--------|----------------|
| `src/facts.rs` | pure | `Snapshot` and `Fact` types, JSON serialization |
| `src/diff.rs` | pure | Compares two snapshots, returns a list of changes |
| `src/collect.rs` | impure | The only file that queries the OS, via `sysinfo` |
| `src/main.rs` | impure | CLI parsing, output, exit codes |

`diff.rs` never touches the operating system, so its logic can be tested on any
machine — including CI runners with completely different hardware.
