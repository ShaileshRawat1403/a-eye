use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use super::shell_state::ShellState;

pub(crate) fn render(area: Rect, buffer: &mut Buffer, state: &ShellState) {
    let accent = state.customization.theme.accent();
    let usage = usage_summary(state);
    let line = Line::from(vec![
        "A-Eye".bold().fg(accent),
        " | ".dim(),
        state.header.project_name.as_ref().into(),
        " | mode ".dim(),
        state.header.safety_mode.label().into(),
        " | risk ".dim(),
        state.header.risk.label().into(),
        " | step ".dim(),
        state.journey_status.state.label().into(),
        " | usage ".dim(),
        usage.into(),
    ]);

    Paragraph::new(line)
        .block(
            Block::default()
                .title(" A-Eye Shell ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .render(area, buffer);
}

fn usage_summary(state: &ShellState) -> String {
    let mut parts = Vec::new();
    if let Some(context) = state.usage.context_remaining_percent {
        parts.push(format!("ctx {context}%"));
    }
    if let (Some(label), Some(percent)) = (
        state.usage.primary_window_label.as_ref(),
        state.usage.primary_remaining_percent,
    ) {
        parts.push(format!("{label} {percent}%"));
    }
    if let (Some(label), Some(percent)) = (
        state.usage.secondary_window_label.as_ref(),
        state.usage.secondary_remaining_percent,
    ) {
        parts.push(format!("{label} {percent}%"));
    }
    if let Some(credits) = state.usage.credits_label.as_ref() {
        parts.push(format!("credits {credits}"));
    }

    if parts.is_empty() {
        "n/a".to_string()
    } else {
        parts.join(" • ")
    }
}
