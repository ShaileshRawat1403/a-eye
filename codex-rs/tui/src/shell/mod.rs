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
pub(crate) use shell_state::KeymapPreset;
pub(crate) use shell_state::PolicyTier;
pub(crate) use shell_state::RiskLevel;
pub(crate) use shell_state::SafetyMode;
pub(crate) use shell_state::ScanStatus;
pub(crate) use shell_state::ShellState;
pub(crate) use shell_state::ShellTab;
pub(crate) use shell_state::SystemArtifact;
pub(crate) use shell_state::UsageSnapshot;
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

        if matches!(
            self.state.interaction.overlay,
            ShellOverlay::Onboarding { .. }
        ) {
            let action = match key_event.code {
                KeyCode::Esc => Some(UserAction::CompleteOnboarding),
                KeyCode::Left => Some(UserAction::PrevOnboardingStep),
                KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => {
                    Some(UserAction::NextOnboardingStep)
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
                code: KeyCode::Tab,
                modifiers: KeyModifiers::NONE,
                ..
            } => Some(UserAction::NextTab),
            KeyEvent {
                code: KeyCode::BackTab,
                modifiers: KeyModifiers::SHIFT,
                ..
            } => Some(UserAction::CycleTheme),
            KeyEvent {
                code: KeyCode::F(2),
                modifiers: KeyModifiers::NONE,
                ..
            } => Some(UserAction::ToggleActionPalette),
            KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => Some(UserAction::ToggleAutoIntentFollow),
            KeyEvent {
                code: KeyCode::F(3),
                modifiers: KeyModifiers::NONE,
                ..
            } => Some(UserAction::NextTab),
            KeyEvent {
                code: KeyCode::F(4),
                modifiers: KeyModifiers::NONE,
                ..
            } => Some(UserAction::PrevTab),
            KeyEvent {
                code: KeyCode::F(6),
                modifiers: KeyModifiers::NONE,
                ..
            } => Some(UserAction::CycleTheme),
            KeyEvent {
                code: KeyCode::F(7),
                modifiers: KeyModifiers::NONE,
                ..
            } => Some(UserAction::ToggleJourneyPanel),
            KeyEvent {
                code: KeyCode::F(8),
                modifiers: KeyModifiers::NONE,
                ..
            } => Some(UserAction::ToggleOverviewPanel),
            KeyEvent {
                code: KeyCode::F(9),
                modifiers: KeyModifiers::NONE,
                ..
            } => Some(UserAction::ToggleActionBar),
            KeyEvent {
                code: KeyCode::F(10),
                modifiers: KeyModifiers::NONE,
                ..
            } => Some(UserAction::ToggleAutoIntentFollow),
            _ => match self.state.customization.keymap_preset {
                KeymapPreset::Mac => match key_event {
                    KeyEvent {
                        code: KeyCode::Char('a'),
                        modifiers: KeyModifiers::ALT,
                        ..
                    } => Some(UserAction::ToggleActionPalette),
                    KeyEvent {
                        code: KeyCode::Char('l'),
                        modifiers: KeyModifiers::ALT,
                        ..
                    } => Some(UserAction::NextTab),
                    KeyEvent {
                        code: KeyCode::Char('h'),
                        modifiers: KeyModifiers::ALT,
                        ..
                    } => Some(UserAction::PrevTab),
                    KeyEvent {
                        code: KeyCode::Char('j'),
                        modifiers: KeyModifiers::ALT,
                        ..
                    } => Some(UserAction::ToggleJourneyPanel),
                    KeyEvent {
                        code: KeyCode::Char('k'),
                        modifiers: KeyModifiers::ALT,
                        ..
                    } => Some(UserAction::ToggleOverviewPanel),
                    KeyEvent {
                        code: KeyCode::Char('b'),
                        modifiers: KeyModifiers::ALT,
                        ..
                    } => Some(UserAction::ToggleActionBar),
                    KeyEvent {
                        code: KeyCode::Char('i'),
                        modifiers: KeyModifiers::ALT,
                        ..
                    } => Some(UserAction::ToggleAutoIntentFollow),
                    KeyEvent {
                        code: KeyCode::Char('y'),
                        modifiers: KeyModifiers::ALT,
                        ..
                    } => Some(UserAction::CycleTheme),
                    _ => None,
                },
                KeymapPreset::Standard | KeymapPreset::Windows => match key_event {
                    KeyEvent {
                        code: KeyCode::Char('/'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => Some(UserAction::ToggleActionPalette),
                    KeyEvent {
                        code: KeyCode::Right,
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => Some(UserAction::NextTab),
                    KeyEvent {
                        code: KeyCode::Left,
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => Some(UserAction::PrevTab),
                    KeyEvent {
                        code: KeyCode::Char('j'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => Some(UserAction::ToggleJourneyPanel),
                    KeyEvent {
                        code: KeyCode::Char('k'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => Some(UserAction::ToggleOverviewPanel),
                    KeyEvent {
                        code: KeyCode::Char('b'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => Some(UserAction::ToggleActionBar),
                    KeyEvent {
                        code: KeyCode::Char('i'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => Some(UserAction::ToggleAutoIntentFollow),
                    KeyEvent {
                        code: KeyCode::Char('y'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => Some(UserAction::CycleTheme),
                    _ => None,
                },
            },
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
        let layout = shell_layout::compute(area, &self.state.customization);
        let chat_focus_area = merged_vertical_rects(layout.overview, layout.chat);

        panel_top_bar::render(layout.top_bar, buffer, &self.state);
        panel_journey::render(layout.journey, buffer, &self.state);
        panel_tabs::render(layout.tabs, buffer, &self.state);
        panel_action_bar::render(layout.action_bar, buffer, &self.state);
        if matches!(self.state.routing.tab, ShellTab::Chat) {
            panel_chat_adapter::render(chat_widget, chat_focus_area, buffer);
        } else {
            panel_overview::render(layout.overview, buffer, &self.state);
            panel_chat_adapter::render(chat_widget, layout.chat, buffer);
        }

        let overlay_active = !matches!(self.state.interaction.overlay, ShellOverlay::None);
        match self.state.interaction.overlay {
            ShellOverlay::ActionPalette { .. } => self.render_action_palette(area, buffer),
            ShellOverlay::Onboarding { .. } => self.render_onboarding(area, buffer),
            ShellOverlay::None => {}
        }

        ShellRenderOutput {
            chat_rect: if matches!(self.state.routing.tab, ShellTab::Chat) {
                chat_focus_area
            } else {
                layout.chat
            },
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

    fn render_onboarding(&self, area: Rect, buffer: &mut Buffer) {
        let ShellOverlay::Onboarding { step } = self.state.interaction.overlay else {
            return;
        };
        let pages: [(&str, &str); 4] = [
            (
                "Welcome to A-Eye Shell",
                "This interface is intent-driven: it follows your task and keeps risk visible.",
            ),
            (
                "Move Fast, Stay Safe",
                "Use Ctrl+/ for actions. Approvals and policy gates protect risky operations.",
            ),
            (
                "Customize Your View",
                "Use Tab to move tabs. Use Shift+Tab to cycle themes quickly.",
            ),
            (
                "Non-dev Friendly Flow",
                "Journey guides you from Idea -> Verify. Press Shift+? for shortcuts and guide.",
            ),
        ];
        let idx = step.min(pages.len().saturating_sub(1));
        let popup = centered_rect(area, 72, 46);
        Clear.render(popup, buffer);
        let (title, body) = pages[idx];
        let lines = vec![
            Line::from(vec![
                "Step ".dim(),
                format!("{}/{}", idx + 1, pages.len()).bold(),
            ]),
            "".into(),
            Line::from(title.bold().cyan()),
            "".into(),
            Line::from(body),
            "".into(),
            "[Left/Right] navigate  [Enter] next  [Esc] close"
                .dim()
                .into(),
        ];
        Paragraph::new(lines)
            .block(Block::default().title(" Onboarding ").borders(Borders::ALL))
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

fn merged_vertical_rects(top: Rect, bottom: Rect) -> Rect {
    if top.height == 0 {
        return bottom;
    }
    if bottom.height == 0 {
        return top;
    }

    let left = top.x.min(bottom.x);
    let width = top.width.max(bottom.width);
    let y = top.y.min(bottom.y);
    let bottom_edge = top
        .y
        .saturating_add(top.height)
        .max(bottom.y.saturating_add(bottom.height));

    Rect::new(left, y, width, bottom_edge.saturating_sub(y))
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
        shell.dispatch(ShellAction::Runtime(RuntimeAction::SetKeymapPreset(
            KeymapPreset::Standard,
        )));
        let result =
            shell.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::CONTROL));
        assert!(result.consumed);
        assert!(matches!(
            shell.state_mut().interaction.overlay,
            ShellOverlay::ActionPalette { .. }
        ));
    }

    #[test]
    fn f2_opens_action_palette() {
        let mut shell = Shell::new(ShellState::new(
            "project".to_string(),
            Personality::Friendly,
        ));
        let result = shell.handle_key_event(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        assert!(result.consumed);
        assert!(matches!(
            shell.state_mut().interaction.overlay,
            ShellOverlay::ActionPalette { .. }
        ));
    }

    #[test]
    fn ctrl_a_toggles_intent_follow() {
        let mut shell = Shell::new(ShellState::new(
            "project".to_string(),
            Personality::Friendly,
        ));
        let initial = shell.state_mut().customization.auto_follow_intent;
        let result =
            shell.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert!(result.consumed);
        assert_ne!(shell.state_mut().customization.auto_follow_intent, initial);
    }

    #[test]
    fn question_mark_passthrough_for_chat_shortcuts() {
        let mut shell = Shell::new(ShellState::new(
            "project".to_string(),
            Personality::Friendly,
        ));
        let result = shell.handle_key_event(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT));
        assert!(!result.consumed);
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
