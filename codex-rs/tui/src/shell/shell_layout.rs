use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ShellLayout {
    pub(crate) top_bar: Rect,
    pub(crate) journey: Rect,
    pub(crate) tabs: Rect,
    pub(crate) overview: Rect,
    pub(crate) chat: Rect,
    pub(crate) action_bar: Rect,
}

pub(crate) fn compute(area: Rect) -> ShellLayout {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    let middle = rows[1];
    let show_journey = middle.width >= 90;
    let cols = if show_journey {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(26), Constraint::Min(20)])
            .split(middle)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(0), Constraint::Min(20)])
            .split(middle)
    };

    let right = cols[1];
    let show_overview = right.height >= 16;
    let right_rows = if show_overview {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Min(6),
            ])
            .split(right)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(0),
                Constraint::Min(6),
            ])
            .split(right)
    };

    ShellLayout {
        top_bar: rows[0],
        journey: cols[0],
        tabs: right_rows[0],
        overview: right_rows[1],
        chat: right_rows[2],
        action_bar: rows[2],
    }
}
