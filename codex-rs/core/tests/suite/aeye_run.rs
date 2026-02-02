use assert_cmd::prelude::*;
use predicates::prelude::*;
use pretty_assertions::assert_eq;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn aeye_run_writes_workflow_artifact() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir(repo_root.join(".git")).unwrap();
    fs::create_dir(repo_root.join("recipes")).unwrap();
    fs::write(
        repo_root.join("recipes/safe_patch.yaml"),
        r#"name: Safe patch workflow
description: Scan and produce a plan artifact.
steps:
  - name: scan
    action: system.scan
  - name: plan
    action: llm.plan
    inputs:
      goal: "{{ user.goal }}"
"#,
    )
    .unwrap();

    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("a-eye").unwrap());
    cmd.current_dir(repo_root);
    cmd.arg("run")
        .arg("safe_patch")
        .arg("--goal")
        .arg("improve startup messaging");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created run:"))
        .stdout(predicate::str::contains(
            "Workflow 'Safe patch workflow' completed successfully.",
        ));

    let mut run_entries = fs::read_dir(repo_root.join(".nlpg/runs")).unwrap();
    let run_path = run_entries
        .next()
        .expect("run should be created")
        .unwrap()
        .path();
    assert!(run_entries.next().is_none());

    let workflow_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(run_path.join("workflow.json")).unwrap()).unwrap();

    assert_eq!(workflow_json["schemaVersion"], "1.0");
    assert_eq!(workflow_json["recipeName"], "Safe patch workflow");
    assert_eq!(
        workflow_json["recipeDescription"],
        "Scan and produce a plan artifact."
    );
    assert_eq!(workflow_json["goal"], "improve startup messaging");
    assert_eq!(workflow_json["status"], "completed");
    assert!(workflow_json["startedAt"].as_str().is_some());
    assert!(workflow_json["finishedAt"].as_str().is_some());
    assert_eq!(workflow_json["error"], serde_json::Value::Null);

    let step_results = workflow_json["stepResults"].as_array().unwrap();
    assert_eq!(step_results.len(), 2);
    assert_eq!(step_results[0]["name"], "scan");
    assert_eq!(step_results[0]["action"], "system.scan");
    assert_eq!(step_results[0]["status"], "completed");
    assert_eq!(step_results[1]["name"], "plan");
    assert_eq!(step_results[1]["action"], "llm.plan");
    assert_eq!(step_results[1]["status"], "completed");
}
