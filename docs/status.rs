use clap::Parser;
use crate::aeye::config::AEyeConfig;
use std::path::PathBuf;

/// Shows current A-Eye status, policy mode, last run ID, and system profile presence.
#[derive(Debug, Parser)]
pub struct StatusCommand {}

pub async fn run(_cmd: StatusCommand, config: AEyeConfig, repo_root: Option<PathBuf>) -> anyhow::Result<()> {
    let profile_path_str = ".nlpg/system.json";
    let profile_exists = repo_root.map_or(false, |root| root.join(profile_path_str).exists());
    let profile_status = if profile_exists { "Found" } else { "Not found" };

    println!("A-Eye Status:");
    println!("  Default Tier: {} (from a-eye.yaml)", config.default_tier);
    let policy_mode = match config.default_tier {
        0 => "Tier 0 (Explain-Only)",
        1 => "Tier 1 (Plan & Diff)",
        _ => "Tier 2+ (Supervised Execution)",
    };
    println!("  Policy Mode: {}", policy_mode);
    println!("  Last Run ID: N/A (no runs yet)");
    println!("  System Profile: {} ({})", profile_status, profile_path_str);
    println!("  Write Allowlist entries: {}", config.write_allowlist.len());

    Ok(())
}
