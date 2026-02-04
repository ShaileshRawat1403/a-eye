use super::shell_state::ApprovalGateRequirement;
use super::shell_state::PolicyTier;
use super::shell_state::policy_requirement_for_risk;
use super::tool_registry::ToolId;
use super::tool_registry::ToolRegistry;
use super::tool_registry::tier_satisfies;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ToolPolicyOutcome {
    pub(crate) tool_id: ToolId,
    pub(crate) requirement: ApprovalGateRequirement,
    pub(crate) blocked: bool,
    pub(crate) reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PolicySimulationReport {
    pub(crate) outcomes: Vec<ToolPolicyOutcome>,
    pub(crate) allow_count: usize,
    pub(crate) approval_count: usize,
    pub(crate) blocked_count: usize,
}

pub(crate) fn simulate_tool(policy_tier: PolicyTier, tool_id: ToolId) -> ToolPolicyOutcome {
    let spec = ToolRegistry::get(tool_id);
    if !tier_satisfies(policy_tier, spec.min_tier) {
        return ToolPolicyOutcome {
            tool_id,
            requirement: ApprovalGateRequirement::Deny,
            blocked: true,
            reason: "policy tier below tool minimum",
        };
    }

    let requirement = policy_requirement_for_risk(policy_tier, spec.risk_class);
    let blocked = matches!(requirement, ApprovalGateRequirement::Deny);
    let reason = match requirement {
        ApprovalGateRequirement::Allow => "allowed by risk policy",
        ApprovalGateRequirement::RequireApproval => "requires approval by risk policy",
        ApprovalGateRequirement::Deny => "blocked by risk policy",
    };

    ToolPolicyOutcome {
        tool_id,
        requirement,
        blocked,
        reason,
    }
}

pub(crate) fn simulate_tools(
    policy_tier: PolicyTier,
    tool_ids: &[ToolId],
) -> PolicySimulationReport {
    let outcomes: Vec<ToolPolicyOutcome> = tool_ids
        .iter()
        .map(|tool_id| simulate_tool(policy_tier, *tool_id))
        .collect();

    let allow_count = outcomes
        .iter()
        .filter(|outcome| matches!(outcome.requirement, ApprovalGateRequirement::Allow))
        .count();
    let approval_count = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome.requirement,
                ApprovalGateRequirement::RequireApproval
            )
        })
        .count();
    let blocked_count = outcomes.iter().filter(|outcome| outcome.blocked).count();

    PolicySimulationReport {
        outcomes,
        allow_count,
        approval_count,
        blocked_count,
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::shell::workflow::WorkflowTemplateId;
    use crate::shell::workflow::workflow_template;

    #[test]
    fn strict_policy_blocks_tools_below_minimum_tier() {
        let outcome = simulate_tool(PolicyTier::Strict, ToolId::ComputeDiff);
        assert_eq!(outcome.requirement, ApprovalGateRequirement::Deny);
        assert!(outcome.blocked);
    }

    #[test]
    fn risk_mapping_is_consistent_with_classifier() {
        let outcome = simulate_tool(PolicyTier::Balanced, ToolId::Verify);
        assert_eq!(
            outcome.requirement,
            policy_requirement_for_risk(PolicyTier::Balanced, ToolRegistry::risk(ToolId::Verify))
        );
    }

    #[test]
    fn simulation_is_deterministic_for_same_inputs() {
        let tool_ids = [
            ToolId::ScanRepo,
            ToolId::GeneratePlan,
            ToolId::ComputeDiff,
            ToolId::Verify,
        ];
        let first = simulate_tools(PolicyTier::Balanced, &tool_ids);
        let second = simulate_tools(PolicyTier::Balanced, &tool_ids);
        assert_eq!(first, second);
    }

    #[test]
    fn workflow_simulation_report_counts_are_stable() {
        let template = workflow_template(WorkflowTemplateId::ScanPlanDiffVerify);
        let tool_ids: Vec<ToolId> = template.steps.iter().map(|step| step.tool_id).collect();
        let report = simulate_tools(PolicyTier::Balanced, &tool_ids);

        assert_eq!(report.allow_count, 3);
        assert_eq!(report.approval_count, 1);
        assert_eq!(report.blocked_count, 0);
    }

    #[test]
    fn strict_workflow_simulation_reports_blocked_steps() {
        let template = workflow_template(WorkflowTemplateId::ScanPlanDiffVerify);
        let tool_ids: Vec<ToolId> = template.steps.iter().map(|step| step.tool_id).collect();
        let report = simulate_tools(PolicyTier::Strict, &tool_ids);

        assert_eq!(report.allow_count, 2);
        assert_eq!(report.approval_count, 0);
        assert_eq!(report.blocked_count, 2);
    }
}
