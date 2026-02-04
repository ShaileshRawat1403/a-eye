use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use codex_core::config::edit::ConfigEdit;
use codex_core::config::edit::ConfigEditsBuilder;
use codex_core::config::find_codex_home;
use std::io;
use std::io::IsTerminal;
use std::io::Write;
use toml_edit::Item as TomlItem;
use toml_edit::value;

use crate::model_setup;

/// First-run wizard for non-developers: model setup + safety tier.
#[derive(Debug, Clone, Parser, Default)]
pub struct SetupCommand {
    #[command(flatten)]
    pub model_setup: model_setup::ModelSetupCommand,

    /// Safety tier to persist (1 = plan+diff only, 2 = supervised apply, 3 = guided autonomy).
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=3))]
    pub tier: Option<u8>,
}

#[derive(Debug, Clone, Copy)]
struct SafetyTierConfig {
    tier: u8,
    approval_policy: &'static str,
    sandbox_mode: &'static str,
    description: &'static str,
}

pub async fn run(command: SetupCommand) -> Result<()> {
    println!("A-Eye first-run wizard");
    println!("Step 1/2 - Model provider");
    model_setup::run_setup_command(command.model_setup).await?;

    let tier = resolve_tier(command.tier)?;
    let tier_config = tier_config(tier)?;

    println!("\nStep 2/2 - Safety tier");
    persist_safety_tier(tier_config).await?;
    print_tier_summary(tier_config);
    println!("\nSetup complete. Run `a-eye` to start the interactive UI.");

    Ok(())
}

fn resolve_tier(provided_tier: Option<u8>) -> Result<u8> {
    if let Some(tier) = provided_tier {
        return Ok(tier);
    }

    let is_interactive = io::stdin().is_terminal() && io::stdout().is_terminal();
    if !is_interactive {
        println!("No safety tier provided. Defaulting to Tier 1 (safest).");
        return Ok(1);
    }

    pick_tier()
}

fn pick_tier() -> Result<u8> {
    println!("Choose your safety tier:");
    println!("  1) Tier 1 - Plan + diff only (safest default)");
    println!("  2) Tier 2 - Supervised apply (recommended once comfortable)");
    println!("  3) Tier 3 - Guided autonomy (advanced users)");

    loop {
        let choice = prompt("Enter choice [1-3]")?;
        match choice.trim() {
            "1" => return Ok(1),
            "2" => return Ok(2),
            "3" => return Ok(3),
            _ => println!("Invalid choice. Enter 1, 2, or 3."),
        }
    }
}

fn tier_config(tier: u8) -> Result<SafetyTierConfig> {
    let config = match tier {
        1 => SafetyTierConfig {
            tier,
            approval_policy: "untrusted",
            sandbox_mode: "read-only",
            description: "Plan + diff only. A-Eye stays in the safest mode.",
        },
        2 => SafetyTierConfig {
            tier,
            approval_policy: "untrusted",
            sandbox_mode: "workspace-write",
            description: "Supervised apply. You approve changes before riskier actions.",
        },
        3 => SafetyTierConfig {
            tier,
            approval_policy: "on-request",
            sandbox_mode: "workspace-write",
            description: "Guided autonomy. Faster flow with interactive approvals.",
        },
        _ => anyhow::bail!("Unsupported tier: {tier}. Expected 1, 2, or 3."),
    };
    Ok(config)
}

async fn persist_safety_tier(tier_config: SafetyTierConfig) -> Result<()> {
    let codex_home = find_codex_home()?;
    let edits = vec![
        set_path(&["approval_policy"], value(tier_config.approval_policy)),
        set_path(&["sandbox_mode"], value(tier_config.sandbox_mode)),
    ];

    ConfigEditsBuilder::new(&codex_home)
        .with_edits(edits)
        .apply()
        .await
        .context("failed to persist safety tier setup")?;

    Ok(())
}

fn set_path(path: &[&str], value: TomlItem) -> ConfigEdit {
    ConfigEdit::SetPath {
        segments: path.iter().map(|segment| (*segment).to_string()).collect(),
        value,
    }
}

fn print_tier_summary(tier_config: SafetyTierConfig) {
    let tier = tier_config.tier;
    let approval_policy = tier_config.approval_policy;
    let sandbox_mode = tier_config.sandbox_mode;
    let description = tier_config.description;

    println!("Safety tier saved.");
    println!("  Tier: {tier}");
    println!("  Approval policy: {approval_policy}");
    println!("  Sandbox mode: {sandbox_mode}");
    println!("  {description}");
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}: ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn tier_one_maps_to_safe_defaults() {
        let config = tier_config(1).expect("tier should resolve");
        assert_eq!(config.approval_policy, "untrusted");
        assert_eq!(config.sandbox_mode, "read-only");
    }

    #[test]
    fn tier_two_maps_to_supervised_apply() {
        let config = tier_config(2).expect("tier should resolve");
        assert_eq!(config.approval_policy, "untrusted");
        assert_eq!(config.sandbox_mode, "workspace-write");
    }

    #[test]
    fn tier_three_maps_to_guided_autonomy() {
        let config = tier_config(3).expect("tier should resolve");
        assert_eq!(config.approval_policy, "on-request");
        assert_eq!(config.sandbox_mode, "workspace-write");
    }
}
