use crate::aeye::commands::plan::{Intent, Plan};
use crate::aeye::config::AEyeConfig;
use crate::aeye::llm_client;
use crate::aeye::scanner::SystemProfile;
use anyhow::{Context, Result};
use codex_core::{Prompt, ResponseEvent};
use futures::StreamExt;
use glob::glob;
use std::fs;
use std::path::Path;
use tokio::time::{sleep, Duration};
use tree_sitter::{Parser as TreeSitterParser, Query, QueryCursor};

/// Generates a patch from a plan by calling an LLM.
pub async fn generate_patch_from_llm(
    repo_root: &Path,
    config: &AEyeConfig,
    intent: &Intent,
    system_profile: &SystemProfile,
    plan: &Plan,
) -> Result<String> {
    let mut prompt_context = String::new();
    prompt_context.push_str("Here are the contents of the relevant files:\n\n");

    if intent.scope.is_empty() {
        prompt_context
            .push_str("<file path=\"unknown\">No files in scope. You may need to create a new file.</file>\n");
    } else {
        for glob_pattern in &intent.scope {
            for entry in glob(&repo_root.join(glob_pattern).to_string_lossy())? {
                if let Ok(path) = entry {
                    if path.is_file() {
                        let relative_path = path.strip_prefix(repo_root).unwrap_or(&path);
                        let contextual_content = get_contextual_content(&path, plan)
                            .unwrap_or_else(|e| format!("Error analyzing file: {}", e));
                        prompt_context.push_str(&format!(
                            "<file path=\"{}\">\n{}</file>\n\n",
                            relative_path.display(),
                            contextual_content
                        ));
                    }
                }
            }
        }
    }

    let prompt = build_patcher_prompt(intent, system_profile, plan, &prompt_context);
    let mut session = llm_client::new_model_client_session(config, "a-eye-patch").await?;

    const MAX_RETRIES: u8 = 2;
    for attempt in 0..=MAX_RETRIES {
        match session.stream(&Prompt::from_user_prompt(&prompt)).await {
            Ok(mut stream) => {
                let mut full_response = String::new();
                while let Some(event) = stream.next().await {
                    if let ResponseEvent::TextDelta { delta } = event? {
                        full_response.push_str(&delta);
                    }
                }
                // Basic cleaning: The model might wrap the diff in ```diff ... ```
                let cleaned_response = full_response
                    .trim()
                    .strip_prefix("```diff")
                    .unwrap_or(&full_response)
                    .strip_prefix("```")
                    .unwrap_or(&full_response)
                    .strip_suffix("```")
                    .unwrap_or(&full_response)
                    .trim()
                    .to_string();

                if cleaned_response.starts_with("--- a/") || cleaned_response.starts_with("diff --git") {
                    return Ok(cleaned_response);
                } else {
                    if attempt == MAX_RETRIES {
                        anyhow::bail!("LLM failed to produce a valid diff after {} retries. Last response:\n{}", MAX_RETRIES + 1, cleaned_response);
                    }
                    eprintln!("Warning: LLM did not return a valid diff (attempt {}). Retrying...", attempt + 1);
                    sleep(Duration::from_secs(1 + attempt as u64)).await;
                }
            }
            Err(e) => {
                if attempt == MAX_RETRIES {
                    return Err(e).context("LLM call for patch generation failed after multiple retries.");
                }
                eprintln!("Warning: LLM call failed on attempt {}. Retrying...", attempt + 1);
                sleep(Duration::from_secs(1 + attempt as u64)).await;
            }
        }
    }
    unreachable!("Loop should have returned or errored out.");
}

