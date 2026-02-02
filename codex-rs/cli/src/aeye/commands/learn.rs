use crate::aeye::artifacts::Run;
use crate::aeye::artifacts::read_validated_json_artifact;
use crate::aeye::artifacts::write_redacted_text_file;
use crate::aeye::config;
use crate::aeye::learner;
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use serde::Serialize;
use std::fs;

/// Generates a learning summary from a past run.
#[derive(Debug, Parser)]
pub struct LearnCommand {
    /// The ID of the run to learn from.
    #[clap(value_name = "RUN_ID")]
    pub run_id: String,

    /// The output format for the summary.
    #[clap(long, value_enum, default_value = "text")]
    pub format: OutputFormat,

    /// Suppress non-essential output.
    #[clap(long, short)]
    pub quiet: bool,

    /// Force overwrite of existing learning summary.
    #[clap(long)]
    pub force: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LearnOutput {
    run_id: String,
    intent: serde_json::Value,
    plan: serde_json::Value,
    patch: String,
}

pub async fn run(cmd: LearnCommand) -> Result<()> {
    let repo_root = config::find_repo_root()
        .context("Could not find repository root. Are you in a git repository?")?;

    let run_path = repo_root.join(".nlpg/runs").join(&cmd.run_id);
    if !run_path.exists() {
        anyhow::bail!(
            "Run with ID '{}' not found at {}",
            cmd.run_id,
            run_path.display()
        );
    }

    let summary_path = run_path.join("learning-summary.md");
    if summary_path.exists() && !cmd.force {
        anyhow::bail!(
            "Learning summary already exists at {}. Use --force to overwrite.",
            summary_path.display()
        );
    }

    let run = Run {
        id: cmd.run_id.clone(),
        path: run_path.clone(),
    };

    let intent_path = run_path.join("intent.json");
    let intent_content = read_validated_json_artifact(&intent_path, "intent.json")
        .with_context(|| format!("Failed to read intent.json from run '{}'", cmd.run_id))?;
    let plan_path = run_path.join("plan.json");
    let plan_content = read_validated_json_artifact(&plan_path, "plan.json")
        .with_context(|| format!("Failed to read plan.json from run '{}'", cmd.run_id))?;
    let patch_content = fs::read_to_string(run_path.join("patch.diff"))
        .with_context(|| format!("Failed to read patch.diff from run '{}'", cmd.run_id))?;

    match cmd.format {
        OutputFormat::Text => {
            if !cmd.quiet {
                eprintln!("Generating learning summary for run: {}", run.id);
            }

            let summary =
                learner::generate_summary(&intent_content, &plan_content, &patch_content).await?;
            write_redacted_text_file(&summary_path, &summary)?;

            println!("\n--- Learning Summary ---");
            println!("{summary}");
            println!("----------------------");
            println!("\nSummary saved to: {}", summary_path.display());
        }
        OutputFormat::Json => {
            let output = LearnOutput {
                run_id: cmd.run_id,
                intent: serde_json::from_str(&intent_content)?,
                plan: serde_json::from_str(&plan_content)?,
                patch: patch_content,
            };
            let json_output = serde_json::to_string_pretty(&output)?;
            println!("{json_output}");
        }
    }

    Ok(())
}
