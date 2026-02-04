#![allow(dead_code)]

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use codex_protocol::ThreadId;
use codex_protocol::config_types::Personality;
use codex_protocol::openai_models::ReasoningEffort;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SchemaVersion(pub(crate) u16);

pub(crate) const ARTIFACT_SCHEMA_V1: SchemaVersion = SchemaVersion(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClearReason {
    SessionReset,
    UserRequest,
    Superseded,
    InvalidatedByNewRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellTab {
    Overview,
    System,
    Plan,
    Diff,
    Explain,
    Logs,
}

impl ShellTab {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Overview => Self::System,
            Self::System => Self::Plan,
            Self::Plan => Self::Diff,
            Self::Diff => Self::Explain,
            Self::Explain => Self::Logs,
            Self::Logs => Self::Overview,
        }
    }

    pub(crate) fn prev(self) -> Self {
        match self {
            Self::Overview => Self::Logs,
            Self::System => Self::Overview,
            Self::Plan => Self::System,
            Self::Diff => Self::Plan,
            Self::Explain => Self::Diff,
            Self::Logs => Self::Explain,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::System => "System",
            Self::Plan => "Plan",
            Self::Diff => "Diff",
            Self::Explain => "Explain",
            Self::Logs => "Logs",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JourneyStep {
    Idea,
    Understand,
    Plan,
    Preview,
    Approve,
    Verify,
    Learn,
}

impl JourneyStep {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Idea => "Idea",
            Self::Understand => "Understand system",
            Self::Plan => "Plan change",
            Self::Preview => "Preview change",
            Self::Approve => "Approve",
            Self::Verify => "Verify",
            Self::Learn => "Learn",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JourneyState {
    Idle,
    Scanning,
    Planning,
    Diffing,
    ReviewReady,
    AwaitingApproval,
    Verifying,
    Completed,
    Failed,
}

impl JourneyState {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Scanning => "Scanning",
            Self::Planning => "Planning",
            Self::Diffing => "Diffing",
            Self::ReviewReady => "Review ready",
            Self::AwaitingApproval => "Awaiting approval",
            Self::Verifying => "Verifying",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    UserInput,
    Runtime,
    External,
    Unknown,
}

impl ErrorKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::UserInput => "user-input",
            Self::Runtime => "runtime",
            Self::External => "external",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JourneyError {
    pub(crate) kind: ErrorKind,
    pub(crate) message: Arc<str>,
    pub(crate) run_id: u64,
}

impl JourneyError {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<Arc<str>>, run_id: u64) -> Self {
        Self {
            kind,
            message: message.into(),
            run_id,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct JourneyStatus {
    pub(crate) state: JourneyState,
    pub(crate) step: JourneyStep,
    pub(crate) error: Option<JourneyError>,
    pub(crate) active_run_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SafetyMode {
    Safe,
    Supervised,
    FullAccess,
}

impl SafetyMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Supervised => "Supervised",
            Self::FullAccess => "Full access",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanStatus {
    Unknown,
    Running,
    Ok,
    Warn,
    Fail,
}

impl ScanStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Running => "Running",
            Self::Ok => "Done",
            Self::Warn => "Warn",
            Self::Fail => "Fail",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApplyStatus {
    NotApplied,
    Applied,
}

impl ApplyStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::NotApplied => "Not applied",
            Self::Applied => "Applied",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerifyStatus {
    NotRun,
    Pass,
    Fail,
}

impl VerifyStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::NotRun => "Not run",
            Self::Pass => "Pass",
            Self::Fail => "Fail",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RiskLevel {
    Low,
    Medium,
    High,
}

impl RiskLevel {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PolicyTier {
    Strict,
    Balanced,
    Permissive,
}

impl PolicyTier {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Balanced => "balanced",
            Self::Permissive => "permissive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApprovalAction {
    Read,
    Patch,
    Execute,
    Elicitation,
}

impl ApprovalAction {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Patch => "patch",
            Self::Execute => "execute",
            Self::Elicitation => "elicitation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApprovalRiskClass {
    ReadOnly,
    PatchOnly,
    Execution,
    Destructive,
}

impl ApprovalRiskClass {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::PatchOnly => "patch-only",
            Self::Execution => "execution",
            Self::Destructive => "destructive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApprovalGateRequirement {
    Allow,
    RequireApproval,
    Deny,
}

impl ApprovalGateRequirement {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::RequireApproval => "require-approval",
            Self::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApprovalDecisionKind {
    Approved,
    Denied,
}

impl ApprovalDecisionKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApprovalRequestRecord {
    pub(crate) request_id: String,
    pub(crate) run_id: u64,
    pub(crate) action: ApprovalAction,
    pub(crate) risk: ApprovalRiskClass,
    pub(crate) reason: Arc<str>,
    pub(crate) preview: Arc<str>,
    pub(crate) created_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApprovalDecisionRecord {
    pub(crate) request_id: String,
    pub(crate) run_id: u64,
    pub(crate) action: ApprovalAction,
    pub(crate) decision: ApprovalDecisionKind,
    pub(crate) timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingApproval {
    pub(crate) request: ApprovalRequestRecord,
    pub(crate) sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PolicyGateState {
    pub(crate) run_id: u64,
    pub(crate) action: ApprovalAction,
    pub(crate) risk: ApprovalRiskClass,
    pub(crate) requirement: ApprovalGateRequirement,
    pub(crate) reason: Arc<str>,
}

#[derive(Debug, Clone)]
pub(crate) struct ApprovalState {
    pub(crate) policy_tier: PolicyTier,
    pub(crate) pending: Option<PendingApproval>,
    pub(crate) last_decision: Option<ApprovalDecisionRecord>,
    pub(crate) last_gate: Option<PolicyGateState>,
    pub(crate) next_request_seq: u64,
}

impl Default for ApprovalState {
    fn default() -> Self {
        Self {
            policy_tier: PolicyTier::Balanced,
            pending: None,
            last_decision: None,
            last_gate: None,
            next_request_seq: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShellOverlay {
    None,
    ActionPalette { selected: usize, query: Arc<str> },
    ConfirmQuit,
}

#[derive(Debug, Clone)]
pub(crate) struct ShellHeader {
    pub(crate) project_name: Arc<str>,
    pub(crate) safety_mode: SafetyMode,
    pub(crate) scan: ScanStatus,
    pub(crate) apply: ApplyStatus,
    pub(crate) verify: VerifyStatus,
    pub(crate) risk: RiskLevel,
}

#[derive(Debug, Clone)]
pub(crate) struct ShellRouting {
    pub(crate) journey: JourneyStep,
    pub(crate) tab: ShellTab,
}

#[derive(Debug, Clone)]
pub(crate) struct ShellInteraction {
    pub(crate) overlay: ShellOverlay,
    pub(crate) focus_in_chat: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct SubjectMatterState {
    pub(crate) personality: Personality,
    pub(crate) persona_policy_defaults: PersonaPolicy,
    pub(crate) persona_policy_overrides: PersonaPolicyOverrides,
    pub(crate) persona_policy: PersonaPolicy,
    pub(crate) skills_enabled_count: usize,
    pub(crate) collaboration_mode_label: Arc<str>,
    pub(crate) model_slug: Option<Arc<str>>,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PersonaPolicyOverrides {
    pub(crate) tier_ceiling: Option<PolicyTier>,
    pub(crate) explanation_depth: Option<ExplanationDepth>,
    pub(crate) output_format: Option<PersonaOutputFormat>,
}

impl PersonaPolicyOverrides {
    pub(crate) fn is_empty(self) -> bool {
        self.tier_ceiling.is_none()
            && self.explanation_depth.is_none()
            && self.output_format.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExplanationDepth {
    Brief,
    Standard,
    Detailed,
}

impl ExplanationDepth {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Brief => "brief",
            Self::Standard => "standard",
            Self::Detailed => "detailed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PersonaOutputFormat {
    ImpactFirst,
    TechnicalFirst,
}

impl PersonaOutputFormat {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::ImpactFirst => "impact-first",
            Self::TechnicalFirst => "technical-first",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PersonaPolicy {
    pub(crate) tier_ceiling: PolicyTier,
    pub(crate) explanation_depth: ExplanationDepth,
    pub(crate) output_format: PersonaOutputFormat,
    pub(crate) tab_order: &'static [ShellTab],
    pub(crate) visible_tools: &'static [&'static str],
}

#[derive(Debug, Clone)]
pub(crate) struct ArtifactError {
    pub(crate) kind: ErrorKind,
    pub(crate) message: Arc<str>,
}

#[derive(Debug, Clone)]
pub(crate) struct SystemArtifact {
    pub(crate) schema_version: SchemaVersion,
    pub(crate) run_id: u64,
    pub(crate) artifact_id: u64,
    pub(crate) repo_root: String,
    pub(crate) detected_stack: Vec<String>,
    pub(crate) entrypoints: Vec<String>,
    pub(crate) risk_flags: Vec<String>,
    pub(crate) summary: String,
    pub(crate) error: Option<ArtifactError>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlanStep {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) status: StepStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepStatus {
    Pending,
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone)]
pub(crate) struct PlanArtifact {
    pub(crate) schema_version: SchemaVersion,
    pub(crate) run_id: u64,
    pub(crate) artifact_id: u64,
    pub(crate) title: String,
    pub(crate) steps: Vec<PlanStep>,
    pub(crate) assumptions: Vec<String>,
    pub(crate) error: Option<ArtifactError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiffFileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone)]
pub(crate) struct DiffLine {
    pub(crate) kind: DiffLineKind,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiffLineKind {
    Context,
    Add,
    Remove,
}

#[derive(Debug, Clone)]
pub(crate) struct DiffHunk {
    pub(crate) header: String,
    pub(crate) lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub(crate) struct DiffFile {
    pub(crate) path: String,
    pub(crate) status: DiffFileStatus,
    pub(crate) hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone)]
pub(crate) struct DiffArtifact {
    pub(crate) schema_version: SchemaVersion,
    pub(crate) run_id: u64,
    pub(crate) artifact_id: u64,
    pub(crate) files: Vec<DiffFile>,
    pub(crate) summary: String,
    pub(crate) error: Option<ArtifactError>,
}

#[derive(Debug, Clone)]
pub(crate) struct VerifyCheck {
    pub(crate) name: String,
    pub(crate) status: VerifyCheckStatus,
    pub(crate) details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerifyCheckStatus {
    Pending,
    Running,
    Pass,
    Fail,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerifyOverall {
    Unknown,
    Passing,
    Failing,
}

#[derive(Debug, Clone)]
pub(crate) struct VerifyArtifact {
    pub(crate) schema_version: SchemaVersion,
    pub(crate) run_id: u64,
    pub(crate) artifact_id: u64,
    pub(crate) checks: Vec<VerifyCheck>,
    pub(crate) overall: VerifyOverall,
    pub(crate) error: Option<ArtifactError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogSource {
    App,
    Runtime,
    Shell,
}

#[derive(Debug, Clone)]
pub(crate) struct LogEntry {
    pub(crate) seq: u64,
    pub(crate) level: LogLevel,
    pub(crate) ts_ms: Option<u64>,
    pub(crate) source: LogSource,
    pub(crate) context: Option<String>,
    pub(crate) message: String,
    pub(crate) run_id: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct LogBuffer {
    cap: usize,
    next_seq: u64,
    buf: VecDeque<LogEntry>,
}

impl LogBuffer {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            cap,
            next_seq: 1,
            buf: VecDeque::with_capacity(cap),
        }
    }

    pub(crate) fn append(&mut self, mut entry: LogEntry) {
        entry.seq = self.next_seq;
        self.next_seq += 1;

        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(entry);
    }

    pub(crate) fn clear(&mut self) {
        self.buf.clear();
        self.next_seq = 1;
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &LogEntry> {
        self.buf.iter()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ShellArtifacts {
    pub(crate) schema_version: SchemaVersion,
    pub(crate) system: Option<SystemArtifact>,
    pub(crate) plan: Option<PlanArtifact>,
    pub(crate) diff: Option<DiffArtifact>,
    pub(crate) verify: Option<VerifyArtifact>,
    pub(crate) logs: LogBuffer,
}

impl Default for ShellArtifacts {
    fn default() -> Self {
        Self {
            schema_version: ARTIFACT_SCHEMA_V1,
            system: None,
            plan: None,
            diff: None,
            verify: None,
            logs: LogBuffer::new(2_000),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RuntimeFlagState {
    pub(crate) active: bool,
    pub(crate) run_id: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeFlags {
    pub(crate) scanning: RuntimeFlagState,
    pub(crate) planning: RuntimeFlagState,
    pub(crate) diffing: RuntimeFlagState,
    pub(crate) awaiting_approval: RuntimeFlagState,
    pub(crate) verifying: RuntimeFlagState,
    pub(crate) next_run_id: u64,
}

impl Default for RuntimeFlags {
    fn default() -> Self {
        Self {
            scanning: RuntimeFlagState::default(),
            planning: RuntimeFlagState::default(),
            diffing: RuntimeFlagState::default(),
            awaiting_approval: RuntimeFlagState::default(),
            verifying: RuntimeFlagState::default(),
            next_run_id: 1,
        }
    }
}

impl RuntimeFlags {
    pub(crate) fn clear_all(&mut self) {
        self.scanning.active = false;
        self.planning.active = false;
        self.diffing.active = false;
        self.awaiting_approval.active = false;
        self.verifying.active = false;
    }

    pub(crate) fn current_active_run_id(&self) -> u64 {
        [
            self.scanning,
            self.planning,
            self.diffing,
            self.awaiting_approval,
            self.verifying,
        ]
        .into_iter()
        .filter(|flag| flag.active)
        .map(|flag| flag.run_id)
        .max()
        .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ShellSelection {
    pub(crate) selected_diff_file: Option<String>,
    pub(crate) selected_plan_step: Option<String>,
    pub(crate) log_level_filter: Option<LogLevel>,
    pub(crate) log_search: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ShellState {
    pub(crate) header: ShellHeader,
    pub(crate) routing: ShellRouting,
    pub(crate) journey_status: JourneyStatus,
    pub(crate) interaction: ShellInteraction,
    pub(crate) sm: SubjectMatterState,
    pub(crate) artifacts: ShellArtifacts,
    pub(crate) runtime_flags: RuntimeFlags,
    pub(crate) approval: ApprovalState,
    pub(crate) selection: ShellSelection,
    pub(crate) thread_id: Option<ThreadId>,
    pub(crate) cwd: Option<PathBuf>,
}

const FRIENDLY_VISIBLE_TOOLS: &[&str] = &["scan_repo", "generate_plan", "verify"];
const PRAGMATIC_VISIBLE_TOOLS: &[&str] = &["scan_repo", "generate_plan", "compute_diff", "verify"];
const FRIENDLY_TAB_ORDER: &[ShellTab] = &[
    ShellTab::Overview,
    ShellTab::Plan,
    ShellTab::Explain,
    ShellTab::Diff,
    ShellTab::Logs,
    ShellTab::System,
];
const PRAGMATIC_TAB_ORDER: &[ShellTab] = &[
    ShellTab::Diff,
    ShellTab::Logs,
    ShellTab::Plan,
    ShellTab::System,
    ShellTab::Explain,
    ShellTab::Overview,
];

pub(crate) fn persona_policy_for(personality: Personality) -> PersonaPolicy {
    match personality {
        Personality::Friendly => PersonaPolicy {
            tier_ceiling: PolicyTier::Balanced,
            explanation_depth: ExplanationDepth::Detailed,
            output_format: PersonaOutputFormat::ImpactFirst,
            tab_order: FRIENDLY_TAB_ORDER,
            visible_tools: FRIENDLY_VISIBLE_TOOLS,
        },
        Personality::Pragmatic => PersonaPolicy {
            tier_ceiling: PolicyTier::Permissive,
            explanation_depth: ExplanationDepth::Brief,
            output_format: PersonaOutputFormat::TechnicalFirst,
            tab_order: PRAGMATIC_TAB_ORDER,
            visible_tools: PRAGMATIC_VISIBLE_TOOLS,
        },
    }
}

pub(crate) fn apply_persona_policy_overrides(
    defaults: PersonaPolicy,
    overrides: PersonaPolicyOverrides,
) -> PersonaPolicy {
    PersonaPolicy {
        tier_ceiling: overrides.tier_ceiling.unwrap_or(defaults.tier_ceiling),
        explanation_depth: overrides
            .explanation_depth
            .unwrap_or(defaults.explanation_depth),
        output_format: overrides.output_format.unwrap_or(defaults.output_format),
        tab_order: defaults.tab_order,
        visible_tools: defaults.visible_tools,
    }
}

impl ShellState {
    pub(crate) fn new(project_name: String, personality: Personality) -> Self {
        let persona_policy_defaults = persona_policy_for(personality);
        let persona_policy_overrides = PersonaPolicyOverrides::default();
        Self {
            header: ShellHeader {
                project_name: project_name.into(),
                safety_mode: SafetyMode::Safe,
                scan: ScanStatus::Unknown,
                apply: ApplyStatus::NotApplied,
                verify: VerifyStatus::NotRun,
                risk: RiskLevel::Low,
            },
            routing: ShellRouting {
                journey: JourneyStep::Idea,
                tab: persona_policy_defaults.tab_order[0],
            },
            journey_status: JourneyStatus {
                state: JourneyState::Idle,
                step: JourneyStep::Idea,
                error: None,
                active_run_id: 0,
            },
            interaction: ShellInteraction {
                overlay: ShellOverlay::None,
                focus_in_chat: true,
            },
            sm: SubjectMatterState {
                personality,
                persona_policy_defaults,
                persona_policy_overrides,
                persona_policy: apply_persona_policy_overrides(
                    persona_policy_defaults,
                    persona_policy_overrides,
                ),
                skills_enabled_count: 0,
                collaboration_mode_label: "code".into(),
                model_slug: None,
                reasoning_effort: None,
            },
            artifacts: ShellArtifacts::default(),
            runtime_flags: RuntimeFlags::default(),
            approval: ApprovalState::default(),
            selection: ShellSelection::default(),
            thread_id: None,
            cwd: None,
        }
    }

    pub(crate) fn current_run_id(&self) -> u64 {
        let artifact_run_id = [
            self.artifacts.system.as_ref().map(|a| a.run_id),
            self.artifacts.plan.as_ref().map(|a| a.run_id),
            self.artifacts.diff.as_ref().map(|a| a.run_id),
            self.artifacts.verify.as_ref().map(|a| a.run_id),
        ]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(0);

        self.runtime_flags
            .current_active_run_id()
            .max(artifact_run_id)
            .max(
                self.approval
                    .pending
                    .as_ref()
                    .map(|pending| pending.request.run_id)
                    .unwrap_or(0),
            )
            .max(
                self.approval
                    .last_decision
                    .as_ref()
                    .map(|decision| decision.run_id)
                    .unwrap_or(0),
            )
            .max(self.journey_status.active_run_id)
    }

    pub(crate) fn ordered_tabs(&self) -> &'static [ShellTab] {
        self.sm.persona_policy.tab_order
    }

    pub(crate) fn next_tab(&self) -> ShellTab {
        next_tab_from(self.routing.tab, self.ordered_tabs())
    }

    pub(crate) fn prev_tab(&self) -> ShellTab {
        prev_tab_from(self.routing.tab, self.ordered_tabs())
    }
}

fn next_tab_from(current: ShellTab, order: &[ShellTab]) -> ShellTab {
    if order.is_empty() {
        return current;
    }

    if let Some((idx, _)) = order.iter().enumerate().find(|(_, tab)| **tab == current) {
        return order[(idx + 1) % order.len()];
    }

    order[0]
}

fn prev_tab_from(current: ShellTab, order: &[ShellTab]) -> ShellTab {
    if order.is_empty() {
        return current;
    }

    if let Some((idx, _)) = order.iter().enumerate().find(|(_, tab)| **tab == current) {
        if idx == 0 {
            return order[order.len().saturating_sub(1)];
        }
        return order[idx - 1];
    }

    order[0]
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct JourneyProjection {
    pub(crate) state: JourneyState,
    pub(crate) step: JourneyStep,
    pub(crate) active_run_id: u64,
}

pub(crate) fn derive_journey(
    artifacts: &ShellArtifacts,
    flags: &RuntimeFlags,
    approval: &ApprovalState,
    journey_error: Option<&JourneyError>,
) -> JourneyProjection {
    let active_run_id = [
        flags.scanning,
        flags.planning,
        flags.diffing,
        flags.awaiting_approval,
        flags.verifying,
    ]
    .into_iter()
    .filter(|flag| flag.active)
    .map(|flag| flag.run_id)
    .chain(
        [
            artifacts.system.as_ref().map(|a| a.run_id),
            artifacts.plan.as_ref().map(|a| a.run_id),
            artifacts.diff.as_ref().map(|a| a.run_id),
            artifacts.verify.as_ref().map(|a| a.run_id),
            approval
                .pending
                .as_ref()
                .map(|pending| pending.request.run_id),
        ]
        .into_iter()
        .flatten(),
    )
    .max()
    .unwrap_or(0);

    if let Some(err) = journey_error
        && err.run_id == active_run_id
    {
        return JourneyProjection {
            state: JourneyState::Failed,
            step: JourneyStep::Learn,
            active_run_id,
        };
    }

    if approval
        .pending
        .as_ref()
        .is_some_and(|pending| pending.request.run_id == active_run_id)
        || (flags.awaiting_approval.active && flags.awaiting_approval.run_id == active_run_id)
    {
        return JourneyProjection {
            state: JourneyState::AwaitingApproval,
            step: JourneyStep::Approve,
            active_run_id,
        };
    }

    if flags.verifying.active && flags.verifying.run_id == active_run_id {
        return JourneyProjection {
            state: JourneyState::Verifying,
            step: JourneyStep::Verify,
            active_run_id,
        };
    }

    if flags.diffing.active && flags.diffing.run_id == active_run_id {
        return JourneyProjection {
            state: JourneyState::Diffing,
            step: JourneyStep::Preview,
            active_run_id,
        };
    }

    if flags.planning.active && flags.planning.run_id == active_run_id {
        return JourneyProjection {
            state: JourneyState::Planning,
            step: JourneyStep::Plan,
            active_run_id,
        };
    }

    if flags.scanning.active && flags.scanning.run_id == active_run_id {
        return JourneyProjection {
            state: JourneyState::Scanning,
            step: JourneyStep::Understand,
            active_run_id,
        };
    }

    if let Some(verify) = artifacts.verify.as_ref()
        && verify.run_id == active_run_id
        && verify.overall == VerifyOverall::Passing
    {
        return JourneyProjection {
            state: JourneyState::Completed,
            step: JourneyStep::Learn,
            active_run_id,
        };
    }

    if let Some(diff) = artifacts.diff.as_ref()
        && diff.run_id == active_run_id
    {
        return JourneyProjection {
            state: JourneyState::ReviewReady,
            step: JourneyStep::Preview,
            active_run_id,
        };
    }

    JourneyProjection {
        state: JourneyState::Idle,
        step: JourneyStep::Idea,
        active_run_id,
    }
}

pub(crate) fn artifact_is_newer(
    new_run_id: u64,
    new_artifact_id: u64,
    current: Option<(u64, u64)>,
) -> bool {
    match current {
        None => true,
        Some((run_id, artifact_id)) => {
            new_run_id > run_id || (new_run_id == run_id && new_artifact_id >= artifact_id)
        }
    }
}

pub(crate) fn policy_requirement_for_risk(
    tier: PolicyTier,
    risk: ApprovalRiskClass,
) -> ApprovalGateRequirement {
    match tier {
        PolicyTier::Strict => match risk {
            ApprovalRiskClass::ReadOnly => ApprovalGateRequirement::Allow,
            ApprovalRiskClass::PatchOnly | ApprovalRiskClass::Execution => {
                ApprovalGateRequirement::RequireApproval
            }
            ApprovalRiskClass::Destructive => ApprovalGateRequirement::Deny,
        },
        PolicyTier::Balanced => match risk {
            ApprovalRiskClass::ReadOnly | ApprovalRiskClass::PatchOnly => {
                ApprovalGateRequirement::Allow
            }
            ApprovalRiskClass::Execution | ApprovalRiskClass::Destructive => {
                ApprovalGateRequirement::RequireApproval
            }
        },
        PolicyTier::Permissive => match risk {
            ApprovalRiskClass::Destructive => ApprovalGateRequirement::RequireApproval,
            ApprovalRiskClass::ReadOnly
            | ApprovalRiskClass::PatchOnly
            | ApprovalRiskClass::Execution => ApprovalGateRequirement::Allow,
        },
    }
}