fn build_patcher_prompt(
    intent: &Intent,
    system_profile: &SystemProfile,
    plan: &Plan,
    file_context: &str,
) -> String {
    format!(
        r#"You are an expert software engineer. Your task is to generate a code patch in the unified diff format to implement a given plan.

**User's Goal:**
{}

**Execution Plan:**
- {}

**System Profile:**
- Languages: {}
- Frameworks: {}

**Relevant File Context:**
{}

**Instructions:**
Based on all the context provided, generate a patch in the standard unified diff format (`--- a/...`, `+++ b/...`, `@@ ... @@`).
- The patch should only contain code changes.
- Do not include any other text, explanation, or markdown formatting like ` ```diff ` outside of the patch itself.
- Ensure the patch correctly implements the steps outlined in the plan.
- The patch must be directly applicable with the `patch` command.
"#,
        intent.goal,
        plan.steps.join("\n- "),
        system_profile.languages.join(", "),
        system_profile.frameworks.join(", "),
        file_context
    )
}

const MAX_FILE_SIZE_FOR_FULL_CONTEXT: usize = 20000; // 20KB
const CONTEXT_HEADER_LINES: usize = 50;

/// Extracts relevant snippets from a file based on the plan.
/// Falls back to a header + footer truncation for very large files or on parsing errors.
fn get_contextual_content(path: &Path, _plan: &Plan) -> Result<String> {
    let content = fs::read_to_string(path)?;
    if content.len() <= MAX_FILE_SIZE_FOR_FULL_CONTEXT {
        return Ok(content);
    }

    // For large files, attempt to use tree-sitter for intelligent extraction.
    // In a real implementation, we would extract keywords from the `plan`.
    // For this example, let's pretend the plan mentioned "main".
    let keywords = vec!["main"];

    let mut parser = TreeSitterParser::new();
    // This would be dynamic based on file type.
    let language = tree_sitter_javascript::language();
    parser.set_language(&language)?;

    let tree = match parser.parse(&content, None) {
        Some(tree) => tree,
        None => return keyword_centered_window(&content, _plan),
    };

    // Example query to find function declarations.
    let query_str = r#"(function_declaration name: (identifier) @function.name)"#;
    let query = Query::new(&language, query_str)?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    let mut relevant_snippets = String::new();
    // Always include the file header for imports and global context.
    relevant_snippets.push_str("... (file header)\n");
    relevant_snippets.push_str(
        content
            .lines()
            .take(CONTEXT_HEADER_LINES)
            .collect::<Vec<_>>()
            .join("\n")
            .as_str(),
    );
    relevant_snippets.push_str("\n... (relevant snippets)\n");

    for mat in matches {
        for cap in mat.captures {
            let node = cap.node;
            let node_name = &content[cap.node.byte_range()];
            if keywords.contains(&node_name) {
                // We found a function matching our keyword. Walk up to get the whole function body.
                let mut parent = node;
                while parent.parent().is_some() && parent.kind() != "function_declaration" {
                    parent = parent.parent().unwrap();
                }
                relevant_snippets.push_str(&content[parent.byte_range()]);
                relevant_snippets.push_str("\n...\n");
            }
        }
    }

    if relevant_snippets.len() > content.len() {
        // If our "snippets" are larger than the original, just truncate.
        return keyword_centered_window(&content, _plan);
    }

    Ok(relevant_snippets)
}

/// A simple fallback truncation method.
fn truncate_simple(content: &str) -> String {
    format!(
        "{}... (file truncated) ...{}",
        &content.chars().take(5000).collect::<String>(),
        &content.chars().rev().take(5000).collect::<String>().chars().rev().collect::<String>()
    )
}

/// A better fallback that centers the context window on a keyword from the plan.
fn keyword_centered_window(content: &str, _plan: &Plan) -> Result<String> {
    const TARGET_CONTEXT_CHARS: usize = 8000;
    let keywords = vec!["main"]; // In a real implementation, extract from `_plan`.
    let lines: Vec<&str> = content.lines().collect();

    let Some(target_line_index) = lines.iter().position(|line| keywords.iter().any(|kw| line.contains(kw))) else {
        return Ok(truncate_simple(content));
    };

    let mut start_index = target_line_index;
    let mut end_index = target_line_index;
    let mut current_chars = lines[target_line_index].len();

    while current_chars < TARGET_CONTEXT_CHARS {
        let mut changed = false;
        if start_index > 0 {
            start_index -= 1;
            current_chars += lines[start_index].len() + 1; // +1 for newline
            changed = true;
        }
        if end_index < lines.len() - 1 {
            end_index += 1;
            current_chars += lines[end_index].len() + 1; // +1 for newline
            changed = true;
        }
        if !changed {
            break; // Reached both ends of the file
        }
    }

    let mut result = String::new();
    if start_index > 0 {
        result.push_str("... (file truncated)\n");
    }
    result.push_str(&lines[start_index..=end_index].join("\n"));
    if end_index < lines.len() - 1 {
        result.push_str("\n... (file truncated)");
    }

    Ok(result)
}
