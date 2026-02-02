use anyhow::Context;
use anyhow::Result;
use chrono::Utc;
use rand::distr::Alphanumeric;
use rand::distr::SampleString;
use regex_lite::Regex;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const RUNS_DIR: &str = "runs";
const NLPG_DIR: &str = ".nlpg";

#[derive(Debug, Clone)]
pub struct Run {
    pub id: String,
    pub path: PathBuf,
}

impl Run {
    /// Creates a new run and required subdirectories.
    pub fn new(repo_root: &Path) -> Result<Self> {
        let run_id = format!(
            "{}-{}",
            Utc::now().format("%Y%m%d-%H%M%S"),
            Alphanumeric.sample_string(&mut rand::rng(), 6)
        );

        let run_path = repo_root.join(NLPG_DIR).join(RUNS_DIR).join(&run_id);
        fs::create_dir_all(&run_path)
            .with_context(|| format!("Failed to create run directory at {}", run_path.display()))?;

        fs::create_dir(run_path.join("steps"))?;
        fs::create_dir(run_path.join("logs"))?;

        Ok(Self {
            id: run_id,
            path: run_path,
        })
    }

    pub fn write_artifact<T: Serialize>(&self, name: &str, artifact: &T) -> Result<()> {
        let artifact_path = self.path.join(name);
        write_structured_json_artifact(&artifact_path, name, artifact)
    }

    pub fn write_log_file(&self, name: &str, content: &str) -> Result<()> {
        let log_path = self.path.join("logs").join(name);
        let sanitized = redact_secrets(content);
        fs::write(&log_path, sanitized)
            .with_context(|| format!("Failed to write log file to {}", log_path.display()))?;
        Ok(())
    }
}

pub fn read_validated_json_artifact(path: &Path, artifact_name: &str) -> Result<String> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read artifact from {}", path.display()))?;
    let value: Value = serde_json::from_str(&content)
        .with_context(|| format!("Invalid JSON payload for artifact `{artifact_name}`"))?;
    if !value.is_object() {
        anyhow::bail!("`{artifact_name}` must be a JSON object.");
    }
    if artifact_name == "workflow.json" {
        validate_workflow_contract(&value)?;
    }
    Ok(content)
}

pub fn write_redacted_text_file(path: &Path, content: &str) -> Result<()> {
    let sanitized = redact_secrets(content);
    fs::write(path, sanitized)
        .with_context(|| format!("Failed to write artifact to {}", path.display()))?;
    Ok(())
}

pub fn write_structured_json_artifact<T: Serialize>(
    path: &Path,
    artifact_name: &str,
    artifact: &T,
) -> Result<()> {
    let content = serde_json::to_string_pretty(artifact)?;
    let sanitized = redact_secrets(&content);
    validate_artifact_contract(artifact_name, &sanitized)?;
    fs::write(path, sanitized)
        .with_context(|| format!("Failed to write artifact to {}", path.display()))?;
    Ok(())
}

fn validate_artifact_contract(artifact_name: &str, raw_json: &str) -> Result<()> {
    if !artifact_name.ends_with(".json") {
        return Ok(());
    }

    let value: Value = serde_json::from_str(raw_json)
        .with_context(|| format!("Invalid JSON payload for artifact `{artifact_name}`"))?;

    match artifact_name {
        "intent.json" => validate_intent_contract(&value)?,
        "plan.json" => validate_plan_contract(&value)?,
        "system.json" => validate_system_contract(&value)?,
        "explain.json" => validate_explain_contract(&value)?,
        "workflow.json" => validate_workflow_contract(&value)?,
        _ => {}
    }

    Ok(())
}

fn validate_intent_contract(value: &Value) -> Result<()> {
    let obj = expect_object(value, "intent.json")?;
    require_non_empty_string(obj, "id", "intent.json")?;
    require_non_empty_string(obj, "goal", "intent.json")?;
    require_array(obj, "constraints", "intent.json")?;
    require_array(obj, "scope", "intent.json")?;
    require_array(obj, "successCriteria", "intent.json")?;
    require_non_empty_string(obj, "riskTolerance", "intent.json")?;

    let env = require_object(obj, "environment", "intent.json")?;
    require_non_empty_string(env, "os", "intent.environment")?;
    require_non_empty_string(env, "shell", "intent.environment")?;
    Ok(())
}

fn validate_plan_contract(value: &Value) -> Result<()> {
    let obj = expect_object(value, "plan.json")?;
    require_non_empty_string(obj, "title", "plan.json")?;
    require_non_empty_string_array(obj, "steps", "plan.json")?;
    Ok(())
}

fn validate_system_contract(value: &Value) -> Result<()> {
    let obj = expect_object(value, "system.json")?;
    require_non_empty_string(obj, "repoType", "system.json")?;
    require_non_empty_string(obj, "topology", "system.json")?;
    require_array(obj, "frameworks", "system.json")?;
    require_array(obj, "languages", "system.json")?;
    require_array(obj, "entrypoints", "system.json")?;
    require_array(obj, "verifyCommands", "system.json")?;
    require_array(obj, "riskZones", "system.json")?;
    require_array(obj, "writeAllowlist", "system.json")?;
    require_array(obj, "readAllowlist", "system.json")?;
    Ok(())
}

