use crate::aeye::config;
use crate::aeye::policy::PolicyEngine;
use crate::aeye::workflows::WorkflowEngine;
use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;

/// Runs a deterministic workflow recipe.
#[derive(Debug, Parser)]
pub struct RunCommand {
    /// The name of the recipe to run (e.g., `safe_patch`).
    #[clap(value_name = "RECIPE")]
    pub recipe: String,

    /// The natural language goal for the workflow.
    #[clap(long)]
    pub goal: Option<String>,

    /// Path to a log file to use as input for recipes like `fix_from_logs`.
    #[clap(long)]
    pub log_file: Option<PathBuf>,
}

pub async fn run(cmd: RunCommand, policy_engine: &PolicyEngine) -> Result<()> {
    let repo_root = config::find_repo_root()
        .context("Could not find repository root. Are you in a git repository?")?;

    let engine = WorkflowEngine::new(repo_root, policy_engine);

    let goal = if cmd.recipe == "fix_from_logs" {
        let log_path = cmd.log_file.context("The `fix_from_logs` recipe requires a --log-file argument.")?;
        let log_content = fs::read_to_string(&log_path)?;
        let truncated_log = log_content.chars().take(1000).collect::<String>();
        Some(format!("Fix the error found in the following logs from {}: \n\n{}", log_path.display(), truncated_log))
    } else {
        cmd.goal
    };

    engine.run(&cmd.recipe, goal).await
}
