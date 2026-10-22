use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Modifier, Style};

use super::super::chart::bar;
use super::super::panel::BORDER;
use super::super::text::{Align, fit_start, put, width};
use super::super::view::{ContextDetail, ContextPart, ContextSession};
use crate::format;

const HEADER: u16 = 3;
const TWO_COLUMNS: u16 = 60;
const BAR: u16 = 6;
const BAR_FROM: u16 = 24;
const TOKENS: u16 = 7;
const GAP: u16 = 1;
const LEGEND_GAP: &str = "  ";
const FIXED_CELL: &str = "█";
const DYNAMIC_CELL: &str = "▓";
const FREE_CELL: &str = "░";
const FIXED: Color = Color::Cyan;
const DYNAMIC: Color = Color::Magenta;
const FREE: Color = Color::DarkGray;

enum Item<'a> {
    Part(&'a ContextPart, Color),
    Nothing,
    Heading,
}

struct Sums {
    fixed: u64,
    dynamic: u64,
    largest: u64,
}

impl Sums {
    fn of(session: &ContextSession) -> Self {
        let sum = |parts: &[ContextPart]| parts.iter().map(|part| part.tokens).sum();
        let largest = session.fixed.iter().chain(&session.dynamic).map(|part| part.tokens).max().unwrap_or(0);
        Self { fixed: sum(&session.fixed), dynamic: sum(&session.dynamic), largest }
    }
}

pub(super) fn summary(detail: &ContextDetail) -> String {
    let ContextDetail::Session(session) = detail else { return String::new() };
    match (session.total, session.percent) {
        (Some(total), Some(percent)) => format!("{} · {}", format::tokens(total), format::percent(percent.used)),
        (Some(total), None) => format::tokens(total),
        (None, _) => "no reply since the last compaction: estimates only".to_owned(),
    }
}

pub(super) fn draw(buffer: &mut Buffer, area: Rect, session: &ContextSession) {
    let inner = area.inner(Margin::new(1, 1));
    if inner.is_empty() {
        return;
    }
    let sums = Sums::of(session);
    let window = session.percent.map(|percent| percent.window);
    draw_bar(buffer, line(inner, 0), &sums, window);
    draw_legend(buffer, line(inner, 1), session, &sums);
    let body = Rect { y: inner.y.saturating_add(HEADER), height: inner.height.saturating_sub(HEADER), ..inner };
    if inner.width >= TWO_COLUMNS {
        draw_two_columns(buffer, area, body, session);
    } else {
        draw_one_column(buffer, area, body, session);
    }
}

pub(super) fn scroll_max(session: &ContextSession, area: Rect) -> usize {
    let inner = area.inner(Margin::new(1, 1));
    let shown = usize::from(inner.height.saturating_sub(HEADER));
    let rows = if inner.width >= TWO_COLUMNS {
        session.fixed.len().max(session.dynamic.len()).max(1)
    } else {
        one_column(session).len()
    };
    rows.saturating_sub(shown)
}

fn line(area: Rect, offset: u16) -> Rect {
    let y = area.y.saturating_add(offset);
    Rect { y, height: u16::from(offset < area.height), ..area }
}

fn draw_bar(buffer: &mut Buffer, area: Rect, sums: &Sums, window: Option<u64>) {
    let all = window.unwrap_or(sums.fixed + sums.dynamic);
    if all == 0 || area.height == 0 {
        return;
    }
    let fixed = share_of(area.width, sums.fixed, all);
    let used = share_of(area.width, sums.fixed + sums.dynamic, all);
    for x in 0..area.width {
        let (symbol, color) = match x {
            _ if x < fixed => (FIXED_CELL, FIXED),
            _ if x < used => (DYNAMIC_CELL, DYNAMIC),
            _ => (FREE_CELL, FREE),
        };
        buffer.set_string(area.x + x, area.y, symbol, Style::new().fg(color));
    }
}

fn share_of(width: u16, part: u64, all: u64) -> u16 {
    let cells = (u128::from(width) * u128::from(part) + u128::from(all) / 2) / u128::from(all);
    u16::try_from(cells).unwrap_or(width).min(width)
}

fn draw_legend(buffer: &mut Buffer, area: Rect, session: &ContextSession, sums: &Sums) {
    let mut pieces = vec![
        (format!("{FIXED_CELL} fixed {}", estimate(sums.fixed)), FIXED),
        (format!("{DYNAMIC_CELL} dynamic {}", estimate(sums.dynamic)), DYNAMIC),
    ];
    if let Some(percent) = session.percent {
        let free = percent.window.saturating_sub(sums.fixed + sums.dynamic);
        pieces.push((format!("{FREE_CELL} free {}", format::tokens(free)), FREE));
        pieces.push((format!("cornercase {}", format::percent(percent.cornercase)), Color::Reset));
    }
    let mut x = area.x;
    for (text, color) in pieces {
        let size = width(&text);
        if area.height == 0 || x.saturating_add(size) > area.right() {
            break;
        }
        put(buffer, Rect { x, width: size, ..area }, &text, Align::Left, Style::new().fg(color));
        x = x.saturating_add(size).saturating_add(width(LEGEND_GAP));
    }
}

