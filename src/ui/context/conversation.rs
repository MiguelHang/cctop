use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};

use super::super::chart::bar;
use super::super::hits::{Hit, Hits};
use super::super::text::{Align, fit, fit_start, put};
use super::super::view::{ContextGroup, ContextPart, ContextSession};
use super::{Sums, loaded};
use crate::format;

const TOKENS: u16 = 6;
const PERCENT: u16 = 4;
const BAR: u16 = 12;
const BAR_FROM: u16 = 40;
const GAP: u16 = 1;
const ITEM_INDENT: &str = "    ";
const BAR_COLOR: Color = Color::Cyan;

enum Item<'a> {
    Group(usize, &'a ContextGroup),
    Part(&'a ContextPart),
    More(usize, u64),
    Nothing,
}

struct Columns {
    label: Rect,
    tokens: Rect,
    percent: Rect,
    bar: Rect,
}

impl Columns {
    fn of(line: Rect) -> Self {
        let bar = if line.width >= BAR_FROM { BAR } else { 0 };
        let numbers = TOKENS + GAP + PERCENT + if bar > 0 { GAP + bar } else { 0 };
        let label = Rect { width: line.width.saturating_sub(numbers + GAP), ..line };
        let tokens = Rect { x: label.right() + GAP.min(line.width), width: TOKENS.min(line.width), ..line };
        let percent = Rect { x: tokens.right() + GAP, width: PERCENT, ..line };
        let bar = Rect { x: percent.right() + GAP, width: bar, ..line };
        let clip = |rect: Rect| rect.intersection(line);
        Self { label, tokens: clip(tokens), percent: clip(percent), bar: clip(bar) }
    }
}

pub(super) fn draw(buffer: &mut Buffer, area: Rect, session: &ContextSession, hits: &mut Hits) {
    let inner = area.inner(Margin::new(1, 1));
    if inner.is_empty() {
        return;
    }
    let bottom = loaded::height(session, inner);
    let groups = Rect { height: inner.height - bottom, ..inner };
    let items = items(session);
    let first = session.scroll.min(items.len().saturating_sub(usize::from(groups.height)));
    let conversation = Sums::of(session).conversation;
    for (y, item) in (groups.y..groups.bottom()).zip(items.iter().skip(first)) {
        let line = Rect { y, height: 1, ..groups };
        draw_item(buffer, line, item, conversation);
        if let Item::Group(index, group) = item
            && !group.items.is_empty()
        {
            hits.push(line, Hit::Group(*index));
        }
    }
    loaded::draw(buffer, area, Rect { y: groups.bottom(), height: bottom, ..inner }, session);
}

pub(super) fn scroll_max(session: &ContextSession, area: Rect) -> usize {
    let inner = area.inner(Margin::new(1, 1));
    let shown = inner.height.saturating_sub(loaded::height(session, inner));
    items(session).len().saturating_sub(usize::from(shown))
}

fn items(session: &ContextSession) -> Vec<Item<'_>> {
    let mut items = Vec::new();
    for (index, group) in session.groups.iter().enumerate() {
        items.push(Item::Group(index, group));
        if group.open {
            items.extend(group.items.iter().map(Item::Part));
            if group.more > 0 {
                items.push(Item::More(group.more, group.more_tokens));
            }
        }
    }
    if items.is_empty() {
        items.push(Item::Nothing);
    }
    items
}

fn draw_item(buffer: &mut Buffer, line: Rect, item: &Item<'_>, conversation: u64) {
    let columns = Columns::of(line);
    let (label, tokens) = match item {
        Item::Group(_, group) => return draw_group(buffer, &columns, group, conversation),
        Item::Part(part) => (part_label(part, columns.label.width), part.tokens),
        Item::More(count, tokens) => (format!("{ITEM_INDENT}… {count} more"), *tokens),
        Item::Nothing => return put(buffer, line, "—", Align::Left, Style::new()),
    };
    put(buffer, columns.label, &label, Align::Left, Style::new());
    put(buffer, columns.tokens, &format::tokens(tokens), Align::Right, Style::new());
}

fn part_label(part: &ContextPart, room: u16) -> String {
    let indent = u16::try_from(ITEM_INDENT.len()).unwrap_or(0);
    let room = room.saturating_sub(indent);
    let label = if part.path { fit_start(&part.label, room) } else { fit(&part.label, room) };
    format!("{ITEM_INDENT}{label}")
}

#[expect(clippy::cast_precision_loss, reason = "a share is shown with one decimal at most")]
fn draw_group(buffer: &mut Buffer, columns: &Columns, group: &ContextGroup, conversation: u64) {
    let arrow = match (group.items.is_empty(), group.open) {
        (true, _) => "  ",
        (false, true) => "▾ ",
        (false, false) => "▸ ",
    };
    put(buffer, columns.label, &format!("{arrow}{}", group.name), Align::Left, Style::new());
    put(buffer, columns.tokens, &format::tokens(group.tokens), Align::Right, Style::new());
    let share = (conversation > 0).then(|| group.tokens as f64 / conversation as f64 * 100.0);
    put(buffer, columns.percent, &format::or_missing(share, format::percent), Align::Right, Style::new());
    bar(buffer, columns.bar, share.unwrap_or(0.0) / 100.0, BAR_COLOR);
}