fn validate_explain_contract(value: &Value) -> Result<()> {
    let obj = expect_object(value, "explain.json")?;
    require_non_empty_string(obj, "constructType", "explain.json")?;
    require_non_empty_string(obj, "intent", "explain.json")?;
    require_array(obj, "assumptionsAndInvariants", "explain.json")?;
    require_array(obj, "commonFailureModes", "explain.json")?;
    require_array(obj, "verifyCommands", "explain.json")?;
    require_array(obj, "safestEdits", "explain.json")?;
    Ok(())
}

fn validate_workflow_contract(value: &Value) -> Result<()> {
    let obj = expect_object(value, "workflow.json")?;
    require_non_empty_string(obj, "schemaVersion", "workflow.json")?;
    require_non_empty_string(obj, "recipeName", "workflow.json")?;
    require_non_empty_string(obj, "recipeDescription", "workflow.json")?;
    require_non_empty_string(obj, "goal", "workflow.json")?;
    require_non_empty_string(obj, "status", "workflow.json")?;
    require_non_empty_string(obj, "startedAt", "workflow.json")?;
    require_array(obj, "stepResults", "workflow.json")?;

    if let Some(finished_at) = obj.get("finishedAt")
        && !finished_at.is_null()
        && finished_at
            .as_str()
            .is_none_or(|value| value.trim().is_empty())
    {
        anyhow::bail!("`workflow.json` has invalid `finishedAt`.");
    }

    Ok(())
}

fn expect_object<'a>(value: &'a Value, artifact_name: &str) -> Result<&'a Map<String, Value>> {
    value
        .as_object()
        .with_context(|| format!("`{artifact_name}` must be a JSON object."))
}

fn require_object<'a>(
    obj: &'a Map<String, Value>,
    field: &str,
    artifact_name: &str,
) -> Result<&'a Map<String, Value>> {
    obj.get(field)
        .with_context(|| format!("`{artifact_name}` is missing required field `{field}`."))?
        .as_object()
        .with_context(|| format!("`{artifact_name}` field `{field}` must be an object."))
}

fn require_non_empty_string(
    obj: &Map<String, Value>,
    field: &str,
    artifact_name: &str,
) -> Result<()> {
    let value = obj
        .get(field)
        .with_context(|| format!("`{artifact_name}` is missing required field `{field}`."))?
        .as_str()
        .with_context(|| format!("`{artifact_name}` field `{field}` must be a string."))?;
    if value.trim().is_empty() {
        anyhow::bail!("`{artifact_name}` field `{field}` must not be empty.");
    }
    Ok(())
}

fn require_array(obj: &Map<String, Value>, field: &str, artifact_name: &str) -> Result<()> {
    let _ = obj
        .get(field)
        .with_context(|| format!("`{artifact_name}` is missing required field `{field}`."))?
        .as_array()
        .with_context(|| format!("`{artifact_name}` field `{field}` must be an array."))?;
    Ok(())
}

fn require_non_empty_string_array(
    obj: &Map<String, Value>,
    field: &str,
    artifact_name: &str,
) -> Result<()> {
    let values = obj
        .get(field)
        .with_context(|| format!("`{artifact_name}` is missing required field `{field}`."))?
        .as_array()
        .with_context(|| format!("`{artifact_name}` field `{field}` must be an array."))?;
    if values.is_empty() {
        anyhow::bail!("`{artifact_name}` field `{field}` must not be empty.");
    }
    if values
        .iter()
        .any(|value| value.as_str().is_none_or(|text| text.trim().is_empty()))
    {
        anyhow::bail!("`{artifact_name}` field `{field}` must contain only non-empty strings.");
    }
    Ok(())
}

fn redact_secrets(input: &str) -> String {
    let mut redacted = input.to_string();
    for (pattern, replacement) in [
        (
            r#"(?i)(api[_-]?key|token|secret|password)\s*[:=]\s*(["'])?[A-Za-z0-9_\-\/+=]{8,}\2"#,
            "$1=[REDACTED]",
        ),
        (r"sk-[A-Za-z0-9]{20,}", "[REDACTED_OPENAI_KEY]"),
        (r"AKIA[0-9A-Z]{16}", "[REDACTED_AWS_KEY_ID]"),
        (
            r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----",
            "[REDACTED_PRIVATE_KEY_BLOCK]",
        ),
    ] {
        let Ok(regex) = Regex::new(pattern) else {
            continue;
        };
        redacted = regex.replace_all(&redacted, replacement).into_owned();
    }
    redacted
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn write_artifact_redacts_sensitive_values() {
        let temp = tempdir().unwrap();
        let run = Run::new(temp.path()).unwrap();
        let artifact = json!({
            "id": "intent-1",
            "goal": "Use key sk-123456789012345678901234 in docs",
            "constraints": [],
            "environment": { "os": "macos", "shell": "zsh" },
            "riskTolerance": "low",
            "scope": [],
            "successCriteria": []
        });

        run.write_artifact("intent.json", &artifact).unwrap();

        let content = fs::read_to_string(run.path.join("intent.json")).unwrap();
        assert!(content.contains("[REDACTED_OPENAI_KEY]"));
    }

    #[test]
    fn write_artifact_rejects_invalid_plan_contract() {
        let temp = tempdir().unwrap();
        let run = Run::new(temp.path()).unwrap();
        let artifact = json!({
            "title": "Invalid plan",
            "steps": []
        });

        let err = run.write_artifact("plan.json", &artifact).unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }
}
