use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use super::shell_state::DiffFileStatus;
use super::shell_state::LogEntry;
use super::shell_state::PlanStep;
use super::shell_state::ShellState;
use super::shell_state::ShellTab;
use super::shell_state::StepStatus;

pub(crate) fn render(area: Rect, buffer: &mut Buffer, state: &ShellState) {
    if area.height == 0 {
        return;
    }

    let lines = match state.routing.tab {
        ShellTab::Overview => {
            let gate = state
                .approval
                .last_gate
                .as_ref()
                .map(|gate| {
                    format!(
                        "{}:{}:{}",
                        gate.action.label(),
                        gate.risk.label(),
                        gate.requirement.label()
                    )
                })
                .unwrap_or_else(|| "none".to_string());
            let approval = state
                .approval
                .pending
                .as_ref()
                .map(|pending| format!("pending#{}", pending.request.request_id))
                .or_else(|| {
                    state.approval.last_decision.as_ref().map(|decision| {
                        format!("{}#{}", decision.decision.label(), decision.request_id)
                    })
                })
                .unwrap_or_else(|| "none".to_string());

            vec![
                Line::from(vec![
                    "Journey: ".dim(),
                    state.journey_status.state.label().cyan().bold(),
                    " • Step: ".dim(),
                    state.journey_status.step.label().into(),
                    " • Run: ".dim(),
                    state.journey_status.active_run_id.to_string().into(),
                ]),
                Line::from(vec![
                    "Policy: ".dim(),
                    state.approval.policy_tier.label().into(),
                    " • Gate: ".dim(),
                    gate.into(),
                    " • Approval: ".dim(),
                    approval.into(),
                ]),
                Line::from(vec![
                    "Persona: ".dim(),
                    state.sm.personality.to_string().into(),
                    " • Tier ceiling: ".dim(),
                    state.sm.persona_policy.tier_ceiling.label().into(),
                    " • Explain: ".dim(),
                    state.sm.persona_policy.explanation_depth.label().into(),
                    " • Format: ".dim(),
                    state.sm.persona_policy.output_format.label().into(),
                ]),
            ]
        }
        ShellTab::System => render_system(state),
        ShellTab::Plan => render_plan(state),
        ShellTab::Diff => render_diff(state),
        ShellTab::Explain => render_explain(state),
        ShellTab::Logs => render_logs(state),
    };

    Paragraph::new(lines)
        .block(Block::default().title(" Context ").borders(Borders::ALL))
        .render(area, buffer);
}

fn render_system(state: &ShellState) -> Vec<Line<'static>> {
    let Some(system) = state.artifacts.system.as_ref() else {
        return vec!["No system artifact yet.".dim().into()];
    };

    vec![
        Line::from(format!("Summary: {}", system.summary)),
        Line::from(format!(
            "Stack: {}",
            if system.detected_stack.is_empty() {
                "unknown".to_string()
            } else {
                system.detected_stack.join(", ")
            }
        )),
        Line::from(format!(
            "Entrypoints: {} • Risks: {}",
            if system.entrypoints.is_empty() {
                "none".to_string()
            } else {
                system.entrypoints.len().to_string()
            },
            system.risk_flags.len()
        )),
    ]
}

fn render_plan(state: &ShellState) -> Vec<Line<'static>> {
    let Some(plan) = state.artifacts.plan.as_ref() else {
        return vec!["No plan artifact yet.".dim().into()];
    };

    let selected = state.selection.selected_plan_step.as_deref();
    let line = selected
        .and_then(|selected_id| plan.steps.iter().find(|step| step.id == selected_id))
        .or_else(|| plan.steps.first())
        .map(format_plan_step_line)
        .unwrap_or_else(|| Line::from("No plan steps.".dim()));

    vec![
        Line::from(format!("Title: {}", plan.title)),
        Line::from(format!(
            "Steps: {} • Selected: {}",
            plan.steps.len(),
            selected.unwrap_or("none")
        )),
        line,
    ]
}

fn render_diff(state: &ShellState) -> Vec<Line<'static>> {
    let Some(diff) = state.artifacts.diff.as_ref() else {
        return vec!["No diff artifact yet.".dim().into()];
    };

    let selected_path = state.selection.selected_diff_file.as_deref();
    let selected = selected_path
        .and_then(|path| diff.files.iter().find(|file| file.path == path))
        .or_else(|| diff.files.first());

    let selected_line = if let Some(file) = selected {
        Line::from(format!(
            "Selected: {} • Status: {} • Hunks: {}",
            file.path,
            diff_status_label(file.status),
            file.hunks.len()
        ))
    } else {
        Line::from("No diff files.".dim())
    };

    vec![
        Line::from(format!("Summary: {}", diff.summary)),
        Line::from(format!(
            "Files: {} • Selected: {}",
            diff.files.len(),
            selected_path.unwrap_or("none")
        )),
        selected_line,
    ]
}

fn render_explain(state: &ShellState) -> Vec<Line<'static>> {
    let mut explanation = String::new();

    if let Some(verify) = state.artifacts.verify.as_ref() {
        match verify.overall {
            super::shell_state::VerifyOverall::Passing => {
                explanation = "Verification passed. Changes look healthy.".to_string();
            }
            super::shell_state::VerifyOverall::Failing => {
                explanation =
                    "Verification failed. Review logs and selected diff file.".to_string();
            }
            super::shell_state::VerifyOverall::Unknown => {}
        }
    }

    if explanation.is_empty()
        && let Some(plan) = state.artifacts.plan.as_ref()
    {
        explanation = format!(
            "Planned {} steps. Review selected step for next action.",
            plan.steps.len()
        );
    }

    if explanation.is_empty() {
        explanation = "Explanation will appear once artifacts are available.".to_string();
    }

    vec![Line::from(explanation)]
}

fn render_logs(state: &ShellState) -> Vec<Line<'static>> {
    let filtered: Vec<&LogEntry> = state
        .artifacts
        .logs
        .iter()
        .filter(|entry| {
            let level_matches = state
                .selection
                .log_level_filter
                .is_none_or(|level| level == entry.level);
            let search = state.selection.log_search.trim();
            let search_matches = if search.is_empty() {
                true
            } else {
                entry
                    .message
                    .to_ascii_lowercase()
                    .contains(&search.to_ascii_lowercase())
            };
            level_matches && search_matches
        })
        .collect();

    let Some(last) = filtered.last() else {
        return vec!["No logs yet.".dim().into()];
    };

    vec![
        Line::from(format!(
            "Visible: {} • Total: {}",
            filtered.len(),
            state.artifacts.logs.iter().count()
        )),
        Line::from(format!(
            "Last #{} [{:?}] {}",
            last.seq, last.level, last.message
        )),
    ]
}

fn diff_status_label(status: DiffFileStatus) -> &'static str {
    match status {
        DiffFileStatus::Added => "added",
        DiffFileStatus::Modified => "modified",
        DiffFileStatus::Deleted => "deleted",
        DiffFileStatus::Renamed => "renamed",
    }
}

fn format_plan_step_line(step: &PlanStep) -> Line<'static> {
    let status = match step.status {
        StepStatus::Pending => "pending",
        StepStatus::Running => "running",
        StepStatus::Done => "done",
        StepStatus::Failed => "failed",
    };
    Line::from(format!("Selected step: {} ({status})", step.label))
}
