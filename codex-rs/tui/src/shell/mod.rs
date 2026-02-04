use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use crate::chatwidget::ChatWidget;

mod panel_action_bar;
mod panel_chat_adapter;
mod panel_journey;
mod panel_overview;
mod panel_tabs;
mod panel_top_bar;
mod persistence;
mod policy_simulation;
mod shell_effects;
mod shell_layout;
mod shell_reducer;
mod shell_state;
mod tool_executor;
mod tool_registry;
mod ui_action;
mod workflow;

pub(crate) use persistence::PersistedExecutionMode;
pub(crate) use persistence::PersistedPersonaPolicy;
pub(crate) use persistence::PersistedShellEvent;
pub(crate) use persistence::PersistedShellSnapshot;
pub(crate) use persistence::PersistedWorkflowStatus;
pub(crate) use persistence::ReplayedWorkflowRun;
pub(crate) use persistence::ShellEventStore;
pub(crate) use persistence::replay_latest_workflow;
pub(crate) use persistence::replay_workflow_from;
pub(crate) use policy_simulation::simulate_tool;
pub(crate) use shell_effects::UiEffect;
pub(crate) use shell_state::ARTIFACT_SCHEMA_V1;
pub(crate) use shell_state::ApplyStatus;
pub(crate) use shell_state::ApprovalAction;
pub(crate) use shell_state::ApprovalDecisionKind;
pub(crate) use shell_state::ApprovalDecisionRecord;
pub(crate) use shell_state::ApprovalGateRequirement;
pub(crate) use shell_state::ApprovalRequestRecord;
pub(crate) use shell_state::ApprovalRiskClass;
pub(crate) use shell_state::ErrorKind;
pub(crate) use shell_state::JourneyState;
pub(crate) use shell_state::PolicyTier;
pub(crate) use shell_state::RiskLevel;
pub(crate) use shell_state::SafetyMode;
pub(crate) use shell_state::ScanStatus;
pub(crate) use shell_state::ShellState;
pub(crate) use shell_state::ShellTab;
pub(crate) use shell_state::SystemArtifact;
pub(crate) use shell_state::VerifyArtifact;
pub(crate) use shell_state::VerifyCheck;
pub(crate) use shell_state::VerifyCheckStatus;
pub(crate) use shell_state::VerifyOverall;
pub(crate) use shell_state::VerifyStatus;
pub(crate) use shell_state::persona_policy_for;
pub(crate) use tool_executor::RuntimeToolExecutor;
pub(crate) use tool_executor::SimulatedToolExecutor;
pub(crate) use tool_executor::ToolExecutionContext;
pub(crate) use tool_executor::ToolExecutionPayload;
pub(crate) use tool_executor::ToolExecutor;
pub(crate) use tool_registry::ToolId;
pub(crate) use tool_registry::ToolInvocation;
pub(crate) use tool_registry::ToolInvocationStatus;
pub(crate) use tool_registry::ToolRegistry;
pub(crate) use tool_registry::ToolResult;
pub(crate) use ui_action::RuntimeAction;
pub(crate) use ui_action::ShellAction;
pub(crate) use ui_action::UserAction;
pub(crate) use workflow::WorkflowTemplateId;
pub(crate) use workflow::workflow_template;

pub(crate) use self::shell_effects::apply_effects;
use self::shell_state::ShellOverlay;
use self::ui_action::PALETTE_ITEMS;
use self::ui_action::filtered_palette_indices;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ShellRenderOutput {
    pub(crate) chat_rect: Rect,
    pub(crate) overlay_active: bool,
}

#[derive(Debug)]
pub(crate) struct ShellInputResult {
    pub(crate) consumed: bool,
    pub(crate) effects: Vec<UiEffect>,
}

pub(crate) struct Shell {
    state: ShellState,
}

impl Shell {
    pub(crate) fn new(state: ShellState) -> Self {
        Self { state }
    }

    pub(crate) fn dispatch(&mut self, action: ShellAction) -> Vec<UiEffect> {
        shell_reducer::reduce(&mut self.state, action)
    }

    pub(crate) fn journey_state(&self) -> JourneyState {
        self.state.journey_status.state
    }

    pub(crate) fn current_run_id(&self) -> u64 {
        self.state.current_run_id()
    }

    pub(crate) fn has_pending_approval(&self, request_id: &str) -> bool {
        self.state
            .approval
            .pending
            .as_ref()
            .is_some_and(|pending| pending.request.request_id == request_id)
    }

    pub(crate) fn pending_approval_run_id(&self, request_id: &str) -> Option<u64> {
        self.state.approval.pending.as_ref().and_then(|pending| {
            if pending.request.request_id == request_id {
                Some(pending.request.run_id)
            } else {
                None
            }
        })
    }

    pub(crate) fn persona_policy_snapshot(&self) -> PersistedPersonaPolicy {
        PersistedPersonaPolicy {
            tier_ceiling: self
                .state
                .sm
                .persona_policy
                .tier_ceiling
                .label()
                .to_string(),
            explanation_depth: self
                .state
                .sm
                .persona_policy
                .explanation_depth
                .label()
                .to_string(),
            output_format: self
                .state
                .sm
                .persona_policy
                .output_format
                .label()
                .to_string(),
        }
    }

