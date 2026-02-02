use crate::aeye::artifacts::Run;
use crate::aeye::commands::plan::{Intent, Plan};
use crate::aeye::config::AEyeConfig;
use crate::aeye::llm_client;
use crate::aeye::scanner::SystemProfile;
use anyhow::{Context, Result};
use codex_core::{Prompt, ResponseEvent};
use futures::StreamExt;
use jsonschema::JSONSchema;
use serde_json;
use tokio::time::{sleep, Duration};

const PLAN_SCHEMA_JSON: &str = r#"{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "title": "A-Eye Plan",
    "description": "A structured plan for A-Eye to execute.",
    "type": "object",
    "properties": {
        "title": {
            "description": "A short, human-readable title for the plan.",
            "type": "string"
        },
        "steps": {
            "description": "A list of steps to achieve the goal.",
            "type": "array",
            "items": { "type": "string" },
            "minItems": 1
        }
    },
    "required": ["title", "steps"]
}"#;

/// Generates a plan by calling an LLM with the user's intent and system context.
/// Implements a retry loop to handle cases where the LLM returns invalid JSON.
pub async fn generate_plan_from_llm(
    run: &Run,
    config: &AEyeConfig,
    intent: &Intent,
    system_profile: &SystemProfile,
) -> Result<Plan> {
    const MAX_RETRIES: u8 = 3;
    let mut last_error: Option<anyhow::Error> = None;

    let schema_val: serde_json::Value = serde_json::from_str(PLAN_SCHEMA_JSON)?;
    let compiled_schema =
        JSONSchema::compile(&schema_val).context("Failed to compile plan JSON schema")?;

    let mut prompt = build_initial_prompt(intent, system_profile);
    let mut session = llm_client::new_model_client_session(config, "a-eye-plan").await?;

    for attempt in 0..=MAX_RETRIES {
        let llm_response_str = match session.stream(&Prompt::from_user_prompt(&prompt)).await {
            Ok(mut stream) => {
                let mut full_response = String::new();
                while let Some(event) = stream.next().await {
                    if let ResponseEvent::TextDelta { delta } = event? {
                        full_response.push_str(&delta);
                    }
                }
                full_response
            }
            Err(e) => {
                last_error = Some(e.into());
                continue; // Go to next retry
            }
        };

        match serde_json::from_str::<serde_json::Value>(&llm_response_str) {
            Ok(json_value) => {
                if let Err(validation_errors) = compiled_schema.validate(&json_value) {
                    let errors: Vec<String> = validation_errors.map(|e| e.to_string()).collect();
                    let error_message = format!(
                        "The previous attempt to generate a JSON plan failed schema validation. Errors: {}. The invalid JSON was:\n```json\n{}\n```\nPlease correct the JSON to match the schema and return only the valid JSON object.",
                        errors.join(", "), llm_response_str
                    );

                    // Log the failure to the run's artifact directory
                    let log_content = format!(
                        "Attempt {}: LLM response failed schema validation.\n\nError(s):\n{}\n\nInvalid JSON:\n{}",
                        attempt,
                        errors.join("\n"),
                        llm_response_str
                    );
                    if let Err(log_err) = run.write_log_file(&format!("llm_validation_error_{}.log", attempt), &log_content) {
                        eprintln!("Warning: Failed to write validation error log: {}", log_err);
                    }

                    eprintln!(
                        "Warning: LLM returned JSON that failed schema validation (attempt {}). Retrying...",
                        attempt + 1
                    );

                    prompt.push_str("\n\n");
                    prompt.push_str(&error_message);
                    last_error = Some(anyhow::anyhow!(
                        "JSON schema validation failed: {}",
                        errors.join(", ")
                    ));
                    continue;
                }

                match serde_json::from_value::<Plan>(json_value) {
                    Ok(plan) => return Ok(plan),
                    Err(e) => {
                        last_error = Some(e.into());
                        let error_message = format!("Internal error: JSON passed schema validation but failed to deserialize into Plan struct. Error: {}", last_error.as_ref().unwrap());
                        prompt.push_str("\n\n");
                        prompt.push_str(&error_message);
                        continue;
                    }
                }
            },
            Err(e) => {
                let error_message = format!(
                    "The previous attempt to generate a JSON plan failed. Error: {}. The invalid JSON was:\n```json\n{}\n```\nPlease correct the JSON syntax and return only the valid JSON object.",
                    e, llm_response_str
                );

                // Log the failure to the run's artifact directory
                let log_content = format!(
                    "Attempt {}: LLM response was not valid JSON.\n\nError:\n{}\n\nInvalid Response:\n{}",
                    attempt,
                    e,
                    llm_response_str
                );
                if let Err(log_err) = run.write_log_file(&format!("llm_parse_error_{}.log", attempt), &log_content) {
                    eprintln!("Warning: Failed to write parse error log: {}", log_err);
                }

                eprintln!(
                    "Warning: LLM returned invalid JSON (attempt {}). Retrying...",
                    attempt + 1
                );

                prompt.push_str("\n\n");
                prompt.push_str(&error_message);
                last_error = Some(e.into());
                sleep(Duration::from_secs(1 + attempt as u64)).await;
            }
        }
    }

    Err(last_error.context(format!(
        "Failed to generate a valid plan after {} retries.",
        MAX_RETRIES + 1
    )))
}

/// Constructs the initial, detailed prompt for the LLM.
fn build_initial_prompt(intent: &Intent, system_profile: &SystemProfile) -> String {
    // In a real implementation, this would be a comprehensive prompt template.
    format!(
        r#"You are a senior software architect. Your task is to create a step-by-step plan in JSON format to achieve a goal.

The user's intent is:
```json
{}
```

The system profile is:
```json
{}
```

Respond with a JSON object matching this schema. The `steps` array must not be empty.
```json
{}
```

Do not include any other text or explanation outside of the JSON object."#,
        serde_json::to_string_pretty(intent).unwrap_or_default(),
        serde_json::to_string_pretty(system_profile).unwrap_or_default(),
        PLAN_SCHEMA_JSON
    )
}
