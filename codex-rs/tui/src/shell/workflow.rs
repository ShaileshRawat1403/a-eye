use super::tool_registry::ToolId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowTemplateId {
    ScanPlanDiffVerify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkflowStepSpec {
    pub(crate) step_id: &'static str,
    pub(crate) tool_id: ToolId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkflowTemplate {
    pub(crate) id: WorkflowTemplateId,
    pub(crate) steps: &'static [WorkflowStepSpec],
}

const SCAN_PLAN_DIFF_VERIFY_STEPS: [WorkflowStepSpec; 4] = [
    WorkflowStepSpec {
        step_id: "scan",
        tool_id: ToolId::ScanRepo,
    },
    WorkflowStepSpec {
        step_id: "plan",
        tool_id: ToolId::GeneratePlan,
    },
    WorkflowStepSpec {
        step_id: "diff",
        tool_id: ToolId::ComputeDiff,
    },
    WorkflowStepSpec {
        step_id: "verify",
        tool_id: ToolId::Verify,
    },
];

const WORKFLOW_TEMPLATES: [WorkflowTemplate; 1] = [WorkflowTemplate {
    id: WorkflowTemplateId::ScanPlanDiffVerify,
    steps: &SCAN_PLAN_DIFF_VERIFY_STEPS,
}];

pub(crate) fn workflow_template(id: WorkflowTemplateId) -> &'static WorkflowTemplate {
    match id {
        WorkflowTemplateId::ScanPlanDiffVerify => &WORKFLOW_TEMPLATES[0],
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn scan_plan_diff_verify_step_order_is_stable() {
        let template = workflow_template(WorkflowTemplateId::ScanPlanDiffVerify);
        let steps: Vec<&'static str> = template.steps.iter().map(|step| step.step_id).collect();
        assert_eq!(steps, vec!["scan", "plan", "diff", "verify"]);
    }
}
