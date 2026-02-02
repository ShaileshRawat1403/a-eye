use anyhow::Result;

pub async fn generate_summary(
    intent_json: &str,
    plan_json: &str,
    patch_diff: &str,
) -> Result<String> {
    let intent: serde_json::Value = serde_json::from_str(intent_json)?;
    let plan: serde_json::Value = serde_json::from_str(plan_json)?;

    let goal = intent
        .get("goal")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("(goal unavailable)");
    let plan_title = plan
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("(plan title unavailable)");

    let steps = plan
        .get("steps")
        .and_then(serde_json::Value::as_array)
        .map_or_else(Vec::new, |values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        });

    let summary = format!(
        "# Learning Summary\n\n## Goal\n{goal}\n\n## Plan\n- {plan_title}\n{}\n\n## Patch\nThe generated patch contains {} lines and should be reviewed before apply.\n\n## Key Learning\nThis learning run shows a repeatable workflow: capture intent, create a plan, generate a patch, and verify safely.",
        steps
            .iter()
            .map(|step| format!("- {step}"))
            .collect::<Vec<_>>()
            .join("\n"),
        patch_diff.lines().count(),
    );

    Ok(summary)
}
