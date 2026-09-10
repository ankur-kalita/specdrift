# specdrift Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a small Rust CLI that snapshots machine facts and reports drift, wrapped in a GitHub Actions pipeline built up stage by stage so a beginner learns CI by watching it grow.

**Architecture:** Four modules split along a purity line. `facts.rs` (types + JSON) and `diff.rs` (comparison) are pure and fully unit-tested; `collect.rs` (sysinfo queries) and `main.rs` (CLI, exit codes) touch the outside world and stay thin. The CI pipeline is added incrementally across tasks rather than all at once.

**Tech Stack:** Rust 2021 edition · `sysinfo` · `serde` + `serde_json` · `clap` · `time` · GitHub Actions

**Spec:** `docs/superpowers/specs/2026-09-10-specdrift-design.md`

## Global Constraints

- Repository: `github.com/ankur-kalita/specdrift`, **public**, default branch `main`.
- Exit codes are a hard contract: `0` no drift, `1` drift detected, `2` error.
- Snapshot facts live in a `BTreeMap<String, Fact>` — sorted key order is a **requirement**, not incidental. Serializing the same snapshot twice must produce byte-identical JSON.
- `diff.rs` must never import `sysinfo` or read any file. Purity is enforced by review.
- No test may assert a real hardware value (e.g. a specific CPU model). Such tests pass locally and fail on CI runners.
- Docker / container images are **out of scope** for this plan (deferred by user decision, 2026-09-10).
- **Spec amendment:** the spec lists four dependencies; this plan adds a fifth, `time` (feature `formatting`), to produce the RFC3339 `captured_at` timestamp the spec's JSON example shows. Rationale: hand-rolling calendar maths is not worth it, and `time` is the standard minimal choice.
- Every task ends with a commit. Tasks 2 onward also end with a push, so the pipeline is observed at each stage.

---

### Task 1: Repository skeleton pushed to GitHub

Establishes the repo with a working Rust project and **no CI at all** — a deliberate baseline so the first green checkmark in Task 2 is visibly caused by something we added.

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`
- Create: `README.md`

- [ ] **Step 1: Scaffold the Rust project in the existing folder**

The folder already exists and is already a git repo containing `docs/`. Do **not** run `cargo new` (it refuses a non-empty directory). Run:

```bash
cargo init --name specdrift
```

Expected: creates `Cargo.toml`, `src/main.rs`, `.gitignore`.

- [ ] **Step 2: Verify it builds and runs**

```bash
cargo run
```

Expected: prints `Hello, world!`

- [ ] **Step 3: Set the crate metadata**

Replace `Cargo.toml` with:

```toml
[package]
name = "specdrift"
version = "0.1.0"
edition = "2021"
description = "Detects what changed about your machine since a saved baseline"
license = "MIT"

[dependencies]
```

- [ ] **Step 4: Write the README**

Create `README.md`:

```markdown
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
```

- [ ] **Step 5: Confirm `.gitignore` ignores build output**

`cargo init` writes `/target`. Verify:

```bash
cat .gitignore
```

Expected: contains `/target`. If missing, add it — committing `target/` would add hundreds of megabytes.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: scaffold specdrift Rust project"
```

- [ ] **Step 7: Create the GitHub repo and push**

```bash
gh repo create specdrift --public --source=. --remote=origin --push
```

Expected: repo created, `main` pushed.

- [ ] **Step 8: Confirm there is no CI**

```bash
gh run list
```

Expected: no runs. **This is the point** — the repo is live and nothing is checking it.

---

### Task 2: The smallest possible pipeline

One workflow, one job, three steps. Every line is explainable before we add complexity.

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the workflow**

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
  pull_request:

jobs:
  check:
    name: Build and test
    runs-on: ubuntu-latest
    steps:
      - name: Get the code
        uses: actions/checkout@v4

      - name: Build
        run: cargo build --verbose

      - name: Test
        run: cargo test --verbose
