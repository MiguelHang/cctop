use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;

const ELLIPSIS: char = '…';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Align {
    Left,
    Right,
}

pub(super) fn width(text: &str) -> u16 {
    u16::try_from(Span::raw(text).width()).unwrap_or(u16::MAX)
}

pub(super) fn fit(text: &str, max: u16) -> String {
    if width(text) <= max {
        return text.to_owned();
    }
    let mut kept = String::new();
    let mut used = 0;
    for character in text.chars() {
        let next = used + width(character.encode_utf8(&mut [0; 4]));
        if next >= max {
            break;
        }
        used = next;
        kept.push(character);
    }
    if max > 0 {
        kept.push(ELLIPSIS);
    }
    kept
}

pub(super) fn put(buffer: &mut Buffer, area: Rect, text: &str, align: Align, style: Style) {
    let shown = fit(text, area.width);
    let x = match align {
        Align::Left => area.x,
        Align::Right => area.x + area.width.saturating_sub(width(&shown)),
    };
    buffer.set_stringn(x, area.y, shown, usize::from(area.width), style);
}
