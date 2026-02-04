use std::path::PathBuf;

use codex_core::config::CONFIG_TOML_FILE;
use codex_core::config::edit::ConfigEdit;
use codex_core::config::edit::ConfigEditsBuilder;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Wrap;
use toml_edit::value;

use crate::key_hint;
use crate::onboarding::onboarding_screen::KeyboardHandler;
use crate::onboarding::onboarding_screen::StepState;
use crate::onboarding::onboarding_screen::StepStateProvider;
use crate::render::Insets;
use crate::render::renderable::ColumnRenderable;
use crate::render::renderable::Renderable;
use crate::render::renderable::RenderableExt as _;
use crate::selection_list::selection_option_row;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SafetyTierSelection {
    Tier1,
    Tier2,
    Tier3,
}

impl SafetyTierSelection {
    fn config_values(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Tier1 => (
                "untrusted",
                "read-only",
                "Safest default: plan and diff first.",
            ),
            Self::Tier2 => (
                "untrusted",
                "workspace-write",
                "Recommended: supervised apply with strong safeguards.",
            ),
            Self::Tier3 => (
                "on-request",
                "workspace-write",
                "Advanced: faster flow with guided approvals.",
            ),
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Tier1 => Self::Tier2,
            Self::Tier2 => Self::Tier3,
            Self::Tier3 => Self::Tier1,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Tier1 => Self::Tier3,
            Self::Tier2 => Self::Tier1,
            Self::Tier3 => Self::Tier2,
        }
    }
}

pub(crate) struct SafetyTierWidget {
    pub codex_home: PathBuf,
    pub selection: Option<SafetyTierSelection>,
    pub highlighted: SafetyTierSelection,
    pub error: Option<String>,
}

impl SafetyTierWidget {
    pub(crate) fn new(codex_home: PathBuf) -> Self {
        Self {
            codex_home,
            selection: None,
            highlighted: SafetyTierSelection::Tier1,
            error: None,
        }
    }

    fn save_selection(&mut self, selection: SafetyTierSelection) {
        let (approval_policy, sandbox_mode, _) = selection.config_values();
        let edits = vec![
            ConfigEdit::SetPath {
                segments: vec!["approval_policy".to_string()],
                value: value(approval_policy),
            },
            ConfigEdit::SetPath {
                segments: vec!["sandbox_mode".to_string()],
                value: value(sandbox_mode),
            },
        ];

        match ConfigEditsBuilder::new(&self.codex_home)
            .with_edits(edits)
            .apply_blocking()
        {
            Ok(()) => {
                self.selection = Some(selection);
                self.highlighted = selection;
                self.error = None;
            }
            Err(err) => {
                self.error = Some(format!(
                    "Failed to save safety tier to {}: {err}",
                    self.codex_home.join(CONFIG_TOML_FILE).display()
                ));
            }
        }
    }
}

impl WidgetRef for &SafetyTierWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let mut column = ColumnRenderable::new();
        column.push(Line::from(vec!["▣ ".cyan(), "Safety tier".bold()]));
        column.push("");
        column.push(
            Paragraph::new(
                "Choose how much autonomy A-Eye should use by default. You can change this later in settings.",
            )
            .wrap(Wrap { trim: true })
            .inset(Insets::tlbr(0, 2, 0, 0)),
        );
        column.push("");

        let options = [
            (
                "Tier 1 - Plan + diff only (safest)",
                SafetyTierSelection::Tier1,
            ),
            (
                "Tier 2 - Supervised apply (recommended)",
                SafetyTierSelection::Tier2,
            ),
            (
                "Tier 3 - Guided autonomy (advanced)",
                SafetyTierSelection::Tier3,
            ),
        ];

        for (idx, (label, selection)) in options.iter().enumerate() {
            column.push(selection_option_row(
                idx,
                (*label).to_string(),
                self.highlighted == *selection,
            ));
        }

        let (_, _, selected_summary) = self.highlighted.config_values();
        column.push("");
        column.push(
            Paragraph::new(selected_summary)
                .dim()
                .wrap(Wrap { trim: true })
                .inset(Insets::tlbr(0, 2, 0, 0)),
        );

        if let Some(error) = &self.error {
            column.push("");
            column.push(
                Paragraph::new(error.to_string())
                    .red()
                    .wrap(Wrap { trim: true })
                    .inset(Insets::tlbr(0, 2, 0, 0)),
            );
        }

        column.push("");
        column.push(
            Line::from(vec![
                "Press ".dim(),
                key_hint::plain(KeyCode::Enter).into(),
                " to save this safety tier".dim(),
            ])
            .inset(Insets::tlbr(0, 2, 0, 0)),
        );

        column.render(area, buf);
    }
}

impl KeyboardHandler for SafetyTierWidget {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.kind == KeyEventKind::Release {
            return;
        }

        match key_event.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.highlighted = self.highlighted.previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.highlighted = self.highlighted.next();
            }
            KeyCode::Char('1') => self.highlighted = SafetyTierSelection::Tier1,
            KeyCode::Char('2') => self.highlighted = SafetyTierSelection::Tier2,
            KeyCode::Char('3') => self.highlighted = SafetyTierSelection::Tier3,
            KeyCode::Enter => self.save_selection(self.highlighted),
            _ => {}
        }
    }
}

impl StepStateProvider for SafetyTierWidget {
    fn get_step_state(&self) -> StepState {
        match self.selection {
            Some(_) => StepState::Complete,
            None => StepState::InProgress,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    #[test]
    fn release_event_does_not_set_selection() {
        let codex_home = TempDir::new().expect("temp home");
        let mut widget = SafetyTierWidget::new(codex_home.path().to_path_buf());

        let release = KeyEvent {
            kind: KeyEventKind::Release,
            ..KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
        };
        widget.handle_key_event(release);
        assert_eq!(widget.selection, None);
    }

    #[test]
    fn selecting_tier_two_persists_expected_policy() {
        let codex_home = TempDir::new().expect("temp home");
        let mut widget = SafetyTierWidget::new(codex_home.path().to_path_buf());

        widget.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        widget.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(widget.selection, Some(SafetyTierSelection::Tier2));
        let config_text = std::fs::read_to_string(codex_home.path().join(CONFIG_TOML_FILE))
            .expect("config should be written");
        assert!(config_text.contains("approval_policy = \"untrusted\""));
        assert!(config_text.contains("sandbox_mode = \"workspace-write\""));
    }
}