```

Line-by-line, for the learner:
- `on:` — what wakes the pipeline. Here: any push, and any pull request.
- `jobs:` — the list of work. One job, named `check`.
- `runs-on: ubuntu-latest` — GitHub creates a fresh Linux machine for this job.
- `uses: actions/checkout@v4` — a prewritten step that copies your repo onto that machine. **Without it the machine is empty.** This is the single most common beginner mistake.
- `run:` — a shell command. Rust is preinstalled on GitHub's runners, so `cargo` just works.

- [ ] **Step 2: Commit and push**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add minimal build-and-test workflow"
git push
```

- [ ] **Step 3: Watch it run**

```bash
gh run watch
```

Expected: the `check` job succeeds. Green checkmark.

- [ ] **Step 4: Read the log**

```bash
gh run view --log
```

Expected: shows the runner setting up, checking out, compiling, and reporting `0 passed`. Zero tests is correct — we have not written any yet.

---

### Task 3: The `facts` module

Defines what a snapshot is, and proves its JSON is deterministic.

**Files:**
- Create: `src/facts.rs`
- Modify: `src/main.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Produces: `FactClass::{Stable, Volatile}`; `Fact { value: String, class: FactClass }` with constructors `Fact::stable(impl Into<String>) -> Fact` and `Fact::volatile(impl Into<String>) -> Fact`; `Snapshot { version: u32, captured_at: String, facts: BTreeMap<String, Fact> }` with `Snapshot::new(captured_at: impl Into<String>) -> Snapshot`, `Snapshot::insert(&mut self, key: impl Into<String>, fact: Fact)`, `Snapshot::to_json(&self) -> Result<String, serde_json::Error>`, `Snapshot::from_json(&str) -> Result<Snapshot, serde_json::Error>`; constant `SNAPSHOT_VERSION: u32 = 1`.

- [ ] **Step 1: Add the serialization dependencies**

```bash
cargo add serde --features derive
cargo add serde_json
```

- [ ] **Step 2: Write the failing tests**

Create `src/facts.rs` containing **only** the test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Snapshot {
        let mut s = Snapshot::new("2026-09-10T12:00:00Z");
        s.insert("cpu.model", Fact::stable("Apple M4"));
        s.insert("memory.total", Fact::stable("17179869184"));
        s.insert("memory.available", Fact::volatile("3221225472"));
        s
    }

    #[test]
    fn json_round_trip_is_identity() {
        let original = sample();
        let json = original.to_json().expect("serialize");
        let restored = Snapshot::from_json(&json).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn serialization_is_deterministic() {
        let first = sample().to_json().expect("serialize");
        let second = sample().to_json().expect("serialize");
        assert_eq!(first, second);
    }

    #[test]
    fn keys_serialize_in_sorted_order() {
        let json = sample().to_json().expect("serialize");
        let cpu = json.find("cpu.model").expect("cpu.model present");
        let avail = json.find("memory.available").expect("memory.available present");
        let total = json.find("memory.total").expect("memory.total present");
        assert!(cpu < avail, "cpu.model must come before memory.available");
        assert!(avail < total, "memory.available must come before memory.total");
    }

    #[test]
    fn snapshot_records_the_version() {
        assert_eq!(sample().version, SNAPSHOT_VERSION);
    }
}
```

Add to the top of `src/main.rs`:

```rust
mod facts;
```

- [ ] **Step 3: Run the tests to verify they fail**

```bash
cargo test
```

Expected: compilation errors — `cannot find type Snapshot`, `cannot find type Fact`. Failing to compile *is* a failing test here.

- [ ] **Step 4: Write the implementation**

