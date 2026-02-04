use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::chatwidget::ChatWidget;
use crate::render::renderable::Renderable;

pub(crate) fn render(chat_widget: &ChatWidget, area: Rect, buffer: &mut Buffer) {
    chat_widget.render(area, buffer);
}
