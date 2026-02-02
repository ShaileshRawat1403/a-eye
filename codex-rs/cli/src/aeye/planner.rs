use crate::aeye::commands::plan::Intent;
use crate::aeye::commands::plan::Plan;
use crate::aeye::scanner::SystemProfile;
use anyhow::Result;

/// Generates a deterministic starter plan.
pub async fn generate_plan_from_llm(
    _intent_run: &crate::aeye::artifacts::Run,
    intent: &Intent,
    _system_profile: &SystemProfile,
) -> Result<Plan> {
    Ok(Plan {
        title: format!("Plan to achieve: {}", intent.goal),
        steps: vec![
            "Inspect the relevant files in scope.".to_string(),
            "Implement the minimal safe change required for the goal.".to_string(),
            "Verify the change using project test commands.".to_string(),
        ],
    })
}
