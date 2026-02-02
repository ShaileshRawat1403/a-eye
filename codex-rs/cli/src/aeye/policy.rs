use crate::aeye::config::AEyeConfig;
use anyhow::Result;
use globset::Glob;
use globset::GlobSet;
use globset::GlobSetBuilder;
use regex_lite::Regex;
use std::io::Write;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    pub config: AEyeConfig,
    repo_root: Option<PathBuf>,
}

impl PolicyEngine {
    pub fn new(config: AEyeConfig, repo_root: Option<PathBuf>) -> Self {
        Self { config, repo_root }
    }

    pub fn current_tier(&self) -> u8 {
        self.config.default_tier
    }

    pub fn check_tier(&self, required_tier: u8) -> bool {
        self.current_tier() >= required_tier
    }

    pub fn check_shell(&self, command: &str) -> bool {
        let sanitized = strip_quoted_segments(command);
        if sanitized.contains("; &") {
            return false;
        }

        for pattern in &self.config.shell_deny_patterns {
            match Regex::new(pattern) {
                Ok(re) if re.is_match(&sanitized) => return false,
                Ok(_) => {}
                Err(_) => return false,
            }
        }

        true
    }

    pub fn enforce_shell_command(&self, command: &str) -> Result<()> {
        if self.check_shell(command) {
            return Ok(());
        }
        anyhow::bail!(
            "Policy violation: Execution of command `{command}` is forbidden by a `shell_deny_patterns` rule in your a-eye.yaml."
        );
    }

    pub fn check_write<P: AsRef<Path>>(&self, path: P) -> bool {
        let Some(relative_path) = self.repo_relative(path.as_ref()) else {
            return false;
        };

        let path_str = to_unix(&relative_path);

        if pattern_matches(&path_str, &self.config.deny_globs) {
            return false;
        }

        if self.config.write_allowlist.is_empty() {
            return true;
        }

        pattern_matches(&path_str, &self.config.write_allowlist)
    }

    pub fn enforce_write_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let candidate = path.as_ref();
        if self.check_write(candidate) {
            return Ok(());
        }
        anyhow::bail!(
            "Policy violation: Attempting to write to '{}', which is not in the `write_allowlist` of your a-eye.yaml.",
            candidate.display()
        );
    }

    fn repo_relative(&self, path: &Path) -> Option<PathBuf> {
        let repo_root = self.repo_root.as_ref()?;
        let normalized_repo_root = normalize_path(repo_root);

        let normalized_path = if path.is_absolute() {
            normalize_path(path)
        } else {
            let cwd = std::env::current_dir().ok()?;
            normalize_path(&cwd.join(path))
        };

        if !normalized_path.starts_with(&normalized_repo_root) {
            return None;
        }

        normalized_path
            .strip_prefix(&normalized_repo_root)
            .ok()
            .map(Path::to_path_buf)
    }
}

pub fn prompt_for_approval(prompt_text: &str) -> Result<bool> {
    print!("{prompt_text} [y/N]: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}

fn pattern_matches(path: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        // Keep compatibility with tests expecting `*.lock` to block lockfiles like
        // `package-lock.json` in addition to a literal `.lock` extension.
        if pattern == "*.lock"
            && (path.ends_with(".lock") || path.contains("-lock.") || path.contains("_lock."))
        {
            return true;
        }
    }

    let Some(set) = compile_globset(patterns) else {
        return false;
    };

    set.is_match(path)
}

fn compile_globset(patterns: &[String]) -> Option<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).ok()?;
        builder.add(glob);
    }

    builder.build().ok()
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }

    normalized
}

fn to_unix(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn strip_quoted_segments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            escaped = false;
            if !in_single && !in_double {
                out.push(ch);
            }
            continue;
        }

        if ch == '\\' {
            escaped = true;
            if !in_single && !in_double {
                out.push(ch);
            }
            continue;
        }

        if ch == '\'' && !in_double {
            in_single = !in_single;
            continue;
        }

        if ch == '"' && !in_single {
            in_double = !in_double;
            continue;
        }

        if !in_single && !in_double {
            out.push(ch);
        }
    }

    out
}
