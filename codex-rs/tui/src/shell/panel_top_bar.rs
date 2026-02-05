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
        " | Theme: ".dim(),
        state.customization.theme.label().into(),
        " | Keys: ".dim(),
        state.customization.keymap_preset.label().into(),
        " | Intent: ".dim(),
        if state.customization.auto_follow_intent {
            "auto".green()
        } else {
            "manual".magenta()
        },
        " | Usage: ".dim(),
        usage.into(),
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
