use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use super::shell_state::ShellState;
use super::shell_state::ShellTab;

const TABS: [ShellTab; 6] = [
    ShellTab::Overview,
    ShellTab::System,
    ShellTab::Plan,
    ShellTab::Diff,
    ShellTab::Explain,
    ShellTab::Logs,
];

pub(crate) fn render(area: Rect, buffer: &mut Buffer, state: &ShellState) {
    let mut spans = Vec::new();
    for (idx, tab) in TABS.iter().enumerate() {
        if idx > 0 {
            spans.push(" | ".dim());
        }
        if *tab == state.routing.tab {
            spans.push(tab.label().cyan().bold());
        } else {
            spans.push(tab.label().dim());
        }
    }

    Paragraph::new(Line::from(spans))
        .block(Block::default().title(" Tabs ").borders(Borders::ALL))
        .render(area, buffer);
}