Insert above the test module in `src/facts.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version of the snapshot file format.
pub const SNAPSHOT_VERSION: u32 = 1;

/// Whether a fact is expected to hold still or change constantly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FactClass {
    /// Does not change during normal use: CPU model, total RAM, OS name.
    Stable,
    /// Changes moment to moment: free memory, battery level, uptime.
    Volatile,
}

/// One recorded thing about a machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    pub value: String,
    pub class: FactClass,
}

impl Fact {
    pub fn stable(value: impl Into<String>) -> Self {
        Fact { value: value.into(), class: FactClass::Stable }
    }

    pub fn volatile(value: impl Into<String>) -> Self {
        Fact { value: value.into(), class: FactClass::Volatile }
    }
}

/// Everything recorded about a machine at one moment.
///
/// `facts` is a `BTreeMap` on purpose: it keeps keys sorted, so serializing
/// the same snapshot twice produces byte-identical JSON. Without that, every
/// diff would be full of phantom changes caused by reordering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub captured_at: String,
    pub facts: BTreeMap<String, Fact>,
}

impl Snapshot {
    pub fn new(captured_at: impl Into<String>) -> Self {
        Snapshot {
            version: SNAPSHOT_VERSION,
            captured_at: captured_at.into(),
            facts: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, fact: Fact) {
        self.facts.insert(key.into(), fact);
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo test
```

Expected: `4 passed`.

- [ ] **Step 6: Commit and push**

```bash
git add -A
git commit -m "feat: add Snapshot and Fact types with deterministic JSON"
git push
```

- [ ] **Step 7: Watch CI run the real tests**

```bash
gh run watch
```

Expected: green, and the log shows `test result: ok. 4 passed`. Those tests just ran on a Linux machine in a data centre.

---

### Task 4: The `diff` module

The heart of the tool, and the payoff for the purity split — this compares two plain structs and touches nothing else.

**Files:**
- Create: `src/diff.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `Snapshot`, `Fact`, `FactClass` from Task 3.
- Produces: `Change::{Added { key, value }, Removed { key, value }, Changed { key, from, to }}` (all fields `String`), and `diff(baseline: &Snapshot, current: &Snapshot, include_volatile: bool) -> Vec<Change>`.

- [ ] **Step 1: Write the failing tests**

Create `src/diff.rs` containing only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{Fact, Snapshot};

    fn snap(pairs: &[(&str, Fact)]) -> Snapshot {
        let mut s = Snapshot::new("2026-09-10T12:00:00Z");
        for (key, fact) in pairs {
            s.insert(*key, fact.clone());
        }
        s
    }

    #[test]
    fn identical_snapshots_have_no_changes() {
        let a = snap(&[("cpu.model", Fact::stable("Apple M4"))]);
        let b = snap(&[("cpu.model", Fact::stable("Apple M4"))]);
        assert!(diff(&a, &b, false).is_empty());
    }

    #[test]
    fn changed_stable_value_is_reported() {
        let a = snap(&[("os.version", Fact::stable("15.6"))]);
        let b = snap(&[("os.version", Fact::stable("15.7"))]);
        assert_eq!(
            diff(&a, &b, false),
            vec![Change::Changed {
                key: "os.version".to_string(),
                from: "15.6".to_string(),
                to: "15.7".to_string(),
            }]
        );
    }

    #[test]
    fn new_key_is_reported_as_added() {
        let a = snap(&[]);
        let b = snap(&[("disk.usb", Fact::stable("62.9 GB"))]);
        assert_eq!(
            diff(&a, &b, false),
            vec![Change::Added {
                key: "disk.usb".to_string(),
                value: "62.9 GB".to_string(),
            }]
        );
    }

    #[test]
    fn missing_key_is_reported_as_removed() {
        let a = snap(&[("disk.usb", Fact::stable("62.9 GB"))]);
        let b = snap(&[]);
        assert_eq!(
            diff(&a, &b, false),
            vec![Change::Removed {
                key: "disk.usb".to_string(),
                value: "62.9 GB".to_string(),
            }]
        );
    }

    #[test]
    fn volatile_changes_are_ignored_by_default() {
        let a = snap(&[("memory.available", Fact::volatile("9000"))]);
        let b = snap(&[("memory.available", Fact::volatile("3000"))]);
        assert!(diff(&a, &b, false).is_empty());
    }

    #[test]
    fn volatile_changes_are_reported_when_requested() {
        let a = snap(&[("memory.available", Fact::volatile("9000"))]);
        let b = snap(&[("memory.available", Fact::volatile("3000"))]);
        assert_eq!(
            diff(&a, &b, true),
            vec![Change::Changed {
                key: "memory.available".to_string(),
                from: "9000".to_string(),
                to: "3000".to_string(),
            }]
        );
    }

    #[test]
    fn changes_come_back_in_sorted_key_order() {
        let a = snap(&[("zulu", Fact::stable("1")), ("alpha", Fact::stable("1"))]);
        let b = snap(&[("zulu", Fact::stable("2")), ("alpha", Fact::stable("2"))]);
        let changes = diff(&a, &b, false);
        let keys: Vec<&str> = changes.iter().map(|c| c.key()).collect();
        assert_eq!(keys, vec!["alpha", "zulu"]);
    }
}
```

