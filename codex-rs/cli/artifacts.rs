use anyhow::{Context, Result};
use chrono::Utc;
use rand::distributions::{Alphanumeric, DistString};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const RUNS_DIR: &str = "runs";
const NLPG_DIR: &str = ".nlpg";

pub struct Run {
    pub id: String,
    pub path: PathBuf,
}

impl Run {
    /// Creates a new run, which includes creating the necessary directory structure.
    pub fn new(repo_root: &Path) -> Result<Self> {
        let run_id = format!(
            "{}-{}",
            Utc::now().format("%Y%m%d-%H%M%S"),
            Alphanumeric.sample_string(&mut rand::thread_rng(), 6)
        );

        let run_path = repo_root.join(NLPG_DIR).join(RUNS_DIR).join(&run_id);
        fs::create_dir_all(&run_path)
            .with_context(|| format!("Failed to create run directory at {}", run_path.display()))?;

        // Create subdirectories
        fs::create_dir(run_path.join("steps"))?;
        fs::create_dir(run_path.join("logs"))?;

        Ok(Self {
            id: run_id,
            path: run_path,
        })
    }

    /// Persists a serializable artifact to the run directory.
    pub fn write_artifact<T: Serialize>(&self, name: &str, artifact: &T) -> Result<()> {
        let artifact_path = self.path.join(name);
        let content = serde_json::to_string_pretty(artifact)?;
        fs::write(&artifact_path, content)
            .with_context(|| format!("Failed to write artifact to {}", artifact_path.display()))?;
        Ok(())
    }

    /// Persists a raw string to a file in the `logs` subdirectory of the run.
    pub fn write_log_file(&self, name: &str, content: &str) -> Result<()> {
        let log_path = self.path.join("logs").join(name);
        fs::write(&log_path, content)
            .with_context(|| format!("Failed to write log file to {}", log_path.display()))?;
        Ok(())
    }
}
