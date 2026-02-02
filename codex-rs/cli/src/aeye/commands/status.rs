use crate::aeye::config::AEyeConfig;
use clap::Parser;
use std::path::PathBuf;

/// Shows current A-Eye status and policy mode.
#[derive(Debug, Parser)]
pub struct StatusCommand {}

pub async fn run(
    _cmd: StatusCommand,
    config: &AEyeConfig,
    repo_root: Option<PathBuf>,
) -> anyhow::Result<()> {
    let profile_path = ".nlpg/system.json";
    let profile_exists = repo_root
        .as_ref()
        .is_some_and(|root| root.join(profile_path).exists());
    let profile_status = if profile_exists { "Found" } else { "Not found" };

    println!("==============================");
    println!("A-Eye Status:");
    println!(
        "  Tier: {} (Default - Plan & Diff only)",
        config.default_tier
    );
    println!("  Policy Mode: Safe");
    println!("  Last Run ID: N/A (no runs yet)");
    println!("  System Profile: {profile_status} ({profile_path})");
    println!("==============================");

    Ok(())
}
