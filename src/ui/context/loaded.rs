use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use super::super::panel::BORDER;
use super::super::text::{Align, fit, fit_start, put, width};
use super::super::view::{ContextPart, ContextSession};
use super::Sums;
use crate::format;

const GROUPS_MIN: u16 = 3;
const SEPARATOR: &str = " · ";
const HEADING: &str = "loaded on every message";
const BIG: u64 = 10;
const SPLIT_NOTE: &str = "The split is estimated; the total is Claude's own.";
const NO_TOTAL: &str = "No reply since the last compaction: estimates only.";

pub(super) fn height(session: &ContextSession, inner: Rect) -> u16 {
    let wanted = u16::try_from(lines(session, inner.width).len()).unwrap_or(u16::MAX).saturating_add(1);
    let room = inner.height.saturating_sub(GROUPS_MIN);
    if room < 2 { 0 } else { wanted.min(room) }
}

pub(super) fn draw(buffer: &mut Buffer, area: Rect, part: Rect, session: &ContextSession) {
    if part.height == 0 {
        return;
    }
    let style = Style::new().fg(BORDER);
    let rule = format!("├{}┤", "─".repeat(usize::from(area.width.saturating_sub(2))));
    buffer.set_string(area.x, part.y, rule, style);
    let heading = format!(" {HEADING} ({}) ", format::tokens(Sums::of(session).loaded));
    let title = Rect { x: part.x.saturating_add(1), y: part.y, width: part.width.saturating_sub(1), height: 1 };
    put(buffer, title, &heading, Align::Left, Style::new().add_modifier(Modifier::BOLD));
    for (y, text) in (part.y + 1..part.bottom()).zip(lines(session, part.width)) {
        put(buffer, Rect { y, height: 1, ..part }, &text, Align::Left, Style::new());
    }
}

fn lines(session: &ContextSession, room: u16) -> Vec<String> {
    let mut lines = wrap(&session.loaded, room);
    let sums = Sums::of(session);
    if sums.conversation > 0 {
        lines.push(format!(
            "/compact would free about {}; the {} stays.",
            format::tokens(sums.conversation),
            format::tokens(sums.loaded)
        ));
    }
    lines.extend(biggest(session, sums.conversation, room));
    lines.push(if session.total.is_some() { SPLIT_NOTE } else { NO_TOTAL }.to_owned());
    lines
}

fn wrap(parts: &[ContextPart], room: u16) -> Vec<String> {
    if parts.is_empty() {
        return vec!["—".to_owned()];
    }
    let mut lines: Vec<String> = Vec::new();
    for part in parts {
        let piece = format!("{} {}", part.label, format::tokens(part.tokens));
        match lines.last_mut() {
            Some(line) if width(line) + width(SEPARATOR) + width(&piece) <= room => {
                line.push_str(SEPARATOR);
                line.push_str(&piece);
            }
            _ => lines.push(piece),
        }
    }
    lines
}

fn biggest(session: &ContextSession, conversation: u64, room: u16) -> Option<String> {
    let (group, part) = session
        .groups
        .iter()
        .filter_map(|group| Some((group, group.items.first()?)))
        .max_by_key(|(_, part)| part.tokens)?;
    if part.tokens.saturating_mul(BIG) <= conversation {
        return None;
    }
    let tokens = format::tokens(part.tokens);
    let room = room.saturating_sub(width(&format!("One {}, ``, took {tokens}.", group.noun)));
    let label = if part.path { fit_start(&part.label, room) } else { fit(&part.label, room) };
    Some(format!("One {}, `{label}`, took {tokens}.", group.noun))
}
