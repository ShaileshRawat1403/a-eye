use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;

use super::shell_state::ShellCustomization;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ShellLayout {
    pub(crate) top_bar: Rect,
    pub(crate) journey: Rect,
    pub(crate) tabs: Rect,
    pub(crate) overview: Rect,
    pub(crate) chat: Rect,
    pub(crate) action_bar: Rect,
}

pub(crate) fn compute(area: Rect, customization: &ShellCustomization) -> ShellLayout {
    let top_bar_height = 3_u16.min(area.height);
    let action_bar_height = if customization.show_action_bar && area.height >= 16 {
        3
    } else {
        0
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_bar_height),
            Constraint::Min(1),
            Constraint::Length(action_bar_height),
        ])
        .split(area);

    let middle = rows[1];
    let middle_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(middle);

    let content = middle_rows[1];
    let show_journey = customization.show_journey && content.width >= 130 && content.height >= 10;
    let journey_width = (content.width / 4).clamp(22, 28);
    let cols = if show_journey {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(journey_width), Constraint::Min(20)])
            .split(content)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(0), Constraint::Min(20)])
            .split(content)
    };

    let right = cols[1];
    let show_overview = customization.show_overview && right.width >= 100 && right.height >= 8;
    let inspector_width = if show_overview {
        (right.width / 3).clamp(28, 44)
    } else {
        0
    };
    let right_cols = if show_overview {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(40), Constraint::Length(inspector_width)])
            .split(right)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(20), Constraint::Length(0)])
            .split(right)
    };

    ShellLayout {
        top_bar: rows[0],
        journey: cols[0],
        tabs: middle_rows[0],
        overview: right_cols[1],
        chat: right_cols[0],
        action_bar: if customization.show_action_bar {
            rows[2]
        } else {
            Rect::new(rows[2].x, rows[2].y, rows[2].width, 0)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn layout_snapshot(name: &str, area: Rect, customization: ShellCustomization) {
        let layout = compute(area, &customization);
        let snapshot = format!(
            "area=({},{} {}x{})\n\
             top=({},{} {}x{})\n\
             journey=({},{} {}x{})\n\
             tabs=({},{} {}x{})\n\
             overview=({},{} {}x{})\n\
             chat=({},{} {}x{})\n\
             action=({},{} {}x{})",
            area.x,
            area.y,
            area.width,
            area.height,
            layout.top_bar.x,
            layout.top_bar.y,
            layout.top_bar.width,
            layout.top_bar.height,
            layout.journey.x,
            layout.journey.y,
            layout.journey.width,
            layout.journey.height,
            layout.tabs.x,
            layout.tabs.y,
            layout.tabs.width,
            layout.tabs.height,
            layout.overview.x,
            layout.overview.y,
            layout.overview.width,
            layout.overview.height,
            layout.chat.x,
            layout.chat.y,
            layout.chat.width,
            layout.chat.height,
            layout.action_bar.x,
            layout.action_bar.y,
            layout.action_bar.width,
            layout.action_bar.height
        );
        assert_snapshot!(name, snapshot);
    }

    #[test]
    fn shell_layout_small_terminal_snapshot() {
        layout_snapshot(
            "shell_layout_small",
            Rect::new(0, 0, 80, 20),
            ShellCustomization {
                theme: super::super::shell_state::UiTheme::Classic,
                keymap_preset: super::super::shell_state::KeymapPreset::Standard,
                show_journey: true,
                show_overview: true,
                show_action_bar: true,
                auto_follow_intent: true,
            },
        );
    }

    #[test]
    fn shell_layout_medium_terminal_snapshot() {
        layout_snapshot(
            "shell_layout_medium",
            Rect::new(0, 0, 120, 32),
            ShellCustomization {
                theme: super::super::shell_state::UiTheme::Classic,
                keymap_preset: super::super::shell_state::KeymapPreset::Standard,
                show_journey: true,
                show_overview: true,
                show_action_bar: true,
                auto_follow_intent: true,
            },
        );
    }

    #[test]
    fn shell_layout_large_terminal_snapshot() {
        layout_snapshot(
            "shell_layout_large",
            Rect::new(0, 0, 180, 48),
            ShellCustomization {
                theme: super::super::shell_state::UiTheme::Classic,
                keymap_preset: super::super::shell_state::KeymapPreset::Standard,
                show_journey: true,
                show_overview: true,
                show_action_bar: true,
                auto_follow_intent: true,
            },
        );
    }
}
