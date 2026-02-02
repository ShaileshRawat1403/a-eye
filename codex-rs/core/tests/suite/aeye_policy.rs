use codex_cli::aeye::config::AEyeConfig;
use codex_cli::aeye::policy::PolicyEngine;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use tempfile::tempdir;

// Helper to create a mock repo structure
fn setup_test_repo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    let repo_root = dir.path().to_path_buf();
    fs::create_dir(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join("src/components")).unwrap();
    fs::create_dir_all(repo_root.join("docs")).unwrap();
    fs::write(repo_root.join("src/components/button.js"), "").unwrap();
    fs::write(repo_root.join("docs/guide.md"), "").unwrap();
    fs::write(repo_root.join("package-lock.json"), "").unwrap();
    fs::write(repo_root.join("credentials.json"), "").unwrap();
    let mut gitignore = fs::File::create(repo_root.join(".gitignore")).unwrap();
    writeln!(gitignore, "credentials.json").unwrap();
    writeln!(gitignore, "*.lock").unwrap();
    drop(gitignore);

    fs::write(repo_root.join("README.md"), "").unwrap();
    (dir, repo_root)
}

#[test]
fn test_policy_engine_check_write() {
    let (_temp_dir, repo_root) = setup_test_repo();
    let original_cwd = std::env::current_dir().unwrap();
    // Change CWD to be inside the repo to test relative paths correctly
    std::env::set_current_dir(repo_root.join("src")).unwrap();

    let config = AEyeConfig {
        deny_globs: vec!["*.lock".to_string(), "credentials.json".to_string()],
        write_allowlist: vec!["src/**/*.js".to_string(), "docs/*.md".to_string()],
        ..Default::default()
    };

    let policy_engine = PolicyEngine::new(config, Some(repo_root.clone()));

    // --- Denied by deny_globs ---
    assert!(
        !policy_engine.check_write(&repo_root.join("package-lock.json")),
        "Should deny .lock file"
    );
    assert!(
        !policy_engine.check_write(&repo_root.join("credentials.json")),
        "Should deny credentials.json"
    );

    // --- Allowed by write_allowlist ---
    assert!(
        policy_engine.check_write(&repo_root.join("src/components/button.js")),
        "Should allow nested .js file in src"
    );
    assert!(
        policy_engine.check_write(&repo_root.join("docs/guide.md")),
        "Should allow .md file in docs"
    );

    // --- Denied because not in write_allowlist (strict mode) ---
    assert!(
        !policy_engine.check_write(&repo_root.join("README.md")),
        "Should deny file not in allowlist"
    );

    // --- Test relative paths from CWD = <repo_root>/src ---
    assert!(
        policy_engine.check_write(Path::new("components/button.js")),
        "Should allow relative path from within src"
    );
    assert!(
        !policy_engine.check_write(Path::new("../README.md")),
        "Should deny relative path to file not in allowlist"
    );
    assert!(
        policy_engine.check_write(Path::new("../docs/guide.md")),
        "Should allow relative path to file in allowlist"
    );

    // --- Test path outside of repo ---
    let another_dir = tempdir().unwrap();
    assert!(
        !policy_engine.check_write(another_dir.path().join("some_file.txt")),
        "Should deny path outside of repo"
    );

    // Restore original CWD
    std::env::set_current_dir(original_cwd).unwrap();
}

#[test]
fn test_policy_engine_check_write_permissive() {
    let (_temp_dir, repo_root) = setup_test_repo();
    std::env::set_current_dir(&repo_root).unwrap();

    let config = AEyeConfig {
        deny_globs: vec!["*.lock".to_string()],
        write_allowlist: vec![], // Empty allowlist enables permissive mode
        ..Default::default()
    };

    let policy_engine = PolicyEngine::new(config, Some(repo_root.clone()));

    // Denied by deny_globs
    assert!(
        !policy_engine.check_write(Path::new("package-lock.json")),
        "Should deny .lock file in permissive mode"
    );

    // Allowed because not in deny_globs and allowlist is empty
    assert!(
        policy_engine.check_write(Path::new("src/components/button.js")),
        "Should allow any non-denied file in permissive mode"
    );
    assert!(
        policy_engine.check_write(Path::new("README.md")),
        "Should allow README.md in permissive mode"
    );
}

#[test]
fn test_policy_engine_check_shell() {
    let config = AEyeConfig {
        shell_deny_patterns: vec![
            r"rm -rf".to_string(),
            r"git push --force".to_string(),
            r"^sudo".to_string(),
            r":\s*&".to_string(), // Powershell background operator
        ],
        ..Default::default()
    };

    let policy_engine = PolicyEngine::new(config, None); // repo_root not needed

    // --- Denied commands ---
    assert!(!policy_engine.check_shell("rm -rf /tmp/foo"));
    assert!(!policy_engine.check_shell("git push --force origin main"));
    assert!(!policy_engine.check_shell("sudo apt-get update"));
    assert!(!policy_engine.check_shell("cat /etc/passwd; & ls"));

    // --- Allowed commands ---
    assert!(policy_engine.check_shell("ls -la"));
    assert!(policy_engine.check_shell("git commit -m 'test'"));
    assert!(policy_engine.check_shell("echo 'rm -rf'")); // not a match
    assert!(policy_engine.check_shell("npm install"));
    assert!(policy_engine.check_shell("cat file.txt"));
}
