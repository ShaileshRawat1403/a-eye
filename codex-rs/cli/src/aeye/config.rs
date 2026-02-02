use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

const CONFIG_FILE: &str = "a-eye.yaml";
const LEGACY_CONFIG_FILE: &str = "aeye.yaml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AEyeConfig {
    pub default_tier: u8,
    pub write_allowlist: Vec<String>,
    pub read_allowlist: Vec<String>,
    pub deny_globs: Vec<String>,
    pub shell_deny_patterns: Vec<String>,
    pub require_branch_for_apply: bool,
}

impl Default for AEyeConfig {
    fn default() -> Self {
        Self {
            default_tier: 1,
            write_allowlist: Vec::new(),
            read_allowlist: Vec::new(),
            deny_globs: Vec::new(),
            shell_deny_patterns: vec![
                "rm -rf".to_string(),
                "git push --force".to_string(),
                "^sudo".to_string(),
            ],
            require_branch_for_apply: true,
        }
    }
}

pub fn find_repo_root() -> Option<PathBuf> {
    let start = std::env::current_dir().ok()?;
    for path in start.as_path().ancestors() {
        if path.join(".git").exists() {
            return Some(path.to_path_buf());
        }
    }
    None
}

pub fn load_config() -> Result<(AEyeConfig, Option<PathBuf>)> {
    let repo_root = find_repo_root();
    let config_path = repo_root.as_ref().map_or_else(
        || {
            if PathBuf::from(CONFIG_FILE).exists() {
                PathBuf::from(CONFIG_FILE)
            } else {
                PathBuf::from(LEGACY_CONFIG_FILE)
            }
        },
        |root| {
            let primary = root.join(CONFIG_FILE);
            if primary.exists() {
                primary
            } else {
                root.join(LEGACY_CONFIG_FILE)
            }
        },
    );

    if !config_path.exists() {
        return Ok((AEyeConfig::default(), repo_root));
    }

    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    let config: AEyeConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", config_path.display()))?;

    Ok((config, repo_root))
}
