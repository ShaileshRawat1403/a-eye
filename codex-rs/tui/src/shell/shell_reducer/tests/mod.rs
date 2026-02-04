use codex_protocol::config_types::Personality;
use pretty_assertions::assert_eq;

pub(super) use super::derive_journey;
pub(super) use super::reduce;
pub(super) use crate::shell::shell_effects::UiEffect;
pub(super) use crate::shell::shell_state::ARTIFACT_SCHEMA_V1;
pub(super) use crate::shell::shell_state::ApprovalAction;
pub(super) use crate::shell::shell_state::ApprovalDecisionKind;
pub(super) use crate::shell::shell_state::ApprovalDecisionRecord;
pub(super) use crate::shell::shell_state::ApprovalRequestRecord;
pub(super) use crate::shell::shell_state::ApprovalRiskClass;
pub(super) use crate::shell::shell_state::ArtifactError;
pub(super) use crate::shell::shell_state::ClearReason;
pub(super) use crate::shell::shell_state::DiffArtifact;
pub(super) use crate::shell::shell_state::DiffFile;
pub(super) use crate::shell::shell_state::DiffFileStatus;
pub(super) use crate::shell::shell_state::ErrorKind;
pub(super) use crate::shell::shell_state::ExplanationDepth;
pub(super) use crate::shell::shell_state::JourneyError;
pub(super) use crate::shell::shell_state::JourneyProjection;
pub(super) use crate::shell::shell_state::JourneyState;
pub(super) use crate::shell::shell_state::JourneyStep;
pub(super) use crate::shell::shell_state::LogBuffer;
pub(super) use crate::shell::shell_state::LogEntry;
pub(super) use crate::shell::shell_state::LogLevel;
pub(super) use crate::shell::shell_state::LogSource;
pub(super) use crate::shell::shell_state::PersonaOutputFormat;
pub(super) use crate::shell::shell_state::PlanArtifact;
pub(super) use crate::shell::shell_state::PlanStep;
pub(super) use crate::shell::shell_state::PolicyTier;
pub(super) use crate::shell::shell_state::ShellOverlay;
pub(super) use crate::shell::shell_state::ShellState;
pub(super) use crate::shell::shell_state::ShellTab;
pub(super) use crate::shell::shell_state::StepStatus;
pub(super) use crate::shell::shell_state::SystemArtifact;
pub(super) use crate::shell::shell_state::VerifyArtifact;
pub(super) use crate::shell::shell_state::VerifyOverall;
pub(super) use crate::shell::shell_state::policy_requirement_for_risk;
pub(super) use crate::shell::ui_action::RuntimeAction;
pub(super) use crate::shell::ui_action::RuntimeFlag;
pub(super) use crate::shell::ui_action::ShellAction;
pub(super) use crate::shell::ui_action::UserAction;

mod approvals_policy;
mod artifact_guards;
mod invariants;
mod log_buffer;
mod persona_projection;
mod projection_matrix;
mod selection_reconcile;

fn state() -> ShellState {
    ShellState::new("project".to_string(), Personality::Friendly)
}

fn system_artifact(run_id: u64, artifact_id: u64, summary: &str) -> SystemArtifact {
    SystemArtifact {
        schema_version: ARTIFACT_SCHEMA_V1,
        run_id,
        artifact_id,
        repo_root: "/repo".to_string(),
        detected_stack: Vec::new(),
        entrypoints: Vec::new(),
        risk_flags: Vec::new(),
        summary: summary.to_string(),
        error: None,
    }
}

fn plan_artifact(run_id: u64, artifact_id: u64, steps: Vec<PlanStep>) -> PlanArtifact {
    PlanArtifact {
        schema_version: ARTIFACT_SCHEMA_V1,
        run_id,
        artifact_id,
        title: "Plan".to_string(),
        steps,
        assumptions: Vec::new(),
        error: None,
    }
}

fn diff_artifact(run_id: u64, artifact_id: u64, files: Vec<DiffFile>) -> DiffArtifact {
    DiffArtifact {
        schema_version: ARTIFACT_SCHEMA_V1,
        run_id,
        artifact_id,
        files,
        summary: "diff".to_string(),
        error: None,
    }
}

fn verify_artifact(run_id: u64, artifact_id: u64, overall: VerifyOverall) -> VerifyArtifact {
    VerifyArtifact {
        schema_version: ARTIFACT_SCHEMA_V1,
        run_id,
        artifact_id,
        checks: Vec::new(),
        overall,
        error: None,
    }
}

fn diff_file(path: &str, status: DiffFileStatus) -> DiffFile {
    DiffFile {
        path: path.to_string(),
        status,
        hunks: Vec::new(),
    }
}

fn plan_step(id: &str, status: StepStatus) -> PlanStep {
    PlanStep {
        id: id.to_string(),
        label: id.to_string(),
        status,
    }
}

fn run_runtime(state: &mut ShellState, action: RuntimeAction) {
    let effects = reduce(state, ShellAction::Runtime(action));
    assert!(effects.is_empty());
}

fn assert_projection_sync(state: &ShellState) {
    let JourneyProjection {
        state: projected_state,
        step,
        active_run_id,
    } = derive_journey(
        &state.artifacts,
        &state.runtime_flags,
        &state.approval,
        state.journey_status.error.as_ref(),
    );
    assert_eq!(state.journey_status.state, projected_state);
    assert_eq!(state.journey_status.step, step);
    assert_eq!(state.journey_status.active_run_id, active_run_id);
    assert_eq!(state.routing.journey, step);
}
