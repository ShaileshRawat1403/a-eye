use super::shell_state::ApprovalRiskClass;
use super::shell_state::PolicyTier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ToolId {
    ScanRepo,
    GeneratePlan,
    ComputeDiff,
    Verify,
}

impl ToolId {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ScanRepo => "scan_repo",
            Self::GeneratePlan => "generate_plan",
            Self::ComputeDiff => "compute_diff",
            Self::Verify => "verify",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolInputSpec {
    None,
    Query,
    Plan,
    Patch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactKind {
    System,
    Plan,
    Diff,
    Verify,
    Logs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ToolOutputSpec {
    pub(crate) emits: &'static [ArtifactKind],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ToolSpec {
    pub(crate) id: ToolId,
    pub(crate) title: &'static str,
    pub(crate) description: &'static str,
    pub(crate) risk_class: ApprovalRiskClass,
    pub(crate) min_tier: PolicyTier,
    pub(crate) inputs: ToolInputSpec,
    pub(crate) outputs: ToolOutputSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ToolInvocation {
    pub(crate) run_id: u64,
    pub(crate) invocation_id: u64,
    pub(crate) tool_id: ToolId,
    pub(crate) requested_tier: PolicyTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolInvocationStatus {
    Succeeded,
    Failed,
    #[allow(dead_code)]
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolResult {
    pub(crate) run_id: u64,
    pub(crate) invocation_id: u64,
    pub(crate) tool_id: ToolId,
    pub(crate) status: ToolInvocationStatus,
    pub(crate) artifacts_emitted: Vec<ArtifactKind>,
    pub(crate) logs: Vec<String>,
}

pub(crate) struct ToolRegistry;

const TOOL_SPECS: [ToolSpec; 4] = [
    ToolSpec {
        id: ToolId::ScanRepo,
        title: "Scan Repository",
        description: "Scan the workspace and emit system artifact signals.",
        risk_class: ApprovalRiskClass::ReadOnly,
        min_tier: PolicyTier::Strict,
        inputs: ToolInputSpec::Query,
        outputs: ToolOutputSpec {
            emits: &[ArtifactKind::System, ArtifactKind::Logs],
        },
    },
    ToolSpec {
        id: ToolId::GeneratePlan,
        title: "Generate Plan",
        description: "Generate a structured implementation plan from context.",
        risk_class: ApprovalRiskClass::ReadOnly,
        min_tier: PolicyTier::Strict,
        inputs: ToolInputSpec::Plan,
        outputs: ToolOutputSpec {
            emits: &[ArtifactKind::Plan, ArtifactKind::Logs],
        },
    },
    ToolSpec {
        id: ToolId::ComputeDiff,
        title: "Compute Diff",
        description: "Compute a patch preview and emit diff artifact data.",
        risk_class: ApprovalRiskClass::PatchOnly,
        min_tier: PolicyTier::Balanced,
        inputs: ToolInputSpec::Patch,
        outputs: ToolOutputSpec {
            emits: &[ArtifactKind::Diff, ArtifactKind::Logs],
        },
    },
    ToolSpec {
        id: ToolId::Verify,
        title: "Verify",
        description: "Run verification checks and emit verify artifact data.",
        risk_class: ApprovalRiskClass::Execution,
        min_tier: PolicyTier::Balanced,
        inputs: ToolInputSpec::None,
        outputs: ToolOutputSpec {
            emits: &[ArtifactKind::Verify, ArtifactKind::Logs],
        },
    },
];

impl ToolRegistry {
    #[allow(dead_code)]
    pub(crate) fn list() -> &'static [ToolSpec] {
        &TOOL_SPECS
    }

    pub(crate) fn get(id: ToolId) -> &'static ToolSpec {
        match id {
            ToolId::ScanRepo => &TOOL_SPECS[0],
            ToolId::GeneratePlan => &TOOL_SPECS[1],
            ToolId::ComputeDiff => &TOOL_SPECS[2],
            ToolId::Verify => &TOOL_SPECS[3],
        }
    }

    pub(crate) fn risk(id: ToolId) -> ApprovalRiskClass {
        Self::get(id).risk_class
    }

    #[allow(dead_code)]
    pub(crate) fn min_tier(id: ToolId) -> PolicyTier {
        Self::get(id).min_tier
    }
}

pub(crate) fn tier_rank(tier: PolicyTier) -> u8 {
    match tier {
        PolicyTier::Strict => 0,
        PolicyTier::Balanced => 1,
        PolicyTier::Permissive => 2,
    }
}

pub(crate) fn tier_satisfies(current: PolicyTier, min_tier: PolicyTier) -> bool {
    tier_rank(current) >= tier_rank(min_tier)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn registry_lookup_is_deterministic() {
        let first = ToolRegistry::get(ToolId::ComputeDiff);
        let second = ToolRegistry::get(ToolId::ComputeDiff);
        assert_eq!(first, second);
    }

    #[test]
    fn registry_order_is_stable() {
        let ids: Vec<&'static str> = ToolRegistry::list()
            .iter()
            .map(|spec| spec.id.as_str())
            .collect();
        assert_eq!(
            ids,
            vec!["scan_repo", "generate_plan", "compute_diff", "verify"]
        );
    }

    #[test]
    fn min_tier_is_enforced_by_rank() {
        assert!(!tier_satisfies(PolicyTier::Strict, PolicyTier::Balanced));
        assert!(tier_satisfies(PolicyTier::Permissive, PolicyTier::Balanced));
    }
}