Add to `src/main.rs`:

```rust
mod diff;
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test
```

Expected: compilation errors — `cannot find function diff`, `cannot find type Change`.

- [ ] **Step 3: Write the implementation**

Insert above the test module in `src/diff.rs`:

```rust
use crate::facts::{FactClass, Snapshot};
use std::collections::BTreeSet;

/// One difference between two snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Added { key: String, value: String },
    Removed { key: String, value: String },
    Changed { key: String, from: String, to: String },
}

impl Change {
    pub fn key(&self) -> &str {
        match self {
            Change::Added { key, .. } => key,
            Change::Removed { key, .. } => key,
            Change::Changed { key, .. } => key,
        }
    }
}

/// Compare two snapshots.
///
/// Volatile facts are skipped unless `include_volatile` is true — otherwise
/// free memory alone would make every comparison report drift.
///
/// This function deliberately never touches the operating system. It takes two
/// plain structs and returns a plain list, which is why it can be tested on any
/// machine, including a CI runner with completely different hardware.
pub fn diff(baseline: &Snapshot, current: &Snapshot, include_volatile: bool) -> Vec<Change> {
    let mut keys: BTreeSet<&str> = BTreeSet::new();
    keys.extend(baseline.facts.keys().map(String::as_str));
    keys.extend(current.facts.keys().map(String::as_str));

    let mut changes = Vec::new();

    for key in keys {
        let before = baseline.facts.get(key);
        let after = current.facts.get(key);

        // Classify by whichever side we have; prefer the current one.
        let class = after
            .or(before)
            .map(|fact| fact.class)
            .unwrap_or(FactClass::Stable);

        if class == FactClass::Volatile && !include_volatile {
            continue;
        }

        match (before, after) {
            (Some(before), Some(after)) if before.value != after.value => {
                changes.push(Change::Changed {
                    key: key.to_string(),
                    from: before.value.clone(),
                    to: after.value.clone(),
                });
            }
            (Some(_), Some(_)) => {}
            (None, Some(after)) => changes.push(Change::Added {
                key: key.to_string(),
                value: after.value.clone(),
            }),
            (Some(before), None) => changes.push(Change::Removed {
                key: key.to_string(),
                value: before.value.clone(),
            }),
            (None, None) => unreachable!("key came from one of the two maps"),
        }
    }

    changes
}
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo test
```

Expected: `11 passed` (4 from facts, 7 from diff).

- [ ] **Step 5: Commit and push**

```bash
git add -A
git commit -m "feat: add pure snapshot diffing with volatile filtering"
git push
```

- [ ] **Step 6: Watch CI**

```bash
gh run watch
```

Expected: green, `11 passed`.

---

### Task 5: Break the pipeline on purpose

A teaching task with no lasting deliverable. The goal is to see red and learn to read it. Do not skip it — a pipeline that has only ever been green teaches nothing about diagnosis.

**Files:**
- Modify: `src/diff.rs` (temporarily, then reverted)

- [ ] **Step 1: Make a branch so `main` stays clean**

```bash
git checkout -b break-it-on-purpose
```

- [ ] **Step 2: Introduce a real bug**

In `src/diff.rs`, invert the volatile check — change:

```rust
        if class == FactClass::Volatile && !include_volatile {
```

to:

```rust
        if class == FactClass::Volatile && include_volatile {
```

- [ ] **Step 3: Confirm it fails locally first**

```bash
cargo test
```

Expected: `volatile_changes_are_ignored_by_default` and `volatile_changes_are_reported_when_requested` both FAIL. Read the output: Rust prints the expected value and the actual value.

