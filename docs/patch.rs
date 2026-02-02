use crate::aeye::artifacts::Run;
use crate::aeye::patcher;
use crate::aeye::commands::plan::{Intent, Plan};
use crate::aeye::config::{self, AEyeConfig};
use crate::aeye::scanner::SystemProfile;
use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

/// Generates a patch file from a plan.
#[derive(Debug, Parser)]
pub struct PatchCommand {
    /// Path to the plan.json file from a previous run.
    #[clap(long)]
    pub from: PathBuf,
}

pub async fn run(cmd: PatchCommand, config: &AEyeConfig) -> Result<()> {
    if !cmd.from.exists() || cmd.from.file_name().and_then(|s| s.to_str()) != Some("plan.json") {
        anyhow::bail!("--from path must point to a valid plan.json file.");
    }
    let run_path = cmd.from.parent().context("plan.json must be in a run directory")?;
    let repo_root = config::find_repo_root().context("Not in a git repo")?;

    execute_patch_step_from_path(run_path, &repo_root, config).await
}

/// Core logic for the patch step, callable from `a-eye run`.
pub async fn execute_patch_step(run: &Run, repo_root: &Path, config: &AEyeConfig) -> Result<PathBuf> {
    execute_patch_step_from_path(&run.path, repo_root, config).await
}

async fn execute_patch_step_from_path(run_path: &Path, repo_root: &Path, config: &AEyeConfig) -> Result<PathBuf> {
    let plan_path = run_path.join("plan.json");

    println!("Loading plan from: {}", plan_path.display());

    // 2. Load all necessary artifacts from the run directory.
    let plan_content = fs::read_to_string(&plan_path)?;
    let plan: Plan = serde_json::from_str(&plan_content)?;

    let intent_path = run_path.join("intent.json");
    let intent_content = fs::read_to_string(&intent_path)
        .with_context(|| format!("Could not find intent.json in {}", run_path.display()))?;
    let intent: Intent = serde_json::from_str(&intent_content)?;

    let system_path = run_path.join("system.json");
    let system_content = fs::read_to_string(&system_path)
        .with_context(|| format!("Could not find system.json in {}", run_path.display()))?;
    let system_profile: SystemProfile = serde_json::from_str(&system_content)?;

    // 3. Generate the patch content using the patcher module.
    println!("Generating patch...");
    let patch_content = patcher::generate_patch_from_llm(repo_root, config, &intent, &system_profile, &plan).await?;

    // 4. Save the patch to the artifact store.
    let patch_path = run_path.join("patch.diff");
    fs::write(&patch_path, &patch_content)
        .with_context(|| format!("Failed to write patch to {}", patch_path.display()))?;

    // 5. Print summary.
    println!("\n--- Patch Generated ---");
    println!("{}", patch_content);
    println!("-----------------------");
    println!("\nPatch saved to: {}", patch_path.display());
    println!("\nTo apply this patch (requires Tier 2), run:");
    println!("  a-eye apply --from {}", patch_path.display());

    Ok(patch_path)
}
