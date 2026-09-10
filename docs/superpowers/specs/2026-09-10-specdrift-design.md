# specdrift — Design Document

**Date:** 2026-09-10
**Status:** Approved, ready for implementation
**Author:** ankur-kalita (with Claude)

---

## 1. Why this project exists

The real goal is **learning CI** (Continuous Integration) hands-on with GitHub
Actions. The Rust program is the excuse — it needs to be small enough to finish
quickly, but real enough that a CI pipeline around it is meaningful rather than
a toy.

So there are two success criteria, and the second one is the one that matters:

1. `specdrift` works and is genuinely useful on a laptop.
2. Its CI pipeline exercises the full CI vocabulary: parallel jobs, job
   dependencies, build matrices, caching, artifacts, conditional execution,
   and tag-triggered releases.

If those conflict, criterion 2 wins.

## 2. What specdrift does

`specdrift` is a **hardware drift detector**. It records what your machine
looks like right now, and later tells you what changed.

```
$ specdrift snapshot -o baseline.json
saved 14 facts to baseline.json

  ... time passes: an OS update lands, a USB drive is plugged in ...

$ specdrift diff baseline.json
  ~ os.version          15.6 -> 15.7
  + disk./Volumes/USB   62.9 GB
2 changes (exit 1)

$ specdrift diff baseline.json --all      # include volatile facts too
  ~ os.version          15.6 -> 15.7
  + disk./Volumes/USB   62.9 GB
  ~ memory.available    9.2 GB -> 3.1 GB
  ~ battery.percent     88 -> 61
4 changes (exit 1)
```

### Why this is useful in the real world

- "My machine got slower — did something change?"
- "Is this CI runner the same shape as last week's?"
- "Did that OS update change my available disk?"

It is the same idea as a config-drift detector (Terraform plan, Ansible
`--check`), but pointed at hardware instead of infrastructure.

### CLI contract

```
specdrift snapshot [-o <file>]     Capture current machine facts
specdrift diff <file> [--all]      Compare current machine against a saved file
```

**Exit codes** — these are what make it usable *as* a CI step:

| Code | Meaning |
|------|---------|
| `0`  | No drift detected |
| `1`  | Drift detected |
| `2`  | Error (file missing, malformed JSON, unreadable) |

A CI step that runs `specdrift diff baseline.json` therefore turns a pipeline
red when the machine changed. Exit codes are the universal contract between a
program and a CI system — this project makes that concrete.

## 3. The data model

A **snapshot** is a flat, alphabetically-sorted map of `key -> fact`.

```json
{
  "version": 1,
  "captured_at": "2026-09-10T14:22:31Z",
  "facts": {
    "cpu.cores":        { "value": "10",              "class": "stable" },
    "cpu.model":        { "value": "Apple M4",        "class": "stable" },
    "memory.available": { "value": "3221225472",      "class": "volatile" },
    "memory.total":     { "value": "17179869184",     "class": "stable" },
    "os.name":          { "value": "macOS",           "class": "stable" },
    "os.version":       { "value": "15.6",            "class": "stable" }
  }
}
```

Two design decisions here, both load-bearing:

### 3.1 Flat and sorted

Facts are a flat `BTreeMap<String, Fact>`, not a nested structure. `BTreeMap`
keeps keys sorted, so **serializing the same machine twice produces
byte-identical JSON**.

This matters more than it looks. If the JSON reordered itself between runs,
every diff would be noise and the tool would be worthless. Determinism is a
requirement, not a nicety — and it is the same reason CI builds strive to be
reproducible.

### 3.2 Stable vs volatile facts

Some facts never change (`cpu.model`). Some change every second
(`memory.available`). If `diff` compared everything, it would *always* report
drift.

So every fact is classified:

- **stable** — CPU model, core count, total RAM, OS name, disk capacity
- **volatile** — available memory, load average, battery charge, uptime

`diff` compares **only stable facts by default**. `--all` includes volatile
ones for when you actually want a live comparison.

This gives the tool a real opinion, and it gives the pure logic layer something
worth unit-testing.

## 4. Architecture

Four modules. The split between pure and impure is the whole design.

```
        ┌─────────────────────────────────────────┐
        │  main.rs        arg parsing, printing,   │  impure
        │                 exit codes  (clap)       │
        └───────────────┬─────────────────────────┘
                        │
        ┌───────────────┴──────────┐
        │                          │
┌───────▼────────┐        ┌────────▼─────────┐
│ collect.rs     │        │  diff.rs         │
│ reads the OS   │        │  compares two    │  PURE
│ (sysinfo)      │        │  snapshots       │
│      impure    │        │                  │
└───────┬────────┘        └────────┬─────────┘
        │                          │
        └──────────┬───────────────┘
                   │
          ┌────────▼─────────┐
          │  facts.rs        │
          │  Snapshot, Fact, │  PURE
          │  FactClass, JSON │
          └──────────────────┘
```

| Module | Purity | Responsibility | Unit-testable |
|---|---|---|---|
| `facts.rs` | pure | `Snapshot`, `Fact`, `FactClass` types; JSON ser/de | Yes, fully |
| `diff.rs` | pure | Compare two snapshots → `Vec<Change>` | Yes, fully |
| `collect.rs` | impure | Query the OS via `sysinfo`, fill a `Snapshot` | No (kept thin) |
| `main.rs` | impure | `clap` parsing, output formatting, exit codes | Via integration test |

### Why the pure/impure split matters *for CI*

CI runs on a Linux cloud VM with completely different hardware than a MacBook.

If diffing logic were tangled with hardware reads, tests would pass locally and
fail in CI — the single most common "why is my pipeline red" experience for
beginners.

