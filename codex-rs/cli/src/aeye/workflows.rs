use crate::aeye::artifacts::Run;
use crate::aeye::commands::apply;
use crate::aeye::commands::patch;
use crate::aeye::commands::plan;
use crate::aeye::commands::scan;
use crate::aeye::commands::verify;
use crate::aeye::policy::PolicyEngine;
use anyhow::Context;
use anyhow::Result;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub description: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
pub struct Step {
    pub name: String,
    pub action: String,
    #[serde(default)]
    pub inputs: HashMap<String, String>,
    #[serde(default)]
    pub when: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum WorkflowRunStatus {
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum WorkflowStepStatus {
    Completed,
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowStepResult {
    name: String,
    action: String,
    status: WorkflowStepStatus,
    #[serde(default)]
    outputs: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowRunArtifact {
    schema_version: String,
    recipe_name: String,
    recipe_description: String,
    goal: String,
    status: WorkflowRunStatus,
    started_at: String,
    finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(default)]
    step_results: Vec<WorkflowStepResult>,
}

impl WorkflowRunArtifact {
    fn new(recipe: &Recipe, goal: String) -> Self {
        Self {
            schema_version: "1.0".to_string(),
            recipe_name: recipe.name.clone(),
            recipe_description: recipe.description.clone(),
            goal,
            status: WorkflowRunStatus::Running,
            started_at: Utc::now().to_rfc3339(),
            finished_at: None,
            error: None,
            step_results: Vec::new(),
        }
    }
}

pub struct WorkflowEngine<'a> {
    repo_root: PathBuf,
    policy_engine: &'a PolicyEngine,
    context: HashMap<String, HashMap<String, String>>,
}

impl<'a> WorkflowEngine<'a> {
    const WORKFLOW_ARTIFACT_FILE: &'static str = "workflow.json";

    pub fn new(repo_root: PathBuf, policy_engine: &'a PolicyEngine) -> Self {
        Self {
            repo_root,
            policy_engine,
            context: HashMap::new(),
        }
    }

    pub async fn run(&mut self, recipe_name: &str, goal: Option<String>) -> Result<()> {
        let recipe_path = self
            .repo_root
            .join("recipes")
            .join(format!("{recipe_name}.yaml"));
        if !recipe_path.exists() {
            anyhow::bail!(
                "Recipe '{recipe_name}' not found at {}",
                recipe_path.display()
            );
        }

        let recipe_content = fs::read_to_string(&recipe_path)?;
        let recipe: Recipe = serde_yaml::from_str(&recipe_content)?;

        println!("\nRunning workflow: {}", recipe.name);
        println!("Description: {}", recipe.description);

        let user_goal = goal.unwrap_or_default();
        self.context.insert(
            "user".to_string(),
            HashMap::from([("goal".to_string(), user_goal.clone())]),
        );

        let run = Run::new(&self.repo_root)?;
        println!("Created run: {}", run.id);
        let mut workflow_artifact = WorkflowRunArtifact::new(&recipe, user_goal);
        run.write_artifact(Self::WORKFLOW_ARTIFACT_FILE, &workflow_artifact)?;

        for (idx, step) in recipe.steps.iter().enumerate() {
            println!("\n--- Step {}/{} ---", idx + 1, recipe.steps.len());
            let step_result = match self.execute_step(step, &run).await {
                Ok(step_result) => step_result,
                Err(error) => {
                    let error_message = error.to_string();
                    workflow_artifact.status = WorkflowRunStatus::Failed;
                    workflow_artifact.finished_at = Some(Utc::now().to_rfc3339());
                    workflow_artifact.error = Some(error_message);
                    run.write_artifact(Self::WORKFLOW_ARTIFACT_FILE, &workflow_artifact)?;
                    return Err(error);
                }
            };

            if !step_result.outputs.is_empty() {
                self.context
                    .insert(step.name.clone(), step_result.outputs.clone());
            }
            workflow_artifact.step_results.push(step_result);
            run.write_artifact(Self::WORKFLOW_ARTIFACT_FILE, &workflow_artifact)?;
        }

        workflow_artifact.status = WorkflowRunStatus::Completed;
        workflow_artifact.finished_at = Some(Utc::now().to_rfc3339());
        run.write_artifact(Self::WORKFLOW_ARTIFACT_FILE, &workflow_artifact)?;

        println!("\nWorkflow '{}' completed successfully.", recipe.name);
        Ok(())
    }

    async fn execute_step(&self, step: &Step, run: &Run) -> Result<WorkflowStepResult> {
        println!("Name: {}", step.name);
        println!("Action: {}", step.action);

        if let Some(condition) = &step.when
            && !self.evaluate_condition(condition)?
        {
            println!("Skipped (condition not met)");
            return Ok(WorkflowStepResult {
                name: step.name.clone(),
                action: step.action.clone(),
                status: WorkflowStepStatus::Skipped,
                outputs: HashMap::new(),
                skipped_reason: Some(format!("condition_not_met: {condition}")),
            });
        }

        let mut outputs = HashMap::new();

        match step.action.as_str() {
            "system.scan" => scan::run(scan::ScanCommand {}).await?,
            "llm.plan" => {
                let goal = self.get_input(&step.inputs, "goal")?;
                let plan_cmd = plan::PlanCommand {
                    goal,
                    scope: vec![],
                    risk: plan::RiskTolerance::Low,
                };
                plan::execute_plan_step(run, plan_cmd).await?;
            }
            "llm.patch" => {
                let patch_path = patch::execute_patch_step(run, &self.repo_root).await?;
                outputs.insert(
                    "patch_path".to_string(),
                    patch_path.to_string_lossy().to_string(),
                );
            }
            "tools.apply" => {
                let patch_path_str = self.get_input(&step.inputs, "patch")?;
                let apply_cmd = apply::ApplyCommand {
                    from: PathBuf::from(patch_path_str),
                    dry_run: false,
                };
                apply::run(apply_cmd, self.policy_engine).await?;
            }
            "tools.verify" => verify::run(verify::VerifyCommand {}, self.policy_engine).await?,
            _ => anyhow::bail!("Unknown workflow action: {}", step.action),
        }

        Ok(WorkflowStepResult {
            name: step.name.clone(),
            action: step.action.clone(),
            status: WorkflowStepStatus::Completed,
            outputs,
            skipped_reason: None,
        })
    }

    fn evaluate_condition(&self, condition: &str) -> Result<bool> {
        if let Some(tier_str) = condition.strip_prefix("tier >= ") {
            let required_tier = tier_str
                .trim()
                .parse::<u8>()
                .with_context(|| format!("Invalid tier in condition: {condition}"))?;
            return Ok(self.policy_engine.check_tier(required_tier));
        }

        if let Some(file_path_str) = condition.strip_prefix("file_exists: ") {
            return Ok(self.repo_root.join(file_path_str.trim()).exists());
        }

        anyhow::bail!("Unknown condition format in recipe: '{condition}'");
    }

    fn get_input(&self, inputs: &HashMap<String, String>, key: &str) -> Result<String> {
        let template = inputs
            .get(key)
            .with_context(|| format!("Missing required input '{key}'"))?;
        self.render_template(template)
    }

    fn render_template(&self, template: &str) -> Result<String> {
        let re = regex_lite::Regex::new(r"\{\{\s*(.*?)\s*\}\}")?;
        let mut result = template.to_string();

        for captures in re.captures_iter(template) {
            let Some(full_match) = captures.get(0).map(|m| m.as_str()) else {
                continue;
            };
            let binding = captures
                .get(1)
                .context("Invalid template binding")?
                .as_str()
                .trim();
            let parts: Vec<&str> = binding.split('.').collect();

            let value = if parts.len() == 4 && parts[0] == "steps" && parts[2] == "outputs" {
                self.context
                    .get(parts[1])
                    .and_then(|outputs| outputs.get(parts[3]))
                    .with_context(|| format!("Could not resolve template variable '{binding}'"))?
                    .to_string()
            } else if parts.len() == 2 && parts[0] == "user" {
                self.context
                    .get("user")
                    .and_then(|user_data| user_data.get(parts[1]))
                    .with_context(|| format!("Could not resolve template variable '{binding}'"))?
                    .to_string()
            } else {
                anyhow::bail!("Could not resolve template variable '{binding}'");
            };

            result = result.replace(full_match, &value);
        }

        Ok(result)
    }
}