fn estimate(tokens: u64) -> String {
    format::estimated(format::tokens(tokens), true)
}

fn draw_two_columns(buffer: &mut Buffer, area: Rect, body: Rect, session: &ContextSession) {
    let sums = Sums::of(session);
    let half = body.width / 2;
    let left = Rect { width: half, ..body };
    let right = Rect { x: body.x + half + 1, width: body.width.saturating_sub(half + 1), ..body };
    let divider = body.x + half;
    let y = body.y.saturating_sub(1);
    separator(buffer, area, y, Some(divider));
    heading(buffer, Rect { y, height: 1, ..left }, "fixed", sums.fixed, FIXED);
    heading(buffer, Rect { y, height: 1, ..right }, "dynamic", sums.dynamic, DYNAMIC);
    let rows = session.fixed.len().max(session.dynamic.len()).max(1);
    let first = session.scroll.min(rows.saturating_sub(usize::from(body.height)));
    let fixed = column(&session.fixed, FIXED);
    let dynamic = column(&session.dynamic, DYNAMIC);
    for (y, index) in (body.y..body.bottom()).zip(first..) {
        let style = Style::new().fg(BORDER);
        buffer.set_string(divider, y, "│", style);
        for (items, side) in [(&fixed, left), (&dynamic, right)] {
            if let Some(item) = items.get(index) {
                draw_item(buffer, Rect { y, height: 1, ..side }, item, &sums);
            }
        }
    }
    if body.height > 0 {
        buffer.set_string(divider, area.bottom() - 1, "┴", Style::new().fg(BORDER));
    }
}

fn column(parts: &[ContextPart], color: Color) -> Vec<Item<'_>> {
    if parts.is_empty() { vec![Item::Nothing] } else { parts.iter().map(|part| Item::Part(part, color)).collect() }
}

fn one_column(session: &ContextSession) -> Vec<Item<'_>> {
    let mut items = column(&session.fixed, FIXED);
    items.push(Item::Heading);
    items.extend(column(&session.dynamic, DYNAMIC));
    items
}

fn draw_one_column(buffer: &mut Buffer, area: Rect, body: Rect, session: &ContextSession) {
    let sums = Sums::of(session);
    let y = body.y.saturating_sub(1);
    separator(buffer, area, y, None);
    heading(buffer, Rect { y, height: 1, ..body }, "fixed", sums.fixed, FIXED);
    let items = one_column(session);
    let first = session.scroll.min(items.len().saturating_sub(usize::from(body.height)));
    for (y, item) in (body.y..body.bottom()).zip(items.iter().skip(first)) {
        if let Item::Heading = item {
            separator(buffer, area, y, None);
            heading(buffer, Rect { y, height: 1, ..body }, "dynamic", sums.dynamic, DYNAMIC);
        } else {
            draw_item(buffer, Rect { y, height: 1, ..body }, item, &sums);
        }
    }
}

fn separator(buffer: &mut Buffer, area: Rect, y: u16, joint: Option<u16>) {
    if area.width < 2 || y >= area.bottom() {
        return;
    }
    let style = Style::new().fg(BORDER);
    let line = format!("├{}┤", "─".repeat(usize::from(area.width - 2)));
    buffer.set_string(area.x, y, line, style);
    if let Some(x) = joint {
        buffer.set_string(x, y, "┬", style);
    }
}

fn heading(buffer: &mut Buffer, area: Rect, name: &str, tokens: u64, color: Color) {
    let text = format!(" {name} {} ", estimate(tokens));
    let area = Rect { x: area.x.saturating_add(1), width: area.width.saturating_sub(1), ..area };
    put(buffer, area, &text, Align::Left, Style::new().fg(color).add_modifier(Modifier::BOLD));
}

#[expect(clippy::cast_precision_loss, reason = "a bar has a few dozen steps")]
fn draw_item(buffer: &mut Buffer, area: Rect, item: &Item<'_>, sums: &Sums) {
    let Item::Part(part, color) = item else {
        put(buffer, area, "—", Align::Left, Style::new());
        return;
    };
    let numbers = if area.width >= BAR_FROM { BAR + GAP + TOKENS } else { TOKENS };
    let label_width = area.width.saturating_sub(numbers + GAP);
    let label = if part.path { fit_start(&part.label, label_width) } else { part.label.clone() };
    put(buffer, Rect { width: label_width, ..area }, &label, Align::Left, Style::new());
    let tokens = Rect { x: area.right().saturating_sub(TOKENS), width: TOKENS.min(area.width), ..area };
    put(buffer, tokens, &estimate(part.tokens), Align::Right, Style::new());
    if area.width >= BAR_FROM && sums.largest > 0 {
        let rect = Rect { x: area.right() - TOKENS - GAP - BAR, width: BAR, ..area };
        bar(buffer, rect, part.tokens as f64 / sums.largest as f64, *color);
    }
}
