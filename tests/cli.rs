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
        .output()
        .expect("run diff");
    assert_eq!(
        diff.status.code(),
        Some(0),
        "stable facts should not drift seconds apart, but these did:\n{}",
        String::from_utf8_lossy(&diff.stdout)
    );

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
