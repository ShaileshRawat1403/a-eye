use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn aeye_explain_creates_artifact_and_prints_summary() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    // Create a mock git repo and a file to explain
    fs::create_dir(repo_root.join(".git")).unwrap();
    let file_to_explain = repo_root.join("src.js");
    fs::write(&file_to_explain, "console.log('hello');").unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(repo_root);

    cmd.arg("explain")
        .arg(file_to_explain.to_str().unwrap())
        .arg("--line")
        .arg("1");

    let output = cmd.output().unwrap();
    assert!(output.status.success(), "a-eye explain command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--- Explanation ---"));
    assert!(stdout.contains("Construct Type: Function Call"));
    assert!(stdout.contains("Intent:"));
    assert!(stdout.contains("Safest Edits:"));

    // Find the run directory
    let runs_dir = repo_root.join(".nlpg/runs");
    let run_entry = fs::read_dir(runs_dir)
        .unwrap()
        .next()
        .expect("No run directory created")
        .unwrap();
    let run_path = run_entry.path();

    let explain_artifact_path = run_path.join("explain.json");
    assert!(
        explain_artifact_path.exists(),
        "explain.json artifact should be created"
    );

    let content = fs::read_to_string(explain_artifact_path).unwrap();
    let artifact: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(artifact["constructType"], "Function Call");
    assert!(
        artifact["intent"]
            .as_str()
            .unwrap()
            .contains("calling a function")
    );
}
