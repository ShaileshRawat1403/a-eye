use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

pub(crate) fn render(area: Rect, buffer: &mut Buffer) {
    let line = Line::from(vec![
        "[Enter] ".cyan(),
        "Continue  ".dim(),
        "[Ctrl+/] ".cyan(),
        "Actions  ".dim(),
        "[Esc] ".cyan(),
        "Close overlay".dim(),
    ]);

    Paragraph::new(line)
        .block(Block::default().title(" Actions ").borders(Borders::ALL))
        .render(area, buffer);
}
