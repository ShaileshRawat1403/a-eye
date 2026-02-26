use crate::aeye::config;
use crate::aeye::policy::PolicyEngine;
use crate::aeye::scanner::SystemProfile;
use anyhow::{Context, Result};
use clap::Parser;
use owo_colors::OwoColorize;
use std::fs;
use std::io::{self, Write};

/// Runs verification commands from SystemProfile (Tier 2+).
#[derive(Debug, Parser)]
pub struct VerifyCommand {}

pub async fn run(_cmd: VerifyCommand, policy_engine: &PolicyEngine) -> Result<()> {
    let repo_root = config::find_repo_root()
        .context("Could not find repository root. Are you in a git repository?")?;

    // 1. Load SystemProfile
    let profile_path = repo_root.join(".nlpg/system.json");
    if !profile_path.exists() {
        println!("System profile not found at {}. Run `a-eye scan` to generate it.", profile_path.display());
        return Ok(());
    }
    let profile_content = fs::read_to_string(&profile_path)?;
    let system_profile: SystemProfile = serde_json::from_str(&profile_content)?;

    if system_profile.verify_commands.is_empty() {
        println!("No verification commands found in the system profile.");
        return Ok(());
    }

    // 2. Check Tier and act accordingly
    if !policy_engine.check_tier(2) {
        // Tier 1: Print commands only
        println!("{}", "Running in read-only mode (Tier < 2).".yellow());
        println!("The following verification commands would be run:");
        for cmd in &system_profile.verify_commands {
            println!("  - {}", cmd.cyan());
        }
        println!("\nTo execute these commands, run A-Eye in Tier 2 or higher.");
    } else {
        // Tier 2: Prompt and execute
        println!("The following verification commands will be run:");
        for cmd in &system_profile.verify_commands {
            println!("  - {}", cmd.cyan());
        }

        if !prompt_for_approval("Execute these commands?")? {
            println!("Verification cancelled by user.");
            return Ok(());
        }

        for command_str in &system_profile.verify_commands {
            // Check against shell denylist
            if !policy_engine.check_shell(command_str) {
                anyhow::bail!(
                    "Policy violation: Execution of command `{}` is forbidden by a `shell_deny_patterns` rule in your a-eye.yaml.",
                    command_str
                );
            }

            println!("\n> {}", command_str.bold());

            // Use a shell to execute the command string directly.
            // This is more robust for commands with quotes or other shell syntax.
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(command_str)
                .current_dir(&repo_root)
                .output()
                .with_context(|| format!("Failed to execute command: {}", command_str))?;

            // Stream stdout and stderr to give the user immediate feedback.
            if !output.stdout.is_empty() {
                io::stdout().write_all(&output.stdout)?;
            }
            if !output.stderr.is_empty() {
                io::stderr().write_all(&output.stderr.red().to_string().as_bytes())?;
            }

            if !output.status.success() {
                anyhow::bail!(
                    "Verification command `{}` failed with exit code {}. Aborting.",
                    command_str,
                    output.status.code().unwrap_or(1)
                );
            }
        }
        println!("\n✅ All verification commands passed successfully.");
    }

    Ok(())
}

fn prompt_for_approval(prompt_text: &str) -> Result<bool> {
    print!("{} [y/N]: ", prompt_text);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().eq_ignore_ascii_case("y"))
}
