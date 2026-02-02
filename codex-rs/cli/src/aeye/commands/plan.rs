use crate::aeye::artifacts::Run;
use crate::aeye::artifacts::read_validated_json_artifact;
use crate::aeye::config;
use crate::aeye::planner;
use crate::aeye::scanner::SystemProfile;
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use serde::Deserialize;
use serde::Serialize;
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

pub async fn run(cmd: PlanCommand) -> Result<()> {
    let repo_root = config::find_repo_root()
        .context("Could not find repository root. Are you in a git repository?")?;

    let run = Run::new(&repo_root)?;
    println!("Starting run: {}", run.id);
    println!("Artifacts will be saved to: {}", run.path.display());

    execute_plan_step(&run, cmd).await?;

    println!("\nTo generate a patch from this plan, run:");
    println!(
        "  a-eye patch --from {}",
        run.path.join("plan.json").display()
    );

    Ok(())
}

/// Core logic for workflows and CLI.
pub async fn execute_plan_step(run: &Run, cmd: PlanCommand) -> Result<()> {
    let repo_root = config::find_repo_root().context("Not in a git repo")?;

    let profile_path = repo_root.join(".nlpg/system.json");
    if !profile_path.exists() {
        anyhow::bail!(
            "System profile not found at .nlpg/system.json. Please run `a-eye scan` first."
        );
    }

    let profile_content = read_validated_json_artifact(&profile_path, "system.json")?;
    let mut system_profile: SystemProfile = serde_json::from_str(&profile_content)?;
    if system_profile.topology.trim().is_empty() {
        system_profile.topology = "single_service".to_string();
    }
    if system_profile.repo_type.trim().is_empty() {
        system_profile.repo_type = "unknown".to_string();
    }

    let intent = Intent {
        id: Uuid::new_v4().to_string(),
        goal: cmd.goal,
        constraints: vec![],
        environment: Environment {
            os: std::env::consts::OS.to_string(),
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string()),
        },
        risk_tolerance: cmd.risk,
        scope: cmd.scope,
        success_criteria: vec![],
    };

    run.write_artifact("intent.json", &intent)?;
    run.write_artifact("system.json", &system_profile)?;

    let plan = planner::generate_plan_from_llm(run, &intent, &system_profile).await?;
    run.write_artifact("plan.json", &plan)?;

    println!("\n--- Plan Generated ---");
    println!("Title: {}", plan.title);
    for (idx, step) in plan.steps.iter().enumerate() {
        println!("{}. {}", idx + 1, step);
    }
    println!("----------------------");

    Ok(())
}