- [ ] **Step 4: Push it anyway**

```bash
git add src/diff.rs
git commit -m "test: deliberately break volatile filtering to study a red pipeline"
git push -u origin break-it-on-purpose
```

- [ ] **Step 5: Watch it fail**

```bash
gh run watch
```

Expected: the job fails. `gh run watch` exits non-zero.

- [ ] **Step 6: Read the failure the way you will in real life**

```bash
gh run view --log-failed
```

Expected: only the failing step's output. Confirm you can find, in order: which job failed, which step, which test, and the expected-vs-actual values. **That sequence — job, step, test, values — is the whole skill.**

- [ ] **Step 7: Throw the branch away**

```bash
git checkout main
git branch -D break-it-on-purpose
git push origin --delete break-it-on-purpose
```

Expected: `main` is untouched and still green.

---

### Task 6: Collect real machine facts

The only module that talks to the hardware. Kept deliberately dumb: it fills a map and nothing else.

**Files:**
- Create: `src/collect.rs`
- Modify: `src/main.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: `Snapshot`, `Fact` from Task 3.
- Produces: `collect() -> Snapshot`.

- [ ] **Step 1: Add the dependencies**

```bash
cargo add sysinfo
cargo add time --features formatting
```

- [ ] **Step 2: Check the sysinfo API version before writing code**

```bash
cargo tree | grep sysinfo
```

`sysinfo` changed its API substantially at 0.30. The code below targets the 0.30+ shape (`System::new_all()`, associated functions like `System::name()`, and a separate `Disks` type). If the resolved version is older, or a method does not exist, open the docs for the exact version rather than guessing:

```bash
cargo doc --open -p sysinfo
```

- [ ] **Step 3: Write the collector**

Create `src/collect.rs`:

```rust
use crate::facts::{Fact, Snapshot};
use sysinfo::{Disks, System};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// Ask this machine what it is made of.
///
/// This is the only place in the program that talks to the operating system.
/// It contains no decisions — it just fills in a map — so there is very little
/// here that can be wrong, and nothing here that needs testing on three
/// different operating systems.
pub fn collect() -> Snapshot {
    let captured_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());

    let mut snapshot = Snapshot::new(captured_at);

    let mut system = System::new_all();
    system.refresh_all();

    // --- operating system -------------------------------------------------
    if let Some(name) = System::name() {
        snapshot.insert("os.name", Fact::stable(name));
    }
    if let Some(version) = System::os_version() {
        snapshot.insert("os.version", Fact::stable(version));
    }
    if let Some(kernel) = System::kernel_version() {
        snapshot.insert("os.kernel", Fact::stable(kernel));
    }
    snapshot.insert("os.arch", Fact::stable(std::env::consts::ARCH));

    // --- cpu --------------------------------------------------------------
    snapshot.insert("cpu.count", Fact::stable(system.cpus().len().to_string()));
    if let Some(cpu) = system.cpus().first() {
        snapshot.insert("cpu.brand", Fact::stable(cpu.brand().trim()));
        snapshot.insert("cpu.frequency_mhz", Fact::stable(cpu.frequency().to_string()));
    }

    // --- memory (bytes) ---------------------------------------------------
    snapshot.insert("memory.total", Fact::stable(system.total_memory().to_string()));
    snapshot.insert("memory.available", Fact::volatile(system.available_memory().to_string()));
    snapshot.insert("memory.used", Fact::volatile(system.used_memory().to_string()));
    snapshot.insert("swap.total", Fact::stable(system.total_swap().to_string()));

    // --- disks ------------------------------------------------------------
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        let mount = disk.mount_point().display().to_string();
        snapshot.insert(
            format!("disk.{mount}.total"),
            Fact::stable(disk.total_space().to_string()),
        );
        snapshot.insert(
            format!("disk.{mount}.available"),
            Fact::volatile(disk.available_space().to_string()),
        );
    }

    // --- uptime -----------------------------------------------------------
    snapshot.insert("system.uptime_secs", Fact::volatile(System::uptime().to_string()));

    snapshot
}
```

Note the classifications: capacities are `stable`, free space and uptime are `volatile`. Getting this wrong is what makes a drift tool noisy.

- [ ] **Step 4: Wire the module in**

Add to `src/main.rs`:

```rust
mod collect;
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo build
```

Expected: compiles. Warnings about unused functions are fine — `main.rs` does not call it yet.

- [ ] **Step 6: Commit and push**

```bash
git add -A
git commit -m "feat: collect machine facts via sysinfo"
git push
```

---

### Task 7: The command-line interface

Turns the library into a usable program, and puts the exit-code contract into effect.

**Files:**
- Modify: `src/main.rs`
- Create: `tests/cli.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: `collect()`, `diff()`, `Change`, `Snapshot`.
- Produces: binary `specdrift` with subcommands `snapshot` and `diff`.