By keeping `diff.rs` pure (it takes two plain structs and never touches the
OS), CI can meaningfully test the real logic on any machine, on any OS. **This
is the design lesson hiding inside the CI lesson: testable code and
CI-friendly code are the same thing.**

`collect.rs` stays deliberately stupid — it only fills a map. There is almost
no logic in it to get wrong.

### Core types

```rust
enum FactClass { Stable, Volatile }

struct Fact { value: String, class: FactClass }

struct Snapshot {
    version: u32,
    captured_at: String,
    facts: BTreeMap<String, Fact>,
}

enum Change {
    Added   { key: String, value: String },
    Removed { key: String, value: String },
    Changed { key: String, from: String, to: String },
}

// The one function that carries the whole design:
fn diff(baseline: &Snapshot, current: &Snapshot, include_volatile: bool) -> Vec<Change>
```

### Dependencies

Four crates, all boring and standard:

| Crate | Purpose |
|---|---|
| `sysinfo` | Cross-platform hardware/OS facts |
| `serde` + `serde_json` | JSON serialization |
| `clap` | CLI argument parsing |

## 5. Testing strategy

Test-driven: `diff.rs` is written test-first, because it is pure and the tests
are trivial to express.

**Unit tests — `diff.rs`**
- identical snapshots → no changes
- a changed stable value → one `Changed`
- a new key → one `Added`
- a removed key → one `Removed`
- a changed *volatile* value → **no change** by default
- the same, with `include_volatile: true` → one `Changed`

**Unit tests — `facts.rs`**
- JSON round-trip: `Snapshot → JSON → Snapshot` is identity
- key ordering is deterministic across serializations

**Integration test — `tests/cli.rs`**
- run `snapshot`, then `diff` the result against the live machine → exit `0`

**What we deliberately do NOT test**

No test asserts a real hardware value, e.g. `cpu.model == "Apple M4"`. Such a
test passes on the developer's Mac and fails instantly on GitHub's Linux
runners.

This absence is intentional and is itself part of the curriculum: it is the
clearest possible demonstration of what "portable test" means, and why CI
catches assumptions that local development hides.

## 6. The CI pipeline

Two workflow files, because they answer two different questions.

### 6.1 `.github/workflows/ci.yml` — "is this code good?"

Triggers on every `push` and `pull_request`.

```
  lint  ──┐
          ├──> build  (ubuntu · macos · windows)
  test  ──┘
```

| Job | Runs | Purpose |
|---|---|---|
| `lint` | ubuntu | `cargo fmt --check`, `cargo clippy -- -D warnings` |
| `test` | ubuntu | `cargo test` |
| `build` | 3-OS matrix | `cargo build --release`, upload artifacts |

- `lint` and `test` run **in parallel** — neither depends on the other.
- `build` declares `needs: [lint, test]`, so it starts only if both pass.
- `build` uses `strategy.matrix` to run the same steps on three operating
  systems from one job definition.

This fan-out / fan-in shape is where `needs:` and `strategy.matrix` stop being
syntax and become obvious.

**Caching:** both jobs use `Swatinem/rust-cache`. Rust CI without caching takes
roughly 3 minutes per job; with it, roughly 30 seconds. The difference is
visible in the Actions UI, which makes the lesson stick.

### 6.2 `.github/workflows/release.yml` — "ship it"

Triggers **only** on a tag matching `v*`.

- Builds release binaries on the 3-OS matrix
- Packages them (`.tar.gz` on Unix, `.zip` on Windows)
- Attaches them to a GitHub Release

This workflow needs `permissions: contents: write`, because `GITHUB_TOKEN` is
read-only by default. That single line is a concrete lesson in CI least
privilege.

### 6.3 CI concepts this pipeline teaches

| Concept | Where it appears |
|---|---|
| Workflow triggers (`on:`) | push / PR vs. tag |
| Parallel jobs | `lint` alongside `test` |
| Job dependencies | `needs: [lint, test]` |
| Build matrix | 3 operating systems from one definition |
| Caching | `Swatinem/rust-cache` |
| Artifacts | `upload-artifact` in `build` |
| Conditional execution | tag-only release workflow |
| Permissions / least privilege | `contents: write` |
| Exit codes as the CI contract | specdrift's own `0` / `1` / `2` |

## 7. Build order

The pipeline is built **incrementally and pushed at every step**, so the
pipeline is watched growing rather than appearing finished.

| Step | What is added | What is observed |
|---|---|---|
| 1 | Repo, `cargo init`, first commit | Push with no CI — a baseline |
| 2 | Minimal one-job workflow | First green checkmark |
| 3 | `facts.rs` + `diff.rs` via TDD | Real tests running in the cloud |
| 4 | `collect.rs` + `main.rs` | A working CLI |
| 5 | Split into lint / test / build matrix | The job graph fans out |
| 6 | `release.yml`, tag `v0.1.0` | Real binaries on a Release page |

At one point a workflow is **deliberately broken** in order to read a failed CI
log. Reading a red pipeline is the actual skill; a pipeline that only ever
passes teaches nothing about diagnosis.

## 8. Non-goals

Explicitly out of scope, to keep the project small:

- No TUI, no live monitoring, no graphs
- No historical database — snapshots are plain files the user manages
- No remote/agent mode
- No Docker image (the CI lesson is complete without it)
- No `crates.io` publishing
- No Windows-specific hardware beyond what `sysinfo` gives for free

## 9. Success criteria

The project is done when:

1. `cargo test` passes locally and on all three CI operating systems
2. `specdrift snapshot` and `specdrift diff` work end to end on macOS
3. The CI badge on the README is green
4. A `v0.1.0` GitHub Release exists with downloadable binaries for three OSes
5. Every line of both workflow files can be explained from memory