    pub(crate) fn handle_key_event(&mut self, key_event: KeyEvent) -> ShellInputResult {
        if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return ShellInputResult {
                consumed: false,
                effects: Vec::new(),
            };
        }

        if matches!(
            self.state.interaction.overlay,
            ShellOverlay::ActionPalette { .. }
        ) {
            let action = match key_event.code {
                KeyCode::Esc => Some(UserAction::CloseOverlay),
                KeyCode::Up => Some(UserAction::OverlayMoveUp),
                KeyCode::Down => Some(UserAction::OverlayMoveDown),
                KeyCode::Enter => Some(UserAction::OverlaySubmit),
                KeyCode::Backspace => Some(UserAction::OverlayQueryBackspace),
                KeyCode::Char(ch)
                    if key_event.modifiers.is_empty()
                        || key_event.modifiers == KeyModifiers::SHIFT =>
                {
                    Some(UserAction::OverlayQueryInput(ch))
                }
                _ => None,
            };

            let effects = action
                .map(|a| self.dispatch(ShellAction::User(a)))
                .unwrap_or_default();
            return ShellInputResult {
                consumed: true,
                effects,
            };
        }

        let action = match key_event {
            KeyEvent {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => Some(UserAction::ToggleActionPalette),
            _ => None,
        };

        let consumed = action.is_some();
        let effects = action
            .map(|a| self.dispatch(ShellAction::User(a)))
            .unwrap_or_default();
        ShellInputResult { consumed, effects }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) -> ShellInputResult {
        if matches!(
            self.state.interaction.overlay,
            ShellOverlay::ActionPalette { .. }
        ) {
            let effects = self.dispatch(ShellAction::User(UserAction::OverlayQueryPaste(pasted)));
            return ShellInputResult {
                consumed: true,
                effects,
            };
        }

        ShellInputResult {
            consumed: false,
            effects: Vec::new(),
        }
    }

    pub(crate) fn render(
        &self,
        area: Rect,
        buffer: &mut Buffer,
        chat_widget: &ChatWidget,
    ) -> ShellRenderOutput {
        let layout = shell_layout::compute(area);

        panel_top_bar::render(layout.top_bar, buffer, &self.state);
        panel_journey::render(layout.journey, buffer, &self.state);
        panel_tabs::render(layout.tabs, buffer, &self.state);
        panel_overview::render(layout.overview, buffer, &self.state);
        panel_action_bar::render(layout.action_bar, buffer);
        panel_chat_adapter::render(chat_widget, layout.chat, buffer);

        let overlay_active = matches!(
            self.state.interaction.overlay,
            ShellOverlay::ActionPalette { .. }
        );
        if overlay_active {
            self.render_action_palette(area, buffer);
        }

        ShellRenderOutput {
            chat_rect: layout.chat,
            overlay_active,
        }
    }

    fn render_action_palette(&self, area: Rect, buffer: &mut Buffer) {
        let ShellOverlay::ActionPalette { selected, query } = &self.state.interaction.overlay
        else {
            return;
        };

        let popup = centered_rect(area, 70, 50);
        Clear.render(popup, buffer);

        let filtered = filtered_palette_indices(query.as_ref());
        let mut lines = Vec::new();
        lines.push(Line::from(vec!["Query: ".dim(), query.as_ref().into()]));
        lines.push("".into());

        if filtered.is_empty() {
            lines.push("No actions match your query.".dim().into());
        } else {
            for (idx, item_idx) in filtered.iter().enumerate() {
                let label = PALETTE_ITEMS[*item_idx].label;
                if idx == *selected {
                    lines.push(Line::from(vec!["> ".cyan(), label.cyan()]));
                } else {
                    lines.push(Line::from(format!("  {label}")).dim());
                }
            }
        }

        lines.push("".into());
        lines.push("[Enter] run  [Esc] close  [Up/Down] move".dim().into());

        Paragraph::new(lines)
            .block(Block::default().title(" Actions ").borders(Borders::ALL))
            .render(popup, buffer);
    }

    #[cfg(test)]
    pub(crate) fn state_mut(&mut self) -> &mut ShellState {
        &mut self.state
    }
}

fn centered_rect(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1]);

    horizontal[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::config_types::Personality;

    #[test]
    fn slash_passthrough_for_chat_commands() {
        let mut shell = Shell::new(ShellState::new(
            "project".to_string(),
            Personality::Friendly,
        ));
        let result = shell.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert!(!result.consumed);
    }

    #[test]
    fn ctrl_slash_opens_action_palette() {
        let mut shell = Shell::new(ShellState::new(
            "project".to_string(),
            Personality::Friendly,
        ));
        let result =
            shell.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::CONTROL));
        assert!(result.consumed);
        assert!(matches!(
            shell.state_mut().interaction.overlay,
            ShellOverlay::ActionPalette { .. }
        ));
    }

    #[test]
    fn q_passthrough_for_chat_input() {
        let mut shell = Shell::new(ShellState::new(
            "project".to_string(),
            Personality::Friendly,
        ));
        let result = shell.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(!result.consumed);
    }
}
