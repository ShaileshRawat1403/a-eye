use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use super::shell_state::KeymapPreset;
use super::shell_state::ShellState;

pub(crate) fn render(area: Rect, buffer: &mut Buffer, state: &ShellState) {
    if area.height == 0 {
        return;
    }
    let line = match state.customization.keymap_preset {
        KeymapPreset::Mac => Line::from(vec![
            "[/] ".cyan(),
            "Commands  ".dim(),
            "[Shift+?] ".cyan(),
            "Guide  ".dim(),
            "[Tab] ".cyan(),
            "Next  ".dim(),
            "[Shift+Tab] ".cyan(),
            "Theme  ".dim(),
            "[Ctrl+A] ".cyan(),
            "Intent  ".dim(),
            "[Ctrl+/] ".cyan(),
            "Actions  ".dim(),
            "[Ctrl+L] ".cyan(),
            "Clear  ".dim(),
            "[Esc] ".cyan(),
            "Close overlay".dim(),
        ]),
        KeymapPreset::Standard | KeymapPreset::Windows => Line::from(vec![
            "[/] ".cyan(),
            "Commands  ".dim(),
            "[Shift+?] ".cyan(),
            "Guide  ".dim(),
            "[Tab] ".cyan(),
            "Next  ".dim(),
            "[Shift+Tab] ".cyan(),
            "Theme  ".dim(),
            "[Ctrl+/] ".cyan(),
            "Actions  ".dim(),
            "[Ctrl+A] ".cyan(),
            "Intent  ".dim(),
            "[Ctrl+Left/Right] ".cyan(),
            "Tabs  ".dim(),
            "[Ctrl+L] ".cyan(),
            "Clear  ".dim(),
            "[Esc] ".cyan(),
            "Close overlay".dim(),
        ]),
    };

    Paragraph::new(line)
        .block(Block::default().title(" Actions ").borders(Borders::ALL))
        .render(area, buffer);
}
