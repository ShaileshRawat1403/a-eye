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
    let line = Line::from(vec![
        "A-Eye".bold().cyan(),
        " | Project: ".dim(),
        state.header.project_name.as_ref().into(),
        " | Mode: ".dim(),
        state.header.safety_mode.label().into(),
        " | Scan: ".dim(),
        state.header.scan.label().into(),
        " | Apply: ".dim(),
        state.header.apply.label().into(),
        " | Verify: ".dim(),
        state.header.verify.label().into(),
        " | Risk: ".dim(),
        state.header.risk.label().into(),
        " | Journey: ".dim(),
        state.journey_status.state.label().into(),
        " | SME: ".dim(),
        format!(
            "{} • {} • {}",
            state.sm.personality, state.sm.skills_enabled_count, state.sm.collaboration_mode_label,
        )
        .into(),
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
