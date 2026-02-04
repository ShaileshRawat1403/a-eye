use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PersistedExecutionMode {
    Simulated,
    Runtime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PersistedWorkflowStatus {
    Running,
    AwaitingApproval,
    Blocked,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum PersistedShellEvent {
    WorkflowRunStarted {
        run_id: u64,
        template_id: String,
        execution_mode: PersistedExecutionMode,
        policy_tier: String,
        persona_policy: PersistedPersonaPolicy,
    },
    WorkflowStatusChanged {
        run_id: u64,
        status: PersistedWorkflowStatus,
        step_index: usize,
        reason: Option<String>,
    },
    ToolInvocationIssued {
        run_id: u64,
        invocation_id: u64,
        tool_id: String,
    },
    ToolResultRecorded {
        run_id: u64,
        invocation_id: u64,
        tool_id: String,
        status: String,
    },
    ApprovalRequested {
        request_id: String,
        run_id: u64,
        invocation_id: u64,
        tool_id: String,
        risk: String,
        preview: String,
    },
    ApprovalResolved {
        request_id: String,
        run_id: u64,
        decision: String,
    },
    PolicyChanged {
        tier: String,
        source: String,
    },
    PersonaPolicyChanged {
        persona: String,
        policy: PersistedPersonaPolicy,
        source: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedPersonaPolicy {
    pub(crate) tier_ceiling: String,
    pub(crate) explanation_depth: String,
    pub(crate) output_format: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedShellEventRecord {
    pub(crate) seq: u64,
    pub(crate) ts_ms: i64,
    #[serde(flatten)]
    pub(crate) event: PersistedShellEvent,
}

#[derive(Debug)]
pub(crate) struct ShellEventStore {
    path: PathBuf,
    next_seq: u64,
}

impl ShellEventStore {
    pub(crate) fn open(path: PathBuf) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let existing = load_records(path.as_path())?;
        let next_seq = existing
            .iter()
            .map(|record| record.seq)
            .max()
            .map_or(1, |seq| seq.saturating_add(1));
        Ok(Self { path, next_seq })
    }

    pub(crate) fn append(&mut self, event: PersistedShellEvent) -> std::io::Result<u64> {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        let record = PersistedShellEventRecord {
            seq,
            ts_ms: chrono::Utc::now().timestamp_millis(),
            event,
        };
        let line = serde_json::to_string(&record)
            .map_err(|err| std::io::Error::other(format!("serialize: {err}")))?;
        append_line(self.path.as_path(), line.as_str())?;
        Ok(seq)
    }

    pub(crate) fn load(&self) -> std::io::Result<Vec<PersistedShellEventRecord>> {
        load_records(self.path.as_path())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplayedWorkflowRun {
    pub(crate) run_id: u64,
    pub(crate) template_id: String,
    pub(crate) execution_mode: PersistedExecutionMode,
    pub(crate) step_index: usize,
    pub(crate) status: PersistedWorkflowStatus,
    pub(crate) pending_request_id: Option<String>,
    pub(crate) pending_tool_id: Option<String>,
    pub(crate) pending_invocation_id: Option<u64>,
}

pub(crate) fn replay_latest_workflow(
    records: &[PersistedShellEventRecord],
) -> Option<ReplayedWorkflowRun> {
    let mut sorted = records.to_vec();
    sorted.sort_by_key(|record| record.seq);

    let mut latest: Option<ReplayedWorkflowRun> = None;
    for record in sorted {
        match record.event {
            PersistedShellEvent::WorkflowRunStarted {
                run_id,
                template_id,
                execution_mode,
                ..
            } => {
                latest = Some(ReplayedWorkflowRun {
                    run_id,
                    template_id,
                    execution_mode,
                    step_index: 0,
                    status: PersistedWorkflowStatus::Running,
                    pending_request_id: None,
                    pending_tool_id: None,
                    pending_invocation_id: None,
                });
            }
            PersistedShellEvent::WorkflowStatusChanged {
                run_id,
                status,
                step_index,
                ..
            } => {
                if let Some(run) = latest.as_mut()
                    && run.run_id == run_id
                {
                    run.status = status;
                    run.step_index = step_index;
                    if !matches!(status, PersistedWorkflowStatus::AwaitingApproval) {
                        run.pending_request_id = None;
                        run.pending_tool_id = None;
                        run.pending_invocation_id = None;
                    }
                }
            }
            PersistedShellEvent::ToolResultRecorded { run_id, status, .. } => {
                if let Some(run) = latest.as_mut()
                    && run.run_id == run_id
                    && status == "succeeded"
                {
                    run.step_index = run.step_index.saturating_add(1);
                }
            }
            PersistedShellEvent::ApprovalRequested {
                request_id,
                run_id,
                invocation_id,
                tool_id,
                ..
            } => {
                if let Some(run) = latest.as_mut()
                    && run.run_id == run_id
                {
                    run.status = PersistedWorkflowStatus::AwaitingApproval;
                    run.pending_request_id = Some(request_id);
                    run.pending_tool_id = Some(tool_id);
                    run.pending_invocation_id = Some(invocation_id);
                }
            }
            PersistedShellEvent::ApprovalResolved {
                request_id,
                run_id,
                decision,
            } => {
                if let Some(run) = latest.as_mut()
                    && run.run_id == run_id
                    && run.pending_request_id.as_deref() == Some(request_id.as_str())
                {
                    if decision == "approved" {
                        run.status = PersistedWorkflowStatus::Running;
                    } else {
                        run.status = PersistedWorkflowStatus::Blocked;
                    }
                    run.pending_request_id = None;
                    run.pending_tool_id = None;
                    run.pending_invocation_id = None;
                }
            }
            PersistedShellEvent::ToolInvocationIssued { .. }
            | PersistedShellEvent::PolicyChanged { .. }
            | PersistedShellEvent::PersonaPolicyChanged { .. } => {}
        }
    }

    latest
}

fn load_records(path: &Path) -> std::io::Result<Vec<PersistedShellEventRecord>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(record) = serde_json::from_str::<PersistedShellEventRecord>(&line) {
            records.push(record);
        }
    }
    Ok(records)
}

fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    let mut opts = OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::PersistedExecutionMode;
    use super::PersistedPersonaPolicy;
    use super::PersistedShellEvent;
    use super::PersistedWorkflowStatus;
    use super::ShellEventStore;
    use super::replay_latest_workflow;
    use pretty_assertions::assert_eq;

    fn policy() -> PersistedPersonaPolicy {
        PersistedPersonaPolicy {
            tier_ceiling: "balanced".to_string(),
            explanation_depth: "detailed".to_string(),
            output_format: "impact-first".to_string(),
        }
    }

    #[test]
    fn append_records_are_monotonic() {
        let dir = tempdir().expect("tmpdir");
        let path = dir.path().join("events.jsonl");
        let mut store = ShellEventStore::open(path).expect("open");
        let seq1 = store
            .append(PersistedShellEvent::WorkflowRunStarted {
                run_id: 1,
                template_id: "scan_plan_diff_verify".to_string(),
                execution_mode: PersistedExecutionMode::Simulated,
                policy_tier: "balanced".to_string(),
                persona_policy: policy(),
            })
            .expect("append");
        let seq2 = store
            .append(PersistedShellEvent::WorkflowStatusChanged {
                run_id: 1,
                status: PersistedWorkflowStatus::Running,
                step_index: 0,
                reason: None,
            })
            .expect("append");

        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);
        let loaded = store.load().expect("load");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].seq, 1);
        assert_eq!(loaded[1].seq, 2);
    }

    #[test]
    fn replay_workflow_tracks_approval_lifecycle() {
        let records = vec![
            super::PersistedShellEventRecord {
                seq: 1,
                ts_ms: 0,
                event: PersistedShellEvent::WorkflowRunStarted {
                    run_id: 7,
                    template_id: "scan_plan_diff_verify".to_string(),
                    execution_mode: PersistedExecutionMode::Runtime,
                    policy_tier: "balanced".to_string(),
                    persona_policy: policy(),
                },
            },
            super::PersistedShellEventRecord {
                seq: 2,
                ts_ms: 0,
                event: PersistedShellEvent::ApprovalRequested {
                    request_id: "req-1".to_string(),
                    run_id: 7,
                    invocation_id: 3,
                    tool_id: "compute_diff".to_string(),
                    risk: "patch-only".to_string(),
                    preview: "workflow-tool compute_diff".to_string(),
                },
            },
            super::PersistedShellEventRecord {
                seq: 3,
                ts_ms: 0,
                event: PersistedShellEvent::ApprovalResolved {
                    request_id: "req-1".to_string(),
                    run_id: 7,
                    decision: "approved".to_string(),
                },
            },
        ];

        let run = replay_latest_workflow(&records).expect("replay");
        assert_eq!(run.run_id, 7);
        assert_eq!(run.status, PersistedWorkflowStatus::Running);
        assert!(run.pending_request_id.is_none());
    }

    #[test]
    fn replay_tracks_succeeded_results_into_step_index() {
        let records = vec![
            super::PersistedShellEventRecord {
                seq: 1,
                ts_ms: 0,
                event: PersistedShellEvent::WorkflowRunStarted {
                    run_id: 9,
                    template_id: "scan_plan_diff_verify".to_string(),
                    execution_mode: PersistedExecutionMode::Simulated,
                    policy_tier: "strict".to_string(),
                    persona_policy: policy(),
                },
            },
            super::PersistedShellEventRecord {
                seq: 2,
                ts_ms: 0,
                event: PersistedShellEvent::ToolResultRecorded {
                    run_id: 9,
                    invocation_id: 1,
                    tool_id: "scan_repo".to_string(),
                    status: "succeeded".to_string(),
                },
            },
            super::PersistedShellEventRecord {
                seq: 3,
                ts_ms: 0,
                event: PersistedShellEvent::ToolResultRecorded {
                    run_id: 9,
                    invocation_id: 2,
                    tool_id: "generate_plan".to_string(),
                    status: "succeeded".to_string(),
                },
            },
        ];

        let run = replay_latest_workflow(&records).expect("replay");
        assert_eq!(run.step_index, 2);
    }
}
