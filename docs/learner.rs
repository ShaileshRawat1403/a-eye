use crate::aeye::{artifacts::Run, config::AEyeConfig, llm_client};
use anyhow::{Context, Result};
use codex_core::{Prompt, ResponseEvent};
use futures::StreamExt;
use tokio::time::{sleep, Duration};

/// Constructs the prompt for the LLM to generate a learning summary.
fn build_prompt(intent_json: &str, plan_json: &str, patch_diff: &str) -> String {
    format!(
r#"You are an expert software engineer. Your task is to analyze a completed run from an AI agent and generate a concise, human-readable summary of what was learned.

The run was composed of an intent, a plan, and a resulting patch.

**Intent:**
```json
{}
```

**Plan:**
```json
{}
```

**Generated Patch:**
```diff
{}
```

**Instructions:**
Based on these artifacts, generate a learning summary in Markdown format. The summary should answer:
1.  **What was the original goal?** (Summarize the intent).
2.  **What was the agent's approach?** (Summarize the plan).
3.  **What was the result?** (Explain the patch and its effect).
4.  **What is the key takeaway or learning?** (e.g., "This demonstrates how to add a new route in a React component," or "This shows a pattern for fixing null-pointer exceptions by adding a guard clause.").

Your output should be a single markdown block. Do not include any other text or explanation outside of the markdown.
"#,
        intent_json, plan_json, patch_diff
    )
}

/// Generates a learning summary from the artifacts of a run.
pub async fn generate_summary_from_llm(
    run: &Run,
    aeye_config: &AEyeConfig,
    intent_json: &str,
    plan_json: &str,
    patch_diff: &str,
) -> Result<String> {
    let mut session = llm_client::new_model_client_session(aeye_config, "a-eye-learn").await?;

    let prompt_str = build_prompt(intent_json, plan_json, patch_diff);
    let prompt = Prompt::from_user_prompt(&prompt_str);

    const MAX_RETRIES: u8 = 2;
    for attempt in 0..=MAX_RETRIES {
        match session.stream(&prompt).await {
            Ok(mut stream) => {
                let mut full_response = String::new();
                while let Some(event) = stream.next().await {
                    if let ResponseEvent::TextDelta { delta } = event? {
                        full_response.push_str(&delta);
                    }
                }
                return Ok(full_response);
            }
            Err(e) => {
                if attempt == MAX_RETRIES {
                    return Err(e).context("LLM call failed after multiple retries.");
                }
                let log_msg = format!("LLM call failed on attempt {}. Retrying. Error: {}", attempt + 1, e);
                eprintln!("Warning: {}", &log_msg);
                run.write_log_file(&format!("learner_llm_error_{}.log", attempt), &log_msg)?;
                sleep(Duration::from_secs(1 + attempt as u64)).await;
            }
        }
    }

    unreachable!("Loop should have returned or errored out.");
}
