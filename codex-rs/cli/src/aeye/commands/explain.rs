use crate::aeye::artifacts::Run;
use crate::aeye::artifacts::read_validated_json_artifact;
use crate::aeye::config;
use crate::aeye::explainer;
use crate::aeye::scanner::SystemProfile;
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::PathBuf;

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

pub async fn run(cmd: ExplainCommand) -> Result<()> {
    let repo_root = config::find_repo_root()
        .context("Could not find repository root. Are you in a git repository?")?;

    let run = Run::new(&repo_root)?;
    println!("Starting run: {}", run.id);
    println!("Artifacts will be saved to: {}", run.path.display());

    let file_content = fs::read_to_string(&cmd.path)
        .with_context(|| format!("Failed to read file: {}", cmd.path.display()))?;

    let lines: Vec<&str> = file_content.lines().collect();
    let line_index = cmd.line.saturating_sub(1);
    if line_index >= lines.len() {
        anyhow::bail!(
            "Line number {} is out of bounds for file {}",
            cmd.line,
            cmd.path.display()
        );
    }

    let start = line_index.saturating_sub(3);
    let end = (line_index + 4).min(lines.len());
    let snippet_with_context: String = lines[start..end]
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            let current_line = start + idx;
            let marker = if current_line == line_index { ">" } else { " " };
            format!("{marker} {:4} | {line}\n", current_line + 1)
        })
        .collect();

    let profile_path = repo_root.join(".nlpg/system.json");
    let verify_commands = if profile_path.exists() {
        let profile_content = read_validated_json_artifact(&profile_path, "system.json")?;
        serde_json::from_str::<SystemProfile>(&profile_content)
            .map(|profile| profile.verify_commands)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let target_code_line = lines.get(line_index).copied().unwrap_or_default();
    println!(
        "\nGenerating explanation for: `{}`...",
        target_code_line.trim()
    );
    let explanation =
        explainer::generate_explanation_from_llm(&snippet_with_context, verify_commands).await?;

    run.write_artifact("explain.json", &explanation)?;

    println!("\n--- Explanation ---");
    println!("Construct Type: {}", explanation.construct_type);
    println!("\nIntent:\n  {}", explanation.intent);
    println!("\nAssumptions & Invariants:");
    for item in &explanation.assumptions_and_invariants {
        println!("  - {item}");
    }
    println!("\nCommon Failure Modes:");
    for item in &explanation.common_failure_modes {
        println!("  - {item}");
    }
    println!("\nSafest Edits:");
    for edit in &explanation.safest_edits {
        println!("  {}. {}", edit.rank, edit.description);
    }
    if !explanation.verify_commands.is_empty() {
        println!("\nSuggested Verify Commands:");
        for command in &explanation.verify_commands {
            println!("  - {command}");
        }
    }
    println!("-------------------");

    Ok(())
}
