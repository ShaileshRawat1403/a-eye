use crate::aeye::config;
use crate::aeye::policy::PolicyEngine;
use anyhow::{Context, Result};
use clap::Parser;
use owo_colors::OwoColorize;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Applies a patch after user approval (Tier 2+).
#[derive(Debug, Parser)]
pub struct ApplyCommand {
    /// Path to the patch.diff file from a previous run.
    #[clap(long)]
    pub from: PathBuf,

    /// Perform a dry run without applying the patch or creating a branch.
    #[clap(long)]
    pub dry_run: bool,
}

pub async fn run(cmd: ApplyCommand, policy_engine: &PolicyEngine) -> Result<()> {
    // 1. Check Tier
    if !policy_engine.check_tier(2) {
        anyhow::bail!(
            "`a-eye apply` requires Tier 2 or higher. Your current tier is {}. You can change this in a-eye.yaml or with an override.",
            policy_engine.current_tier()
        );
    }

    // 2. Load patch file and find repo root
    let patch_content = fs::read_to_string(&cmd.from)
        .with_context(|| format!("Failed to read patch file: {}", cmd.from.display()))?;
    let repo_root = config::find_repo_root()
        .context("`a-eye apply` must be run from within a git repository.")?;

    // 3. Check write permissions for all files in the patch
    let files_in_patch = parse_files_from_diff(&patch_content)?;
    for file in &files_in_patch {
        if !policy_engine.check_write(file) {
            anyhow::bail!(
                "Policy violation: Attempting to write to '{}', which is not in the `write_allowlist` of your a-eye.yaml.",
                file.display()
            );
        }
    }

    // 4. Show diff summary and ask for approval
    println!("--- Patch to be Applied ---");
    for line in patch_content.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            println!("{}", line.green());
        } else if line.starts_with('-') && !line.starts_with("---") {
            println!("{}", line.red());
        } else if line.starts_with("@@") {
            println!("{}", line.cyan());
        } else {
            println!("{}", line);
        }
    }
    println!("-------------------------");

    // NEW: Handle dry-run mode
    if cmd.dry_run {
        println!("\n-- DRY RUN MODE --");
        println!("The following actions would be taken:");

        let run_path = cmd.from.parent().context("Patch file must be in a run directory")?;
        let run_id = run_path.file_name().unwrap_or_default().to_string_lossy();
        let branch_name = format!("a-eye/patch-{}", run_id);

        if policy_engine.config.require_branch_for_apply {
            println!("- Create new git branch: {}", branch_name);
        }
        println!("- Apply patch to the following files:");
        for file in &files_in_patch {
            println!("  - {}", file.display());
        }
        println!("\nNo changes were made.");
        return Ok(());
    }

    if !prompt_for_approval("Apply this patch?")? {
        println!("Apply operation cancelled by user.");
        return Ok(());
    }

    // 5. Create git branch
    let run_path = cmd.from.parent().context("Patch file must be in a run directory")?;
    let run_id = run_path.file_name().unwrap_or_default().to_string_lossy();
    let branch_name = format!("a-eye/patch-{}", run_id);

    if policy_engine.config.require_branch_for_apply {
        println!("Creating new branch: {}", branch_name);
        let status = std::process::Command::new("git")
            .current_dir(&repo_root)
            .args(["checkout", "-b", &branch_name])
            .status()?;
        if !status.success() {
            anyhow::bail!("Failed to create git branch '{}'. Does it already exist? You can disable automatic branch creation with `require_branch_for_apply: false` in a-eye.yaml.", branch_name);
        }
    }

    // 6. Apply patch
    let mut child = std::process::Command::new("git")
        .current_dir(&repo_root)
        .arg("apply")
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().context("Failed to open stdin for git apply")?;
    stdin.write_all(patch_content.as_bytes())?;
    drop(stdin);

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("`git apply` failed. The patch may be invalid or have conflicts. Try applying it manually from the artifact directory.");
    }

    println!("\n✅ Patch applied successfully on branch '{}'.", branch_name);
    println!("Run `git diff` to review the changes.");

    Ok(())
}

fn prompt_for_approval(prompt_text: &str) -> Result<bool> {
    print!("{} [y/N]: ", prompt_text);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().eq_ignore_ascii_case("y"))
}

fn parse_files_from_diff(diff: &str) -> Result<Vec<PathBuf>> {
    let mut files = std::collections::HashSet::new();
    let re = regex_lite::Regex::new(r"^\+\+\+ b/(.+)").unwrap();
    for line in diff.lines() {
        if let Some(caps) = re.captures(line) {
            if let Some(path_str) = caps.get(1) {
                files.insert(PathBuf::from(path_str.as_str().trim()));
            }
        }
    }
    Ok(files.into_iter().collect())
}