- [ ] **Step 1: Add clap**

```bash
cargo add clap --features derive
```

- [ ] **Step 2: Write the failing integration test**

Create `tests/cli.rs`:

```rust
use std::process::Command;

/// `env!("CARGO_BIN_EXE_specdrift")` is filled in by Cargo with the path to the
/// compiled binary, so the test runs the real program end to end.
fn specdrift() -> Command {
    Command::new(env!("CARGO_BIN_EXE_specdrift"))
}

#[test]
fn snapshot_then_diff_against_itself_reports_no_drift() {
    let dir = std::env::temp_dir().join(format!("specdrift-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("make temp dir");
    let baseline = dir.join("baseline.json");

    let snap = specdrift()
        .args(["snapshot", "-o"])
        .arg(&baseline)
        .status()
        .expect("run snapshot");
    assert!(snap.success(), "snapshot should exit 0");

    let diff = specdrift()
        .arg("diff")
        .arg(&baseline)
        .status()
        .expect("run diff");
    assert_eq!(diff.code(), Some(0), "stable facts should not drift seconds apart");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn diff_against_a_missing_file_exits_two() {
    let status = specdrift()
        .args(["diff", "definitely-not-a-real-file.json"])
        .status()
        .expect("run diff");
    assert_eq!(status.code(), Some(2), "errors must exit 2, not 1");
}
```

- [ ] **Step 3: Run to verify it fails**

```bash
cargo test --test cli
```

Expected: FAIL — the binary does not understand these arguments yet.

- [ ] **Step 4: Write main.rs**

Replace `src/main.rs` entirely:

```rust
mod collect;
mod diff;
mod facts;

use clap::{Parser, Subcommand};
use diff::Change;
use facts::Snapshot;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "specdrift", version, about = "Detect what changed about your machine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Record what this machine looks like right now
    Snapshot {
        /// Where to write the snapshot
        #[arg(short, long, default_value = "specdrift.json")]
        output: PathBuf,
    },
    /// Compare this machine against a saved snapshot
    Diff {
        /// The snapshot file to compare against
        baseline: PathBuf,
        /// Also compare facts that change constantly, like free memory
        #[arg(long)]
        all: bool,
    },
}

/// Exit codes are the contract with CI: 0 fine, 1 drift, 2 broken.
const EXIT_OK: u8 = 0;
const EXIT_DRIFT: u8 = 1;
const EXIT_ERROR: u8 = 2;

fn main() -> ExitCode {
    match Cli::parse().command {
        Commands::Snapshot { output } => run_snapshot(output),
        Commands::Diff { baseline, all } => run_diff(baseline, all),
    }
}

fn run_snapshot(output: PathBuf) -> ExitCode {
    let snapshot = collect::collect();

    let json = match snapshot.to_json() {
        Ok(json) => json,
        Err(err) => return fail(format!("could not build the snapshot: {err}")),
    };

    if let Err(err) = std::fs::write(&output, json) {
        return fail(format!("could not write {}: {err}", output.display()));
    }

    println!("saved {} facts to {}", snapshot.facts.len(), output.display());
    ExitCode::from(EXIT_OK)
}

fn run_diff(baseline_path: PathBuf, all: bool) -> ExitCode {
    let text = match std::fs::read_to_string(&baseline_path) {
        Ok(text) => text,
        Err(err) => return fail(format!("could not read {}: {err}", baseline_path.display())),
    };

    let baseline: Snapshot = match Snapshot::from_json(&text) {
        Ok(snapshot) => snapshot,
        Err(err) => return fail(format!("{} is not a valid snapshot: {err}", baseline_path.display())),
    };

    let current = collect::collect();
    let changes = diff::diff(&baseline, &current, all);

    if changes.is_empty() {
        println!("no drift since {}", baseline.captured_at);
        return ExitCode::from(EXIT_OK);
    }

    for change in &changes {
        match change {
            Change::Added { key, value } => println!("  + {key}  {value}"),
            Change::Removed { key, value } => println!("  - {key}  {value}"),
            Change::Changed { key, from, to } => println!("  ~ {key}  {from} -> {to}"),
        }
    }

    let noun = if changes.len() == 1 { "change" } else { "changes" };
    println!("{} {noun} since {}", changes.len(), baseline.captured_at);
    ExitCode::from(EXIT_DRIFT)
}

fn fail(message: String) -> ExitCode {
    eprintln!("specdrift: {message}");
    ExitCode::from(EXIT_ERROR)
}
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo test
```

