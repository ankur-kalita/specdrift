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

    println!(
        "saved {} facts to {}",
        snapshot.facts.len(),
        output.display()
    );
    ExitCode::from(EXIT_OK)
}

fn run_diff(baseline_path: PathBuf, all: bool) -> ExitCode {
    let text = match std::fs::read_to_string(&baseline_path) {
        Ok(text) => text,
        Err(err) => {
            return fail(format!("could not read {}: {err}", baseline_path.display()))
        }
    };

    let baseline: Snapshot = match Snapshot::from_json(&text) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            return fail(format!(
                "{} is not a valid snapshot: {err}",
                baseline_path.display()
            ))
        }
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
