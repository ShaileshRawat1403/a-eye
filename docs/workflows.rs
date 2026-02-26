use crate::aeye::{
    artifacts::Run,
    commands::{apply, patch, plan, scan, verify},
    config,
    policy::PolicyEngine,
};
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use serde::Deserialize;
use futures::future::join_all;
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
    #[serde(default)]
    pub parallel: bool,
}

pub struct WorkflowEngine<'a> {
    repo_root: PathBuf,
    policy_engine: &'a PolicyEngine,
    context: HashMap<String, HashMap<String, String>>,
}

impl<'a> WorkflowEngine<'a> {
    pub fn new(repo_root: PathBuf, policy_engine: &'a PolicyEngine) -> Self {
        Self {
            repo_root,
            policy_engine,
            context: HashMap::new(),
        }
    }

    pub async fn run(&mut self, recipe_name: &str, goal: Option<String>) -> Result<()> {
        let recipe_path = self.repo_root.join("recipes").join(format!("{}.yaml", recipe_name));
        if !recipe_path.exists() {
            anyhow::bail!("Recipe '{}' not found at {}", recipe_name, recipe_path.display());
        }

        let recipe_content = fs::read_to_string(&recipe_path)?;
        let recipe: Recipe = serde_yaml::from_str(&recipe_content)?;

        println!("\nRunning workflow: {}", recipe.name.bold());
        println!("Description: {}", recipe.description);

        // Prime the context with user-provided inputs
        self.context.insert("user".to_string(), HashMap::from([("goal".to_string(), goal.clone().unwrap_or_default())]));

        let run = Run::new(&self.repo_root)?;
        println!("Created run: {}", run.id.cyan());
        println!("Artifacts will be saved to: {}", run.path.display());

        let mut i = 0;
        while i < recipe.steps.len() {
            let mut parallel_steps = vec![];
            // Group consecutive parallel steps
            while i < recipe.steps.len() && recipe.steps[i].parallel {
                parallel_steps.push(&recipe.steps[i]);
                i += 1;
            }

            if !parallel_steps.is_empty() {
                println!("\n--- Running Parallel Steps ---");
                let futures: Vec<_> = parallel_steps
                    .iter()
                    .map(|step| self.execute_step(step, &run))
                    .collect();

                let results = join_all(futures).await;

                for (step, result) in parallel_steps.iter().zip(results) {
                    let outputs = result?;
                    if let Some(outputs) = outputs {
                        self.context.insert(step.name.clone(), outputs);
                    }
                }
            }

            // Execute next sequential step
            if i < recipe.steps.len() {
                let step = &recipe.steps[i];
                println!("\n--- Step {}/{} ---", i + 1, recipe.steps.len());
                if let Some(outputs) = self.execute_step(step, &run).await? {
                    self.context.insert(step.name.clone(), outputs);
                }
                i += 1;
            }
        }

        println!("\n✅ Workflow '{}' completed successfully.", recipe.name);
        Ok(())
    }

    async fn execute_step(&self, step: &Step, run: &Run) -> Result<Option<HashMap<String, String>>> {
        println!("Name: {}", step.name.bold());
        println!("Action: {}", step.action.yellow());
        let mut outputs = HashMap::new();

        if let Some(condition) = &step.when {
            println!("Condition: {}", condition.italic());
            if !self.evaluate_condition(condition)? {
                println!("{}", "Skipped (condition not met)".yellow());
                return Ok(None);
            }
        }

        println!("--------------------");

        match step.action.as_str() {
            "system.scan" => scan::run(scan::ScanCommand {}).await?,
            "llm.plan" => {
                let goal = self.get_input(&step.inputs, "goal")?;
                let plan_cmd = plan::PlanCommand { goal, scope: vec![], risk: plan::RiskTolerance::Low, model: None };
                plan::execute_plan_step(run, plan_cmd, &self.policy_engine.config).await?;
            }
            "llm.patch" => {
                let patch_path = patch::execute_patch_step(run, &self.repo_root, &self.policy_engine.config).await?;
                outputs.insert("patch_path".to_string(), patch_path.to_string_lossy().to_string());
            }
            "tools.apply" => {
                let patch_path_str = self.get_input(&step.inputs, "patch")?;
                let patch_path = PathBuf::from(patch_path_str);
                let apply_cmd = apply::ApplyCommand { from: patch_path, dry_run: false };
                apply::run(apply_cmd, self.policy_engine).await?;
            }
            "tools.verify" => verify::run(verify::VerifyCommand {}, self.policy_engine).await?,
            _ => anyhow::bail!("Unknown workflow action: {}", step.action),
        }
        if outputs.is_empty() {
            Ok(None)
        } else {
            Ok(Some(outputs))
        }
    }

    /// Evaluates a condition string from a recipe step.
    fn evaluate_condition(&self, condition: &str) -> Result<bool> {
        if let Some(tier_str) = condition.strip_prefix("tier >= ") {
            let required_tier = tier_str.trim().parse::<u8>()
                .with_context(|| format!("Invalid tier in condition: {}", condition))?;
            return Ok(self.policy_engine.check_tier(required_tier));
        }

        if let Some(file_path_str) = condition.strip_prefix("file_exists: ") {
            let file_path = self.repo_root.join(file_path_str.trim());
            return Ok(file_path.exists());
        }

        anyhow::bail!("Unknown condition format in recipe: '{}'", condition);
    }

    fn get_input(&self, inputs: &HashMap<String, String>, key: &str) -> Result<String> {
        let template = inputs.get(key).with_context(|| format!("Missing required input '{}'", key))?;
        self.render_template(template)
    }

    fn render_template(&self, template: &str) -> Result<String> {
        let re = regex_lite::Regex::new(r"\{\{\s*(.*?)\s*\}\}")?;
        let mut result = template.to_string();
        let mut replacements = vec![];

        for captures in re.captures_iter(template) {
            let full_match = captures.get(0).unwrap().as_str();
            let binding = captures.get(1).context("Invalid template binding")?.as_str().trim();
            let parts: Vec<&str> = binding.split('.').collect();

            if parts.len() == 4 && parts[0] == "steps" && parts[2] == "outputs" {
                let step_name = parts[1].trim();
                let output_key = parts[3].trim();
                let value = self.context.get(step_name)
                    .and_then(|outputs| outputs.get(output_key))
                    .with_context(|| format!("Could not resolve template variable '{}'", binding))?;
                replacements.push((full_match.to_string(), value.clone()));
            } else if parts.len() == 2 && parts[0] == "user" {
                let key = parts[1].trim();
                let value = self.context.get("user")
                    .and_then(|user_data| user_data.get(key))
                    .with_context(|| format!("Could not resolve template variable '{}'", binding))?;
                replacements.push((full_match.to_string(), value.clone()));
            } else {
                anyhow::bail!("Could not resolve template variable '{}'", binding);
            }
        }

        for (placeholder, value) in replacements {
            result = result.replace(placeholder, &value);
        }

        Ok(result)
    }
}
