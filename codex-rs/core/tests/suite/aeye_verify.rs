use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

fn setup_git_repo() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let repo_root = dir.path().to_path_buf();
    Command::new("git")
        .current_dir(&repo_root)
        .arg("init")
        .status()
        .unwrap();
    (dir, repo_root)
}

#[test]
fn aeye_verify_prints_commands_in_tier_1() {
    let (_dir, repo_root) = setup_git_repo();
    fs::write(repo_root.join("a-eye.yaml"), "default_tier: 1").unwrap();
    fs::create_dir(repo_root.join(".nlpg")).unwrap();
    fs::write(
        repo_root.join(".nlpg/system.json"),
        r#"{ "verifyCommands": ["npm test", "npm run lint"] }"#,
    )
    .unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(&repo_root);

    cmd.arg("verify");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Running in read-only mode"))
        .stdout(predicate::str::contains("- npm test"))
        .stdout(predicate::str::contains("- npm run lint"));
}

#[test]
fn aeye_verify_executes_commands_in_tier_2_with_approval() {
    let (_dir, repo_root) = setup_git_repo();
    fs::write(repo_root.join("a-eye.yaml"), "default_tier: 2").unwrap();
    fs::create_dir(repo_root.join(".nlpg")).unwrap();
    // Use `echo` which is safe and available on all systems for testing.
    let test_file = repo_root.join("verify_output.txt");
    let command_to_run = format!("echo 'verified' > {}", test_file.display());
    let system_profile = format!(r#"{{ "verifyCommands": ["{}"] }}"#, command_to_run);
    fs::write(repo_root.join(".nlpg/system.json"), system_profile).unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(&repo_root);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.arg("verify");

    let mut child = cmd.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    // Approve the prompt
    std::thread::spawn(move || {
        stdin.write_all(b"y\n").unwrap();
    });

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "Stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All verification commands passed successfully."));

    // Check that the command actually ran
    let content = fs::read_to_string(test_file).unwrap();
    assert_eq!(content.trim(), "verified");
}

#[test]
fn aeye_verify_is_cancelled_by_user() {
    let (_dir, repo_root) = setup_git_repo();
    fs::write(repo_root.join("a-eye.yaml"), "default_tier: 2").unwrap();
    fs::create_dir(repo_root.join(".nlpg")).unwrap();
    fs::write(
        repo_root.join(".nlpg/system.json"),
        r#"{ "verifyCommands": ["echo 'should not run'"] }"#,
    )
    .unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(&repo_root);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.arg("verify");

    let mut child = cmd.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    // Deny the prompt
    std::thread::spawn(move || {
        stdin.write_all(b"n\n").unwrap();
    });

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Verification cancelled by user."));
}

#[test]
fn aeye_verify_is_blocked_by_denylist() {
    let (_dir, repo_root) = setup_git_repo();
    fs::write(
        repo_root.join("a-eye.yaml"),
        "default_tier: 2\nshell_deny_patterns:\n  - 'rm -rf'",
    )
    .unwrap();
    fs::create_dir(repo_root.join(".nlpg")).unwrap();
    fs::write(
        repo_root.join(".nlpg/system.json"),
        r#"{ "verifyCommands": ["rm -rf /tmp/forbidden"] }"#,
    )
    .unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(&repo_root);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.arg("verify");

    let mut child = cmd.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    // Approve the prompt
    std::thread::spawn(move || {
        stdin.write_all(b"y\n").unwrap();
    });

    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Policy violation"));
    assert!(stderr.contains("rm -rf /tmp/forbidden"));
}
