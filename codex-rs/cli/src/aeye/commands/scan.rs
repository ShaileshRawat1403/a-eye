use crate::aeye::artifacts;
use crate::aeye::config;
use crate::aeye::scanner::Scanner;
use crate::aeye::scanner::SystemProfile;
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::Path;

const NLPG_DIR: &str = ".nlpg";
const SYSTEM_PROFILE_FILE: &str = "system.json";

/// Scans the repository and generates a SystemProfile.
#[derive(Debug, Parser)]
pub struct ScanCommand {}

pub async fn run(_cmd: ScanCommand) -> Result<()> {
    let repo_root = config::find_repo_root()
        .context("Could not find repository root. Are you in a git repository?")?;

    println!("Scanning repository at {}...", repo_root.display());

    let profile = Scanner::scan(&repo_root)?;

    let nlpg_path = repo_root.join(NLPG_DIR);
    if !nlpg_path.exists() {
        fs::create_dir(&nlpg_path)
            .with_context(|| format!("Failed to create directory: {}", nlpg_path.display()))?;
    }

    let profile_path = nlpg_path.join(SYSTEM_PROFILE_FILE);
    artifacts::write_structured_json_artifact(&profile_path, SYSTEM_PROFILE_FILE, &profile)?;

    print_summary(&profile, &profile_path);

    Ok(())
}

fn print_summary(profile: &SystemProfile, profile_path: &Path) {
    println!(
        "\nScan complete. System profile saved to {}",
        profile_path.display()
    );
    println!("\n--- System Profile Summary ---");
    println!(
        "Languages: {}",
        if profile.languages.is_empty() {
            "none detected".to_string()
        } else {
            profile.languages.join(", ")
        }
    );
    println!(
        "Package Manager: {}",
        profile
            .package_manager
            .as_deref()
            .unwrap_or("none detected")
    );
    println!("Suggested Verify Commands:");
    if profile.verify_commands.is_empty() {
        println!("  - None detected. Consider adding a test script to your project.");
    } else {
        for command in &profile.verify_commands {
            println!("  - {command}");
        }
    }
    println!("----------------------------");
}
