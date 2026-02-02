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
    Command::new("git")
        .current_dir(&repo_root)
        .args(["config", "user.name", "Test"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&repo_root)
        .args(["config", "user.email", "test@example.com"])
        .status()
        .unwrap();
    (dir, repo_root)
}

#[test]
fn aeye_apply_fails_in_tier_1() {
    let (_dir, repo_root) = setup_git_repo();
    fs::write(repo_root.join("a-eye.yaml"), "default_tier: 1").unwrap();
    let patch_path = repo_root.join("patch.diff");
    fs::write(&patch_path, "dummy patch").unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(&repo_root);

    cmd.arg("apply").arg("--from").arg(&patch_path);

    cmd.assert().failure().stderr(predicate::str::contains(
        "`a-eye apply` requires Tier 2 or higher",
    ));
}

#[test]
fn aeye_apply_succeeds_in_tier_2_with_approval() {
    let (_dir, repo_root) = setup_git_repo();
    fs::write(repo_root.join("a-eye.yaml"), "default_tier: 2").unwrap();
    fs::write(repo_root.join("file.txt"), "original content\n").unwrap();
    Command::new("git")
        .current_dir(&repo_root)
        .args(["add", "file.txt"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&repo_root)
        .args(["commit", "-m", "initial"])
        .status()
        .unwrap();

    let run_dir = repo_root.join(".nlpg/runs/test-run-123");
    fs::create_dir_all(&run_dir).unwrap();
    let patch_path = run_dir.join("patch.diff");
    let patch_content =
        "--- a/file.txt\n+++ b/file.txt\n@@ -1 +1 @@\n-original content\n+new content\n";
    fs::write(&patch_path, patch_content).unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(&repo_root);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.arg("apply").arg("--from").arg(&patch_path);

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
    assert!(stdout.contains("Patch applied successfully"));

    // Check file content
    let new_content = fs::read_to_string(repo_root.join("file.txt")).unwrap();
    assert_eq!(new_content, "new content\n");

    // Check git branch
    let branch_output = Command::new("git")
        .current_dir(&repo_root)
        .arg("branch")
        .arg("--show-current")
        .output()
        .unwrap();
    let current_branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();
    assert_eq!(current_branch, "a-eye/patch-test-run-123");
}

#[test]
fn aeye_apply_fails_on_write_outside_allowlist() {
    let (_dir, repo_root) = setup_git_repo();
    fs::write(
        repo_root.join("a-eye.yaml"),
        "default_tier: 2\nwrite_allowlist:\n  - 'src/**'",
    )
    .unwrap();
    let patch_path = repo_root.join("patch.diff");
    let patch_content = "--- a/README.md\n+++ b/README.md\n@@ -0,0 +1 @@\n+forbidden change\n";
    fs::write(&patch_path, patch_content).unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(&repo_root);

    cmd.arg("apply").arg("--from").arg(&patch_path);

    cmd.assert().failure().stderr(predicate::str::contains(
        "Policy violation: Attempting to write to 'README.md'",
    ));
}

#[test]
fn aeye_apply_dry_run_shows_plan_and_makes_no_changes() {
    let (_dir, repo_root) = setup_git_repo();
    fs::write(repo_root.join("a-eye.yaml"), "default_tier: 2").unwrap();
    let original_content = "original content\n";
    fs::write(repo_root.join("file.txt"), original_content).unwrap();
    Command::new("git")
        .current_dir(&repo_root)
        .args(["add", "file.txt"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&repo_root)
        .args(["commit", "-m", "initial"])
        .status()
        .unwrap();

    let run_dir = repo_root.join(".nlpg/runs/test-dry-run");
    fs::create_dir_all(&run_dir).unwrap();
    let patch_path = run_dir.join("patch.diff");
    let patch_content =
        "--- a/file.txt\n+++ b/file.txt\n@@ -1 +1 @@\n-original content\n+new content\n";
    fs::write(&patch_path, patch_content).unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(&repo_root);
    cmd.arg("apply")
        .arg("--from")
        .arg(&patch_path)
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("-- DRY RUN MODE --"))
        .stdout(predicate::str::contains(
            "Create new git branch: a-eye/patch-test-dry-run",
        ))
        .stdout(predicate::str::contains(
            "Apply patch to the following files:",
        ))
        .stdout(predicate::str::contains("- file.txt"))
        .stdout(predicate::str::contains("No changes were made."));

    // Verify no changes were made
    let current_content = fs::read_to_string(repo_root.join("file.txt")).unwrap();
    assert_eq!(current_content, original_content);

    // Verify no new branch was created
    let branch_output = Command::new("git")
        .current_dir(&repo_root)
        .arg("branch")
        .output()
        .unwrap();
    let branches = String::from_utf8_lossy(&branch_output.stdout);
    assert!(!branches.contains("a-eye/patch-test-dry-run"));
}
