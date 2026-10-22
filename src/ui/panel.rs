use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Widget};

use super::hits::{Hit, Hits};
use super::text::{Align, fit, put, width};

const CORNER: u16 = 2;
pub(super) const BORDER: Color = Color::DarkGray;

pub(super) struct Label {
    pub(super) text: String,
    pub(super) style: Style,
    pub(super) hit: Option<Hit>,
}

impl Label {
    pub(super) fn new(text: impl Into<String>, style: Style) -> Self {
        Self { text: text.into(), style, hit: None }
    }
}

pub(super) fn draw(buffer: &mut Buffer, area: Rect) -> Rect {
    if area.is_empty() {
        return area;
    }
    Block::bordered().border_type(BorderType::Rounded).border_style(Style::new().fg(BORDER)).render(area, buffer);
    area.inner(Margin::new(1, 1))
}

pub(super) fn top(area: Rect) -> Rect {
    Rect { height: area.height.min(1), ..area }
}

pub(super) fn bottom(area: Rect) -> Rect {
    if area.height < 2 { Rect { height: 0, ..area } } else { Rect { y: area.bottom() - 1, height: 1, ..area } }
}

pub(super) fn left(buffer: &mut Buffer, line: Rect, labels: &[Label], hits: &mut Hits) -> u16 {
    let mut x = line.x.saturating_add(CORNER);
    let end = line.right().saturating_sub(CORNER);
    for label in labels.iter().filter(|label| !label.text.is_empty()) {
        let text = format!(" {} ", label.text);
        let size = width(&text);
        if line.height == 0 || x.saturating_add(size) > end {
            break;
        }
        let rect = Rect { x, width: size, ..line };
        put(buffer, rect, &text, Align::Left, label.style);
        if let Some(hit) = label.hit {
            hits.push(rect, hit);
        }
        x = rect.right().saturating_add(1);
    }
    x
}

pub(super) fn right(buffer: &mut Buffer, line: Rect, from: u16, label: &Label) {
    let room = room(line, from);
    if line.height == 0 || room == 0 || label.text.is_empty() {
        return;
    }
    let text = format!(" {} ", fit(&label.text, room));
    let size = width(&text);
    let x = line.right().saturating_sub(CORNER).saturating_sub(size);
    put(buffer, Rect { x, width: size, ..line }, &text, Align::Left, label.style);
}

pub(super) fn room(line: Rect, from: u16) -> u16 {
    let end = line.right().saturating_sub(CORNER);
    end.saturating_sub(from.max(line.x.saturating_add(CORNER))).saturating_sub(2)
}
