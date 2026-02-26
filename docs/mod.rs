use clap::Parser;
use codex_common::CliConfigOverrides;
use std::path::PathBuf;

pub mod commands;
pub mod config;
pub mod explainer;
pub mod policy;
pub mod learner;
pub mod llm_client;
pub mod patcher;
pub mod planner;
pub mod scanner;
pub mod workflows;
pub mod artifacts;

/// A-Eye: Intent-driven NLPg development agent.
#[derive(Debug, Parser)]
pub struct AEyeCli {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    #[clap(subcommand)]
    pub subcommand: AEyeSubcommand,
}

#[derive(Debug, Parser)]
pub enum AEyeSubcommand {
    /// Scans the repository and generates a SystemProfile.
    Scan(commands::scan::ScanCommand),
    /// Builds a plan from natural language intent.
    Plan(commands::plan::PlanCommand),
    /// Provides detailed explanation of a code construct.
    Explain(commands::explain::ExplainCommand),
    /// Generates a patch file from a plan.
    Patch(commands::patch::PatchCommand),
    /// Applies a patch after user approval (Tier 2+).
    Apply(commands::apply::ApplyCommand),
    /// Runs verification commands from SystemProfile (Tier 2+).
    Verify(commands::verify::VerifyCommand),
    /// Runs a deterministic workflow recipe.
    Run(commands::run::RunCommand),
    /// Shows current A-Eye status and policy.
    Status(commands::status::StatusCommand),
    /// Generates a learning summary from a past run.
    Learn(commands::learn::LearnCommand),
}

pub async fn run_main(
    cli: AEyeCli,
    _codex_linux_sandbox_exe: Option<PathBuf>, // Placeholder for now
) -> anyhow::Result<()> {
    // Load configuration and initialize the policy engine for the run.
    let (config, repo_root) = config::load_config()?;
    let _policy_engine = policy::PolicyEngine::new(config.clone(), repo_root.clone());

    match cli.subcommand {
        AEyeSubcommand::Apply(cmd) => commands::apply::run(cmd, &_policy_engine).await,
        AEyeSubcommand::Explain(cmd) => commands::explain::run(cmd).await,
        AEyeSubcommand::Patch(cmd) => commands::patch::run(cmd, &config).await,
        AEyeSubcommand::Plan(cmd) => commands::plan::run(cmd, &config).await,
        AEyeSubcommand::Run(cmd) => commands::run::run(cmd, &_policy_engine).await,
        AEyeSubcommand::Scan(cmd) => commands::scan::run(cmd).await,
        AEyeSubcommand::Verify(cmd) => commands::verify::run(cmd, &_policy_engine).await,
        AEyeSubcommand::Status(cmd) => commands::status::run(cmd, config, repo_root).await,
        AEyeSubcommand::Learn(cmd) => commands::learn::run(cmd).await,
    }
}
