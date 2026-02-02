use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

// This test makes a real LLM call and requires credentials (e.g., `OPENAI_API_KEY` env var).
// It is ignored by default to prevent it from running in CI or environments without credentials.
// To run it locally, use: `cargo test -- --ignored aeye_learn_creates_summary_artifact`
#[test]
#[ignore]
fn aeye_learn_creates_summary_artifact() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    // We need a config file for the LLM call to work.
    fs::write(repo_root.join("a-eye.yaml"), "default_tier: 1").unwrap();

    // 1. Set up a fake run directory
    let run_id = "20240520-120000-LEARN1";
    let run_dir = repo_root.join(".nlpg/runs").join(run_id);
    fs::create_dir_all(&run_dir).unwrap();

    let intent_content = r#"{ "goal": "Implement learning command" }"#;
    fs::write(run_dir.join("intent.json"), intent_content).unwrap();

    let plan_content = r#"{ "title": "Learning Plan", "steps": ["Implement learn command"] }"#;
    fs::write(run_dir.join("plan.json"), plan_content).unwrap();

    let patch_content = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,1 +1,1 @@\n-fn main() {}\n+fn main() { /* learn */ }\n";
    fs::write(run_dir.join("patch.diff"), patch_content).unwrap();

    // 2. Run `a-eye learn`
    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(repo_root);

    // The `--force` flag is needed in case the test is run multiple times
    // and the summary artifact already exists.
    cmd.arg("learn").arg(run_id).arg("--force");

    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "a-eye learn command failed. Stderr: {}\nStdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--- Learning Summary ---"));
    assert!(stdout.contains("Summary saved to:"));

    // 3. Verify the artifact was created and has reasonable content
    let summary_path = run_dir.join("learning-summary.md");
    assert!(
        summary_path.exists(),
        "learning-summary.md should be created"
    );

    let summary_content = fs::read_to_string(summary_path).unwrap();
    // Check for keywords we expect the LLM to include in its summary.
    // This is more robust than checking for exact output.
    assert!(summary_content.to_lowercase().contains("goal"));
    assert!(summary_content.to_lowercase().contains("plan"));
    assert!(summary_content.to_lowercase().contains("patch"));
    assert!(summary_content.to_lowercase().contains("learning"));
    assert!(
        summary_content.len() > 50,
        "Summary content should not be trivial, but was {} bytes",
        summary_content.len()
    );
}

#[test]
fn aeye_learn_fails_for_nonexistent_run() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join(".git")).unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(repo_root);

    cmd.arg("learn").arg("nonexistent-run-id");

    cmd.assert().failure().stderr(predicate::str::contains(
        "Run with ID 'nonexistent-run-id' not found",
    ));
}

#[test]
fn aeye_learn_outputs_json() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join(".git")).unwrap();

    // 1. Set up a fake run directory
    let run_id = "20240520-130000-JSON1";
    let run_dir = repo_root.join(".nlpg/runs").join(run_id);
    fs::create_dir_all(&run_dir).unwrap();

    let intent_content = r#"{ "goal": "Output as JSON" }"#;
    fs::write(run_dir.join("intent.json"), intent_content).unwrap();

    let plan_content = r#"{ "title": "JSON Plan", "steps": ["Output JSON"] }"#;
    fs::write(run_dir.join("plan.json"), plan_content).unwrap();

    let patch_content =
        "--- a/file.json\n+++ b/file.json\n@@ -1,1 +1,1 @@\n-{}\n+{\"key\": \"value\"}\n";
    fs::write(run_dir.join("patch.diff"), patch_content).unwrap();

    // 2. Run `a-eye learn --format=json`
    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(repo_root);

    cmd.arg("learn").arg(run_id).arg("--format=json");

    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "a-eye learn --format=json command failed. Stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_output: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(json_output["runId"], run_id);
    assert_eq!(json_output["intent"]["goal"], "Output as JSON");
    assert_eq!(json_output["plan"]["title"], "JSON Plan");
    assert_eq!(json_output["patch"], patch_content);

    // Verify no markdown summary was created
    let summary_path = run_dir.join("learning-summary.md");
    assert!(
        !summary_path.exists(),
        "learning-summary.md should not be created for json format"
    );
}

#[test]
fn aeye_learn_quiet_suppresses_generating_message() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join(".git")).unwrap();

    // 1. Set up a fake run directory
    let run_id = "20240520-140000-QUIET1";
    let run_dir = repo_root.join(".nlpg/runs").join(run_id);
    fs::create_dir_all(&run_dir).unwrap();

    let intent_content = r#"{ "goal": "Be quiet" }"#;
    fs::write(run_dir.join("intent.json"), intent_content).unwrap();

    let plan_content = r#"{ "title": "Quiet Plan", "steps": ["Do nothing"] }"#;
    fs::write(run_dir.join("plan.json"), plan_content).unwrap();

    let patch_content = "--- a/dev/null\n+++ b/dev/null\n";
    fs::write(run_dir.join("patch.diff"), patch_content).unwrap();

    // 2. Run `a-eye learn --quiet`
    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(repo_root);

    cmd.arg("learn").arg(run_id).arg("--quiet");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--- Learning Summary ---"))
        .stdout(predicate::str::contains("Summary saved to:"))
        .stdout(predicate::str::contains("Generating learning summary for run").not());
}
