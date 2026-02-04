#![allow(dead_code)]

use std::path::PathBuf;

use codex_protocol::ThreadId;
use codex_protocol::config_types::Personality;
use codex_protocol::openai_models::ReasoningEffort;

use super::shell_state::ApplyStatus;
use super::shell_state::ApprovalAction;
use super::shell_state::ApprovalDecisionRecord;
use super::shell_state::ApprovalRequestRecord;
use super::shell_state::ApprovalRiskClass;
use super::shell_state::ClearReason;
use super::shell_state::DiffArtifact;
use super::shell_state::ErrorKind;
use super::shell_state::JourneyError;
use super::shell_state::JourneyState;
use super::shell_state::JourneyStep;
use super::shell_state::LogEntry;
use super::shell_state::LogLevel;
use super::shell_state::PlanArtifact;
use super::shell_state::PolicyTier;
use super::shell_state::RiskLevel;
use super::shell_state::SafetyMode;
use super::shell_state::ScanStatus;
use super::shell_state::ShellTab;
use super::shell_state::SystemArtifact;
use super::shell_state::VerifyArtifact;
use super::shell_state::VerifyStatus;

#[derive(Debug, Clone)]
pub(crate) enum ShellAction {
    User(UserAction),
    Runtime(RuntimeAction),
}

#[derive(Debug, Clone)]
pub(crate) enum UserAction {
    ToggleActionPalette,
    CloseOverlay,
    NextTab,
    PrevTab,
    SelectTab(ShellTab),
    NextJourneyStep,
    PrevJourneyStep,
    OverlayMoveUp,
    OverlayMoveDown,
    OverlayQueryInput(char),
    OverlayQueryBackspace,
    OverlayQueryPaste(String),
    OverlaySubmit,
    SelectDiffFile {
        path: String,
    },
    SelectPlanStep {
        id: String,
    },
    SetLogLevelFilter(Option<LogLevel>),
    SetLogSearch(String),
    ClearArtifact {
        which: ClearWhich,
        reason: ClearReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeFlag {
    Scanning,
    Planning,
    Diffing,
    AwaitingApproval,
    Verifying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClearWhich {
    System,
    Plan,
    Diff,
    Verify,
    Logs,
}

#[derive(Debug, Clone)]
pub(crate) enum RuntimeAction {
    SetProjectName(String),
    SetThreadId(Option<ThreadId>),
    SetCwd(Option<PathBuf>),
    SetSafetyMode(SafetyMode),
    SetScanStatus(ScanStatus),
    SetApplyStatus(ApplyStatus),
    SetVerifyStatus(VerifyStatus),
    SetRiskLevel(RiskLevel),
    SetPersonality(Personality),
    SetSkillsEnabledCount(usize),
    SetCollaborationModeLabel(String),
    SetModelSlug(Option<String>),
    SetReasoningEffort(Option<ReasoningEffort>),
    SetTab(ShellTab),
    SetJourney(JourneyStep),
    SetJourneyState(JourneyState),
    SetJourneyError {
        kind: ErrorKind,
        message: String,
    },
    ClearJourneyError,

    SetSystemArtifact(SystemArtifact),
    SetPlanArtifact(PlanArtifact),
    SetDiffArtifact(DiffArtifact),
    SetVerifyArtifact(VerifyArtifact),

    ClearSystemArtifact(ClearReason),
    ClearPlanArtifact(ClearReason),
    ClearDiffArtifact(ClearReason),
    ClearVerifyArtifact(ClearReason),

    SetRuntimeFlag {
        flag: RuntimeFlag,
        active: bool,
        run_id: u64,
    },

    SetJourneyErrorState(Option<JourneyError>),
    SetPolicyTier(PolicyTier),
    AssessPolicyGate {
        run_id: u64,
        action: ApprovalAction,
        risk: ApprovalRiskClass,
        reason: String,
    },
    RequestApproval(ApprovalRequestRecord),
    ResolveApproval(ApprovalDecisionRecord),
    ClearApprovalState(ClearReason),

    AppendStructuredLog(LogEntry),
    ClearLogs(ClearReason),

    // Compatibility actions while app/runtime adapter migrates.
    SetOverview(String),
    SetSystem(String),
    SetPlan(String),
    SetDiff(String),
    SetExplain(String),
    AppendLog(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaletteCommand {
    ContinueInChat,
    OpenPermissions,
    OpenApprovals,
    OpenSkills,
    StartNewSession,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PaletteItem {
    pub(crate) label: &'static str,
    pub(crate) command: PaletteCommand,
}

pub(crate) const PALETTE_ITEMS: [PaletteItem; 6] = [
    PaletteItem {
        label: "Continue in chat",
        command: PaletteCommand::ContinueInChat,
    },
    PaletteItem {
        label: "Open permissions",
        command: PaletteCommand::OpenPermissions,
    },
    PaletteItem {
        label: "Open approvals",
        command: PaletteCommand::OpenApprovals,
    },
    PaletteItem {
        label: "Open skills",
        command: PaletteCommand::OpenSkills,
    },
    PaletteItem {
        label: "Start new session",
        command: PaletteCommand::StartNewSession,
    },
    PaletteItem {
        label: "Quit A-Eye",
        command: PaletteCommand::Quit,
    },
];

pub(crate) fn filtered_palette_indices(query: &str) -> Vec<usize> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return (0..PALETTE_ITEMS.len()).collect();
    }

    PALETTE_ITEMS
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            if item.label.to_ascii_lowercase().contains(&query) {
                Some(idx)
            } else {
                None
            }
        })
        .collect()
}
