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

use crate::key_hint;
use crate::onboarding::onboarding_screen::KeyboardHandler;
use crate::onboarding::onboarding_screen::StepState;
use crate::onboarding::onboarding_screen::StepStateProvider;
use crate::render::Insets;
use crate::render::renderable::ColumnRenderable;
use crate::render::renderable::Renderable;
use crate::render::renderable::RenderableExt as _;

pub(crate) struct SummaryWidget {
    acknowledged: bool,
}

impl SummaryWidget {
    pub(crate) fn new() -> Self {
        Self {
            acknowledged: false,
        }
    }
}

impl WidgetRef for &SummaryWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let mut column = ColumnRenderable::new();
        column.push(Line::from(vec!["✓ ".green(), "You're ready".bold()]));
        column.push("");
        column.push(
            Paragraph::new("A-Eye is configured. Start with a simple goal and iterate safely.")
                .wrap(Wrap { trim: true })
                .inset(Insets::tlbr(0, 2, 0, 0)),
        );
        column.push("");
        column.push("  Next workflow".bold());
        column.push(
            Paragraph::new("1) Scan context → 2) Build plan → 3) Review patch → 4) Verify")
                .dim()
                .wrap(Wrap { trim: true })
                .inset(Insets::tlbr(0, 2, 0, 0)),
        );
        column.push("");
        column.push("  Useful commands".bold());
        column.push(
            Paragraph::new("a-eye scan\n  a-eye plan \"describe your goal\"\n  a-eye")
                .dim()
                .wrap(Wrap { trim: false })
                .inset(Insets::tlbr(0, 2, 0, 0)),
        );
        column.push("");
        column.push(
            Line::from(vec![
                "Press ".dim(),
                key_hint::plain(KeyCode::Enter).into(),
                " to continue".dim(),
            ])
            .inset(Insets::tlbr(0, 2, 0, 0)),
        );
        column.render(area, buf);
    }
}

impl KeyboardHandler for SummaryWidget {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.kind == KeyEventKind::Release {
            return;
        }

        if matches!(
            key_event.code,
            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('c')
        ) {
            self.acknowledged = true;
        }
    }
}

impl StepStateProvider for SummaryWidget {
    fn get_step_state(&self) -> StepState {
        if self.acknowledged {
            StepState::Complete
        } else {
            StepState::InProgress
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use pretty_assertions::assert_eq;

    #[test]
    fn enter_acknowledges_summary() {
        let mut widget = SummaryWidget::new();
        assert_eq!(widget.get_step_state(), StepState::InProgress);

        widget.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(widget.get_step_state(), StepState::Complete);
    }
}