Expected: all unit tests plus `2 passed` in `cli`.

- [ ] **Step 6: Use the real program**

```bash
cargo run -- snapshot -o /tmp/baseline.json
cargo run -- diff /tmp/baseline.json
echo "exit code: $?"
```

Expected: the snapshot reports a fact count; the diff reports no drift and exit code `0`.

Then prove drift is detected:

```bash
cargo run -- diff /tmp/baseline.json --all
echo "exit code: $?"
```

Expected: free memory and uptime show as changed, exit code `1`.

- [ ] **Step 7: Commit and push**

```bash
git add -A
git commit -m "feat: add snapshot and diff subcommands with CI exit codes"
git push
```

---

### Task 8: Grow the pipeline into three jobs

The one small job becomes the fan-out shape: lint and test in parallel, then a three-OS build. Caching goes in here so the speed difference is visible against the previous runs.

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Fix formatting and lints locally first**

CI is about to reject anything sloppy, so clean it up before pushing:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
```

Expected: `cargo fmt` rewrites files silently; clippy reports no warnings. Fix anything it flags.

- [ ] **Step 2: Replace the workflow**

Replace `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
  pull_request:

# Cancel an in-progress run if you push again to the same branch.
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

jobs:
  lint:
    name: Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install rustfmt and clippy
        run: rustup component add rustfmt clippy

      - name: Cache Rust build files
        uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Check lints
        run: cargo clippy --all-targets -- -D warnings

  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: Swatinem/rust-cache@v2
      - name: Run tests
        run: cargo test --all --verbose

  build:
    name: Build (${{ matrix.os }})
    needs: [lint, test]
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: Swatinem/rust-cache@v2

      - name: Build release binary
        run: cargo build --release

      - name: Prove the binary actually runs
        run: cargo run --release -- snapshot -o snapshot.json

      - name: Save the binary
        uses: actions/upload-artifact@v4
        with:
          name: specdrift-${{ matrix.os }}
          path: |
            target/release/specdrift
            target/release/specdrift.exe
          if-no-files-found: error
```

What is new, for the learner:
- `needs: [lint, test]` — `build` waits. `lint` and `test` have no `needs`, so they start together.
- `strategy.matrix.os` — one job definition, three machines. `${{ matrix.os }}` is substituted per run.
- `fail-fast: false` — without it, one OS failing cancels the other two, and you learn less.
- `Swatinem/rust-cache@v2` — stores compiled dependencies between runs.
- `upload-artifact` — the runner is destroyed afterwards; anything not uploaded is gone. Both paths are listed because Windows produces `.exe`; missing paths are ignored.
- `concurrency` — pushing twice quickly cancels the stale run instead of queueing it.

- [ ] **Step 3: Commit and push**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: split into lint, test, and a three-OS build matrix"
git push
```

