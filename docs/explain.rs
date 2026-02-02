use crate::aeye::artifacts::Run;
use crate::aeye::config;
use crate::aeye::explainer;
use crate::aeye::scanner::SystemProfile;
use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Parser as TreeSitterParser, Point};

/// Provides detailed explanation of a code construct.
#[derive(Debug, Parser)]
pub struct ExplainCommand {
    /// Path to the file to explain.
    #[clap(value_name = "PATH")]
    pub path: PathBuf,

    /// The line number of the code construct to explain.
    #[clap(long, short)]
    pub line: usize,

    /// Suggest safe edits for the code.
    #[clap(long)]
    pub safe_edits: bool,

    /// Provide examples of the code construct.
    #[clap(long)]
    pub examples: bool,
}

/// Extracts a syntactically-aware block of code around a target line using tree-sitter.
fn extract_syntactic_block(content: &str, path: &Path, line: usize) -> Result<String> {
    let mut parser = TreeSitterParser::new();

    // A production implementation would dynamically select the language.
    let language = match path.extension().and_then(|s| s.to_str()) {
        Some("js") | Some("mjs") | Some("cjs") => tree_sitter_javascript::language(),
        // Add other languages here, e.g., tree_sitter_rust::language()
        _ => anyhow::bail!("Unsupported file type for syntactic analysis."),
    };
    parser.set_language(&language)?;

    let tree = parser
        .parse(content, None)
        .context("Failed to parse file with tree-sitter")?;
    let root_node = tree.root_node();

    // The line number is 1-based, but tree-sitter points are 0-based.
    let target_point = Point::new(line.saturating_sub(1), 0);

    // Find the smallest node that contains the target line.
    let Some(mut target_node) = root_node.descendant_for_point_range(target_point, target_point) else {
        anyhow::bail!("Could not find a syntax node at the specified line.");
    };

    // Walk up the tree to find a meaningful block-level parent.
    // These node kinds are language-specific.
    const INTERESTING_NODE_KINDS: &[&str] = &[
        "function_declaration",
        "function",
        "arrow_function",
        "if_statement",
        "for_statement",
        "while_statement",
        "try_statement",
        "class_declaration",
        "method_definition",
    ];

    loop {
        if INTERESTING_NODE_KINDS.contains(&target_node.kind()) {
            // We found a good block, break and use this one.
            break;
        }
        if let Some(parent) = target_node.parent() {
            target_node = parent;
        } else {
            // We've reached the root, so just use the current node.
            break;
        }
    }

    // Now, format the extracted block with line numbers and the target marker.
    let start_line = target_node.start_position().row;
    let end_line = target_node.end_position().row;
    let all_lines: Vec<&str> = content.lines().collect();

    let snippet_with_context: String = all_lines[start_line..=end_line]
        .iter()
        .enumerate()
        .map(|(i, line_content)| {
            let current_line_num = start_line + i;
            let marker = if current_line_num == line.saturating_sub(1) { ">" } else { " " };
            format!("{} {:4} | {}\n", marker, current_line_num + 1, line_content)
        })
        .collect();

    Ok(snippet_with_context)
}

pub async fn run(cmd: ExplainCommand) -> Result<()> {
    let repo_root = config::find_repo_root()
        .context("Could not find repository root. Are you in a git repository?")?;

    // Create a new run for artifacts
    let run = Run::new(&repo_root)?;
    println!("Starting run: {}", run.id);
    println!("Artifacts will be saved to: {}", run.path.display());

    // Read file content
    let file_content = fs::read_to_string(&cmd.path)
        .with_context(|| format!("Failed to read file: {}", cmd.path.display()))?;

    // Extract code snippet using tree-sitter, with a fallback to the fixed-window method.
    let snippet_with_context =
        match extract_syntactic_block(&file_content, &cmd.path, cmd.line) {
            Ok(snippet) => snippet,
            Err(e) => {
                eprintln!(
                    "Warning: Syntactic analysis failed ({}). Falling back to line-based context.",
                    e
                );
                // Fallback to fixed-window logic
                let lines: Vec<&str> = file_content.lines().collect();
                let line_index = cmd.line.saturating_sub(1);
                if line_index >= lines.len() {
                    anyhow::bail!(
                        "Line number {} is out of bounds for file {}",
                        cmd.line,
                        cmd.path.display()
                    );
                }

                const CONTEXT_LINES: usize = 3;
                let start = line_index.saturating_sub(CONTEXT_LINES);
                let end = (line_index + CONTEXT_LINES + 1).min(lines.len());

                lines[start..end]
                    .iter()
                    .enumerate()
                    .map(|(i, line)| {
                        let current_line_num = start + i;
                        let marker = if current_line_num == line_index { ">" } else { " " };
                        format!("{} {:4} | {}\n", marker, current_line_num + 1, line)
                    })
                    .collect()
            }
        };

    // Load SystemProfile if it exists, to get verify_commands
    let profile_path = repo_root.join(".nlpg/system.json");
    let system_profile = if profile_path.exists() {
        let profile_content = fs::read_to_string(&profile_path)?;
        serde_json::from_str::<SystemProfile>(&profile_content).ok()
    } else {
        None
    };
    let verify_commands = system_profile.map_or(vec![], |p| p.verify_commands);

    // Generate explanation
    let target_code_line = file_content.lines().nth(cmd.line.saturating_sub(1)).unwrap_or("");
    println!("\nGenerating explanation for: `{}`...", target_code_line.trim());
    let explanation =
        explainer::generate_explanation_from_llm(&snippet_with_context, verify_commands).await?;

    // Persist artifact
    run.write_artifact("explain.json", &explanation)?;

    // Print summary
    println!("\n--- Explanation ---");
    println!("Construct Type: {}", explanation.construct_type);
    println!("\nIntent:\n  {}", explanation.intent);
    println!("\nAssumptions & Invariants:");
    explanation.assumptions_and_invariants.iter().for_each(|item| println!("  - {}", item));
    println!("\nCommon Failure Modes:");
    explanation.common_failure_modes.iter().for_each(|item| println!("  - {}", item));
    println!("\nSafest Edits:");
    explanation.safest_edits.iter().for_each(|edit| println!("  {}. {}", edit.rank, edit.description));
    if !explanation.verify_commands.is_empty() {
        println!("\nSuggested Verify Commands:");
        explanation.verify_commands.iter().for_each(|cmd| println!("  - {}", cmd));
    }
    println!("-------------------");

    Ok(())
}
