use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn aeye_plan_requires_scan() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir(repo_root.join(".git")).unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(repo_root);

    cmd.arg("plan").arg("a simple goal");

    cmd.assert().failure().stderr(predicate::str::contains(
        "System profile not found at .nlpg/system.json. Please run `a-eye scan` first.",
    ));
}

#[test]
fn aeye_plan_creates_run_artifacts() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    // Create a mock git repo and a system profile
    fs::create_dir(repo_root.join(".git")).unwrap();
    fs::create_dir(repo_root.join(".nlpg")).unwrap();
    fs::write(
        repo_root.join(".nlpg/system.json"),
        r#"{ "repoType": "test" }"#,
    )
    .unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(repo_root);

    cmd.arg("plan")
        .arg("implement a new feature")
        .arg("--scope")
        .arg("src/**/*.js");

    let output = cmd.output().unwrap();
    assert!(output.status.success(), "a-eye plan command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Starting run:"));
    assert!(stdout.contains("Plan to achieve: implement a new feature"));

    // Find the run directory
    let runs_dir = repo_root.join(".nlpg/runs");
    let run_entry = fs::read_dir(runs_dir)
        .unwrap()
        .next()
        .expect("No run directory created")
        .unwrap();
    let run_path = run_entry.path();

    assert!(run_path.join("intent.json").exists());
    assert!(run_path.join("system.json").exists());
    assert!(run_path.join("plan.json").exists());
    assert!(run_path.join("steps").exists());
    assert!(run_path.join("logs").exists());

    let intent_content = fs::read_to_string(run_path.join("intent.json")).unwrap();
    let intent: serde_json::Value = serde_json::from_str(&intent_content).unwrap();
    assert_eq!(intent["goal"], "implement a new feature");
    assert_eq!(intent["scope"][0], "src/**/*.js");
    assert_eq!(intent["riskTolerance"], "low");
}
