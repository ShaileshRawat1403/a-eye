use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn aeye_scan_creates_system_profile() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    fs::create_dir(repo_root.join(".git")).unwrap();
    fs::write(
        repo_root.join("package.json"),
        r#"{ "name": "test-project", "scripts": { "test": "jest" } }"#,
    )
    .unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(repo_root);

    cmd.arg("scan");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scan complete."))
        .stdout(predicate::str::contains("Languages: javascript"))
        .stdout(predicate::str::contains("Package Manager: npm"))
        .stdout(predicate::str::contains("Suggested Verify Commands:"))
        .stdout(predicate::str::contains("- npm test"));

    let profile_path = repo_root.join(".nlpg/system.json");
    assert!(profile_path.exists(), "system.json should be created");

    let content = fs::read_to_string(profile_path).unwrap();
    let profile: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(profile["packageManager"], "npm");
    assert_eq!(profile["verifyCommands"][0], "npm test");
}
