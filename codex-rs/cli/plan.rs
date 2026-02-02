use crate::aeye::artifacts::Run;
use crate::aeye::config::{self, AEyeConfig};
use crate::aeye::planner;
use crate::aeye::scanner::SystemProfile;
use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::borrow::Cow;
use uuid::Uuid;

/// Builds a plan from natural language intent.
#[derive(Debug, Parser)]
pub struct PlanCommand {
    /// The natural language goal for the plan.
    #[clap(value_name = "GOAL")]
    pub goal: String,

    /// Globs to scope the files A-Eye should consider.
    #[clap(long, value_name = "GLOB")]
    pub scope: Vec<String>,

    /// The risk tolerance for the operation.
    #[clap(long, value_enum, default_value = "low")]
    pub risk: RiskTolerance,

    /// The model to use for generating the plan, overriding the default.
    #[clap(long)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskTolerance {
    Low,
    Medium,
    High,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Intent {
    pub id: String,
    pub goal: String,
    pub constraints: Vec<String>,
    pub environment: Environment,
    pub risk_tolerance: RiskTolerance,
    pub scope: Vec<String>,
    pub success_criteria: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Environment {
    pub os: String,
    pub shell: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Plan {
    pub title: String,
    pub steps: Vec<String>,
}

pub async fn run(cmd: PlanCommand, config: &AEyeConfig) -> Result<()> {
    let repo_root = config::find_repo_root()
        .context("Could not find repository root. Are you in a git repository?")?;

    // 2. Create a new run and artifact store
    let run = Run::new(&repo_root)?;
    println!("Starting run: {}", run.id);
    println!("Artifacts will be saved to: {}", run.path.display());

    execute_plan_step(&run, cmd, config).await?;

    println!("\nTo generate a patch from this plan, run:");
    println!(
        "  aeye patch --from {}",
        run.path.join("plan.json").display()
    );

    Ok(())
}

/// The core logic of the plan step, callable from `a-eye run` as well.
pub async fn execute_plan_step(run: &Run, cmd: PlanCommand, config: &AEyeConfig) -> Result<()> {
    let repo_root = config::find_repo_root().context("Not in a git repo")?;

    // Create a temporary config for this operation if a model override is provided.
    // This requires AEyeConfig to be Clone.
    let plan_config = if let Some(model_override) = &cmd.model {
        let mut new_config = config.clone();
        new_config.core.model = Some(model_override.clone());
        Cow::Owned(new_config)
    } else {
        Cow::Borrowed(config)
    };

    // 1. Load SystemProfile
    let profile_path = repo_root.join(".nlpg/system.json");
    if !profile_path.exists() {
        anyhow::bail!(
            "System profile not found at {}. Please run `a-eye scan` first.",
            profile_path.display()
        );
    }
    let profile_content = fs::read_to_string(&profile_path)?;
    let system_profile: SystemProfile = serde_json::from_str(&profile_content)?;

    // 3. Build and persist the Intent
    let intent = Intent {
        id: Uuid::new_v4().to_string(),
        goal: cmd.goal,
        constraints: vec![], // For future use
        environment: Environment {
            os: std::env::consts::OS.to_string(),
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string()),
        },
        risk_tolerance: cmd.risk,
        scope: cmd.scope,
        success_criteria: vec![], // For future use
    };
    run.write_artifact("intent.json", &intent)?;

    // 4. Copy the SystemProfile into the run artifacts
    run.write_artifact("system.json", &system_profile)?;

    // 5. Generate a plan by calling the LLM, passing the run for logging.
    let plan = planner::generate_plan_from_llm(run, &plan_config, &intent, &system_profile).await?;
    run.write_artifact("plan.json", &plan)?;

    // 6. Print summary
    println!("\n--- Plan Generated ---");
    println!("Title: {}", plan.title);
    for (i, step) in plan.steps.iter().enumerate() {
        println!("{}. {}", i + 1, step);
    }
    println!("----------------------");

    Ok(())
}
