use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;

const ELLIPSIS: char = '…';
const PLACE: &str = " › ";

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
    let mut kept: String = keep(text.chars(), max).into_iter().collect();
    if max > 0 {
        kept.push(ELLIPSIS);
    }
    kept
}

pub(super) fn fit_start(text: &str, max: u16) -> String {
    if width(text) <= max {
        return text.to_owned();
    }
    let kept = keep(text.chars().rev(), max);
    let mut shown = if max > 0 { ELLIPSIS.to_string() } else { String::new() };
    shown.extend(kept.into_iter().rev());
    shown
}

pub(super) fn fit_place(text: &str, max: u16) -> String {
    if width(text) <= max {
        return text.to_owned();
    }
    let rest = text.split_once(PLACE).map_or(text, |(_, rest)| rest);
    fit_start(rest, max)
}

fn keep(characters: impl Iterator<Item = char>, max: u16) -> Vec<char> {
    let mut kept = Vec::new();
    let mut used = 0;
    for character in characters {
        let next = used + width(character.encode_utf8(&mut [0; 4]));
        if next >= max {
            break;
        }
        used = next;
        kept.push(character);
    }
    kept
}

pub(super) fn put(buffer: &mut Buffer, area: Rect, text: &str, align: Align, style: Style) {
    if area.height == 0 {
        return;
    }
    let shown = fit(text, area.width);
    let x = match align {
        Align::Left => area.x,
        Align::Right => area.x + area.width.saturating_sub(width(&shown)),
    };
    buffer.set_stringn(x, area.y, shown, usize::from(area.width), style);
}

#[cfg(test)]
mod tests;
