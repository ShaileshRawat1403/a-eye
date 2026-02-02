use crate::aeye::artifacts;
use crate::aeye::artifacts::Run;
use crate::aeye::commands::plan::Intent;
use crate::aeye::commands::plan::Plan;
use crate::aeye::config;
use crate::aeye::patcher;
use crate::aeye::scanner::SystemProfile;
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use std::path::Path;
use std::path::PathBuf;

/// Generates a patch file from a plan.
#[derive(Debug, Parser)]
pub struct PatchCommand {
    /// Path to the plan.json file from a previous run.
    #[clap(long)]
    pub from: PathBuf,
}

pub async fn run(cmd: PatchCommand) -> Result<()> {
    if !cmd.from.exists() || cmd.from.file_name().and_then(|v| v.to_str()) != Some("plan.json") {
        anyhow::bail!("--from path must point to a valid plan.json file.");
    }

    let run_path = cmd
        .from
        .parent()
        .context("plan.json must be in a run directory")?;
    let repo_root = config::find_repo_root()
        .or_else(|| std::env::current_dir().ok())
        .context("Could not determine repository root")?;

    execute_patch_step_from_path(run_path, &repo_root).await?;
    Ok(())
}

pub async fn execute_patch_step(run: &Run, repo_root: &Path) -> Result<PathBuf> {
    execute_patch_step_from_path(&run.path, repo_root).await
}

async fn execute_patch_step_from_path(run_path: &Path, repo_root: &Path) -> Result<PathBuf> {
    let plan_path = run_path.join("plan.json");
    println!("Loading plan from: {}", plan_path.display());

    let plan_content = artifacts::read_validated_json_artifact(&plan_path, "plan.json")?;
    let plan: Plan = serde_json::from_str(&plan_content)?;

    let intent_path = run_path.join("intent.json");
    let intent_content = artifacts::read_validated_json_artifact(&intent_path, "intent.json")
        .with_context(|| format!("Could not find intent.json in {}", run_path.display()))?;
    let intent: Intent = serde_json::from_str(&intent_content)?;

    let system_path = run_path.join("system.json");
    let system_content = artifacts::read_validated_json_artifact(&system_path, "system.json")
        .with_context(|| format!("Could not find system.json in {}", run_path.display()))?;
    let system_profile: SystemProfile = serde_json::from_str(&system_content)?;

    println!("Generating patch...");
    let patch_content =
        patcher::generate_patch_from_llm(repo_root, &intent, &system_profile, &plan).await?;

    let patch_path = run_path.join("patch.diff");
    artifacts::write_redacted_text_file(&patch_path, &patch_content)
        .with_context(|| format!("Failed to write patch to {}", patch_path.display()))?;

    println!("\n--- Patch Generated ---");
    println!("{patch_content}");
    println!("-----------------------");
    println!("\nPatch saved to: {}", patch_path.display());
    println!("\nTo apply this patch (requires Tier 2), run:");
    println!("  a-eye apply --from {}", patch_path.display());

    Ok(patch_path)
}