- [ ] **Step 4: Watch the shape change**

```bash
gh run watch
```

Expected: five jobs total — `lint` and `test` side by side, then three `build` jobs. Open the run in the browser to see the graph drawn:

```bash
gh run view --web
```

- [ ] **Step 5: Push again and compare timings**

Make a trivial change, push, and compare the second run's duration against the first. The cache should cut build time substantially.

```bash
gh run list --limit 5
```

---

### Task 9: Release binaries on a version tag

A second workflow triggered only by a tag, demonstrating conditional runs and least-privilege permissions.

**Files:**
- Create: `.github/workflows/release.yml`
- Modify: `README.md`

- [ ] **Step 1: Write the release workflow**

Create `.github/workflows/release.yml`:

```yaml
name: Release

# Only runs for tags that look like v1.2.3 — never on ordinary pushes.
on:
  push:
    tags:
      - "v*"

# GITHUB_TOKEN is read-only by default. Publishing a release needs write access,
# and granting it explicitly here (rather than globally) is least privilege.
permissions:
  contents: write

jobs:
  release:
    name: Release (${{ matrix.os }})
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            asset: specdrift-linux-x86_64.tar.gz
          - os: macos-latest
            asset: specdrift-macos-arm64.tar.gz
          - os: windows-latest
            asset: specdrift-windows-x86_64.zip
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: Swatinem/rust-cache@v2

      - name: Build release binary
        run: cargo build --release

      - name: Package (Unix)
        if: runner.os != 'Windows'
        run: tar -czf ${{ matrix.asset }} -C target/release specdrift

      - name: Package (Windows)
        if: runner.os == 'Windows'
        run: Compress-Archive -Path target/release/specdrift.exe -DestinationPath ${{ matrix.asset }}

      - name: Attach to the release
        uses: softprops/action-gh-release@v2
        with:
          files: ${{ matrix.asset }}
          generate_release_notes: true
```

New ideas here:
- `on.push.tags` — this workflow ignores normal pushes entirely.
- `permissions: contents: write` — without it, the publish step fails with a 403. Worth causing on purpose if you want to see it.
- `matrix.include` — pairs each OS with its own asset name, instead of a plain list.
- `if: runner.os == 'Windows'` — steps that only run on some machines.

- [ ] **Step 2: Commit and push the workflow**

```bash
git add .github/workflows/release.yml
git commit -m "ci: publish release binaries on version tags"
git push
```

- [ ] **Step 3: Confirm it did not run**

```bash
gh run list --limit 3
```

Expected: only CI ran. The release workflow stayed asleep because no tag was pushed. **That is the conditional lesson.**

- [ ] **Step 4: Tag and push the version**

```bash
git tag v0.1.0
git push origin v0.1.0
```

- [ ] **Step 5: Watch the release build**

```bash
gh run watch
```

Expected: three jobs, one per OS.

- [ ] **Step 6: Confirm the release exists**

```bash
gh release view v0.1.0
```

Expected: three downloadable assets.

- [ ] **Step 7: Add the CI badge to the README**

Add directly under the `# specdrift` heading:

```markdown
[![CI](https://github.com/ankur-kalita/specdrift/actions/workflows/ci.yml/badge.svg)](https://github.com/ankur-kalita/specdrift/actions/workflows/ci.yml)
```

Also replace the `## Status` section with:

```markdown
## Install

Download a binary for your system from the
[latest release](https://github.com/ankur-kalita/specdrift/releases/latest).
```

- [ ] **Step 8: Commit, push, and confirm the badge is green**

```bash
git add README.md
git commit -m "docs: add CI badge and install instructions"
git push
gh repo view --web
```

Expected: a green CI badge on the repository front page.

---

## Definition of Done

- [ ] `cargo test` passes locally
- [ ] CI is green on `main`, with `lint`, `test`, and three `build` jobs
- [ ] `specdrift snapshot` and `specdrift diff` work on macOS, with correct exit codes
- [ ] A `v0.1.0` release exists with binaries for three operating systems
- [ ] The README badge is green
- [ ] Every line of both workflow files can be explained without looking it up
