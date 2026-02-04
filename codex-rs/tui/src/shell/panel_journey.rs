use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use super::shell_state::JourneyState;
use super::shell_state::JourneyStep;
use super::shell_state::ShellState;

const JOURNEY: [JourneyStep; 7] = [
    JourneyStep::Idea,
    JourneyStep::Understand,
    JourneyStep::Plan,
    JourneyStep::Preview,
    JourneyStep::Approve,
    JourneyStep::Verify,
    JourneyStep::Learn,
];

pub(crate) fn render(area: Rect, buffer: &mut Buffer, state: &ShellState) {
    if area.width == 0 {
        return;
    }

    let mut lines = Vec::new();
    let current_idx = JOURNEY
        .iter()
        .position(|step| *step == state.routing.journey)
        .unwrap_or(0);
    let failed = matches!(state.journey_status.state, JourneyState::Failed);

    for (idx, step) in JOURNEY.iter().enumerate() {
        let marker = if idx < current_idx && !failed {
            "✓".green()
        } else if idx == current_idx && failed {
            "x".red()
        } else if idx == current_idx {
            ">".cyan()
        } else {
            " ".dim()
        };
        lines.push(Line::from(vec![marker, " ".into(), step.label().into()]));
    }

    Paragraph::new(lines)
        .block(Block::default().title(" Journey ").borders(Borders::ALL))
        .render(area, buffer);
}
