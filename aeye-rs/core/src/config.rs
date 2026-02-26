use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AEyeConfig {
    #[serde(default)]
    pub deny_globs: Vec<String>,
    #[serde(default)]
    pub write_allowlist: Vec<String>,
    #[serde(default)]
    pub shell_deny_patterns: Vec<String>,
}

impl Default for AEyeConfig {
    fn default() -> Self {
        Self {
            deny_globs: Vec::new(),
            write_allowlist: Vec::new(),
            shell_deny_patterns: Vec::new(),
        }
    }
}
