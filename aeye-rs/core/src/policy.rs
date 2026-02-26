use crate::config::AEyeConfig;
use std::path::{Path, PathBuf};
use wildmatch::WildMatch;

pub struct PolicyEngine {
    config: AEyeConfig,
    repo_root: Option<PathBuf>,
}

impl PolicyEngine {
    pub fn new(config: AEyeConfig, repo_root: Option<PathBuf>) -> Self {
        Self { config, repo_root }
    }

    pub fn check_write(&self, path: &Path) -> bool {
        let path = self.resolve_path(path);
        if !self.is_within_repo(&path) {
            return false;
        }

        let path_str = path.to_string_lossy();

        for glob in &self.config.deny_globs {
            if WildMatch::new(glob).matches(&path_str) {
                return false;
            }
        }

        if self.config.write_allowlist.is_empty() {
            return true;
        }

        for glob in &self.config.write_allowlist {
            if WildMatch::new(glob).matches(&path_str) {
                return true;
            }
        }

        false
    }

    pub fn check_shell(&self, command: &str) -> bool {
        for pattern in &self.config.shell_deny_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(command) {
                    return false;
                }
            }
        }
        true
    }

    fn resolve_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            return path.to_path_buf();
        }

        if let Some(ref repo_root) = self.repo_root {
            repo_root.join(path)
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        }
    }

    fn is_within_repo(&self, path: &Path) -> bool {
        if let Some(ref repo_root) = self.repo_root {
            path.starts_with(repo_root)
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    fn setup_test_repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
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
    fn test_policy_engine_shell_deny() {
        let config = AEyeConfig {
            deny_globs: vec![],
            write_allowlist: vec![],
            shell_deny_patterns: vec![r"rm\s+-rf".to_string()],
        };

        let policy_engine = PolicyEngine::new(config, None);

        assert!(!policy_engine.check_shell("rm -rf /tmp/foo"));
        assert!(policy_engine.check_shell("ls -la"));
    }

    #[test]
    fn test_policy_engine_allows_by_default() {
        let config = AEyeConfig::default();
        let policy_engine = PolicyEngine::new(config, None);

        assert!(policy_engine.check_write(Path::new("any/file.txt")));
        assert!(policy_engine.check_shell("any command"));
    }

    #[test]
    fn test_policy_engine_check_shell_deny() {
        let config = AEyeConfig {
            deny_globs: vec![],
            write_allowlist: vec![],
            shell_deny_patterns: vec![r"rm\s+-rf".to_string()],
        };

        let policy_engine = PolicyEngine::new(config, None);

        assert!(!policy_engine.check_shell("rm -rf /tmp/foo"));
        assert!(policy_engine.check_shell("ls -la"));
    }
}
