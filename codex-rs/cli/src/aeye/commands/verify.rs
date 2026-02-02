use crate::aeye::artifacts::read_validated_json_artifact;
use crate::aeye::config;
use crate::aeye::policy::PolicyEngine;
use crate::aeye::policy::prompt_for_approval;
use crate::aeye::scanner::SystemProfile;
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use std::io;
use std::io::Write;

/// Runs verification commands from SystemProfile (Tier 2+).
#[derive(Debug, Parser)]
pub struct VerifyCommand {}

pub async fn run(_cmd: VerifyCommand, policy_engine: &PolicyEngine) -> Result<()> {
    let repo_root = config::find_repo_root()
        .context("Could not find repository root. Are you in a git repository?")?;

    let profile_path = repo_root.join(".nlpg/system.json");
    if !profile_path.exists() {
        println!(
            "System profile not found at {}. Run `a-eye scan` to generate it.",
            profile_path.display()
        );
        return Ok(());
    }

    let profile_content = read_validated_json_artifact(&profile_path, "system.json")?;
    let system_profile: SystemProfile = serde_json::from_str(&profile_content)?;

    if system_profile.verify_commands.is_empty() {
        println!("No verification commands found in the system profile.");
        return Ok(());
    }

    if !policy_engine.check_tier(2) {
        println!("Running in read-only mode (Tier < 2).");
        println!("The following verification commands would be run:");
        for command in &system_profile.verify_commands {
            println!("  - {command}");
        }
        println!("\nTo execute these commands, run A-Eye in Tier 2 or higher.");
        return Ok(());
    }

    println!("The following verification commands will be run:");
    for command in &system_profile.verify_commands {
        println!("  - {command}");
    }

    if !prompt_for_approval("Execute these commands?")? {
        println!("Verification cancelled by user.");
        return Ok(());
    }

    for command_str in &system_profile.verify_commands {
        policy_engine.enforce_shell_command(command_str)?;

        println!("\n> {command_str}");
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(command_str)
            .current_dir(&repo_root)
            .output()
            .with_context(|| format!("Failed to execute command: {command_str}"))?;

        if !output.stdout.is_empty() {
            io::stdout().write_all(&output.stdout)?;
        }
        if !output.stderr.is_empty() {
            io::stderr().write_all(&output.stderr)?;
        }

        if !output.status.success() {
            anyhow::bail!(
                "Verification command `{}` failed with exit code {}. Aborting.",
                command_str,
                output.status.code().unwrap_or(1)
            );
        }
    }

    println!("\nAll verification commands passed successfully.");
    Ok(())
}
