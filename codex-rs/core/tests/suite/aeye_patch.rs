use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn aeye_patch_creates_diff_artifact() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    // 1. Set up a fake run directory from a `plan` command.
    let run_dir = repo_root.join(".nlpg/runs/20240101-120000-ABCDEF");
    fs::create_dir_all(&run_dir).unwrap();

    let plan_path = run_dir.join("plan.json");
    fs::write(
        &plan_path,
        r#"{ "title": "Test Plan", "steps": ["Do a thing"] }"#,
    )
    .unwrap();

    fs::write(
        run_dir.join("intent.json"),
        r#"{ "id": "1", "goal": "Test Goal", "constraints": [], "environment": {"os": "os", "shell": "shell"}, "riskTolerance": "low", "scope": [], "successCriteria": [] }"#,
    )
    .unwrap();

    fs::write(
        run_dir.join("system.json"),
        r#"{ "repoType": "test", "frameworks": [], "languages": [], "entrypoints": [], "verifyCommands": [], "topology": "single_service", "riskZones": [], "writeAllowlist": [], "readAllowlist": [] }"#,
    )
    .unwrap();

    // 2. Run `a-eye patch`
    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(repo_root);

    cmd.arg("patch")
        .arg("--from")
        .arg(plan_path.to_str().unwrap());

    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "a-eye patch command failed. Stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--- Patch Generated ---"));
    assert!(stdout.contains("a-eye apply --from"));

    // 3. Verify the artifact was created
    let patch_path = run_dir.join("patch.diff");
    assert!(patch_path.exists(), "patch.diff should be created");

    let patch_content = fs::read_to_string(patch_path).unwrap();
    assert!(patch_content.contains("--- a/src/main.rs"));
    assert!(patch_content.contains("+++ b/src/main.rs"));
    assert!(patch_content.contains("-    println!(\"Hello, world!\");"));
    assert!(patch_content.contains("+    println!(\"Hello from A-Eye!\");"));
}

#[test]
fn aeye_patch_fails_with_invalid_plan_path() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(repo_root);

    cmd.arg("patch").arg("--from").arg("nonexistent/plan.json");

    cmd.assert().failure().stderr(predicate::str::contains(
        "--from path must point to a valid plan.json file.",
    ));
}
