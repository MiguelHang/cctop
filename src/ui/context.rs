use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use super::text::{Align, fit_start, put, width};
use super::view::{ContextPart, ContextPercent, ContextSession, ContextView};
use crate::format;

const HEADER: u16 = 5;
const INDENT: u16 = 2;
const GAP: u16 = 1;
const TOKENS: u16 = 8;
const SHARE: u16 = 5;
const FIXED_CELL: &str = "█";
const DYNAMIC_CELL: &str = "▒";
const FIXED: Color = Color::Cyan;
const DYNAMIC: Color = Color::Magenta;

enum ListLine<'a> {
    Heading(&'static str),
    Part(&'a ContextPart),
    Nothing,
    Blank,
}

pub(super) fn draw(buffer: &mut Buffer, area: Rect, context: &ContextView) {
    if area.height == 0 {
        return;
    }
    let message = match context {
        ContextView::Session(session) => return draw_session(buffer, area, session),
        ContextView::NoPane => "Select a pane in Processes to see what fills its Claude context.".to_owned(),
        ContextView::Reading { pane } => format!("Reading the Claude session of {pane}…"),
        ContextView::NoClaude { pane } => format!("{pane} runs no claude process."),
        ContextView::NoSession { pane, pid } => format!("No Claude session found for claude (pid {pid}) in {pane}."),
    };
    put(buffer, line(area, 0), &message, Align::Left, Style::new());
}

pub(crate) fn scroll_max(context: &ContextView, height: u16) -> usize {
    let ContextView::Session(session) = context else { return 0 };
    let shown = height.saturating_sub(1 + HEADER);
    list(session).len().saturating_sub(usize::from(shown))
}

fn draw_session(buffer: &mut Buffer, area: Rect, session: &ContextSession) {
    let title = format!("{} · claude pid {}", session.pane, session.pid);
    put(buffer, line(area, 0), &title, Align::Left, Style::new().add_modifier(Modifier::BOLD));
    put(buffer, line(area, 1), &total(session), Align::Left, Style::new());
    let fixed = sum(&session.fixed);
    let all = fixed + sum(&session.dynamic);
    draw_bar(buffer, line(area, 2), fixed, all);
    draw_legend(buffer, line(area, 3), fixed, all);
    let body = Rect { y: area.y.saturating_add(HEADER), height: area.height.saturating_sub(HEADER), ..area };
    let lines = list(session);
    let first = session.scroll.min(lines.len().saturating_sub(usize::from(body.height)));
    for (y, entry) in (0..body.height).zip(lines.iter().skip(first)) {
        draw_list_line(buffer, line(body, y), entry, all);
    }
}

fn line(area: Rect, offset: u16) -> Rect {
    let y = area.y.saturating_add(offset);
    if offset >= area.height { Rect { y, height: 0, ..area } } else { Rect { y, height: 1, ..area } }
}

fn total(session: &ContextSession) -> String {
    let Some(total) = session.total else {
        return "Total —  no reply since the last compaction: estimates only".to_owned();
    };
    let text = format!("Total {} tokens", format::thousands(total));
    match session.percent {
        Some(percent) => format!("{text}  {}", percent_text(percent)),
        None => text,
    }
}

fn percent_text(percent: ContextPercent) -> String {
    format!(
        "{} of {}  cornercase {}",
        format::percent(percent.used),
        format::tokens(percent.window),
        format::percent(percent.cornercase)
    )
}

fn sum(parts: &[ContextPart]) -> u64 {
    parts.iter().map(|part| part.tokens).sum()
}

fn draw_bar(buffer: &mut Buffer, area: Rect, fixed: u64, all: u64) {
    if all == 0 || area.height == 0 {
        return;
    }
    let cells = share_of(area.width, fixed, all);
    for x in 0..area.width {
        let (symbol, color) = if x < cells { (FIXED_CELL, FIXED) } else { (DYNAMIC_CELL, DYNAMIC) };
        buffer.set_string(area.x + x, area.y, symbol, Style::new().fg(color));
    }
}

fn share_of(width: u16, part: u64, all: u64) -> u16 {
    let cells = (u128::from(width) * u128::from(part) + u128::from(all) / 2) / u128::from(all);
    u16::try_from(cells).unwrap_or(width)
}

fn draw_legend(buffer: &mut Buffer, area: Rect, fixed: u64, all: u64) {
    let dynamic = all - fixed;
    let fixed_text = format!("{FIXED_CELL} Fixed {} {}", estimate(fixed), share(fixed, all));
    let dynamic_text = format!("{DYNAMIC_CELL} Dynamic {} {}", estimate(dynamic), share(dynamic, all));
    put(buffer, area, &fixed_text, Align::Left, Style::new().fg(FIXED));
    let gap = width(&fixed_text).saturating_add(3);
    let rest = Rect { x: area.x.saturating_add(gap), width: area.width.saturating_sub(gap), ..area };
    put(buffer, rest, &dynamic_text, Align::Left, Style::new().fg(DYNAMIC));
}

fn estimate(tokens: u64) -> String {
    format::estimated(format::tokens(tokens), true)
}

#[expect(clippy::cast_precision_loss, reason = "a share is shown with one decimal at most")]
fn share(part: u64, all: u64) -> String {
    if all == 0 { "—".to_owned() } else { format::percent(part as f64 / all as f64 * 100.0) }
}

fn list(session: &ContextSession) -> Vec<ListLine<'_>> {
    let mut lines = Vec::new();
    for (heading, parts) in [("Fixed", &session.fixed), ("Dynamic, since the last compaction", &session.dynamic)] {
        if !lines.is_empty() {
            lines.push(ListLine::Blank);
        }
        lines.push(ListLine::Heading(heading));
        lines.extend(parts.iter().map(ListLine::Part));
        if parts.is_empty() {
            lines.push(ListLine::Nothing);
        }
    }
    lines
}

fn draw_list_line(buffer: &mut Buffer, area: Rect, entry: &ListLine<'_>, all: u64) {
    let inner = Rect { x: area.x.saturating_add(INDENT), width: area.width.saturating_sub(INDENT), ..area };
    match entry {
        ListLine::Heading(text) => put(buffer, area, text, Align::Left, Style::new().add_modifier(Modifier::BOLD)),
        ListLine::Part(part) => draw_part(buffer, inner, part, all),
        ListLine::Nothing => put(buffer, inner, "—", Align::Left, Style::new()),
        ListLine::Blank => {}
    }
}

fn draw_part(buffer: &mut Buffer, area: Rect, part: &ContextPart, all: u64) {
    let numbers = TOKENS + GAP + SHARE;
    let label_width = area.width.saturating_sub(numbers + GAP);
    let label = if part.path { fit_start(&part.label, label_width) } else { part.label.clone() };
    put(buffer, Rect { width: label_width, ..area }, &label, Align::Left, Style::new());
    let right = area.right();
    let tokens = Rect { x: right.saturating_sub(numbers), width: TOKENS.min(area.width), ..area };
    put(buffer, tokens, &estimate(part.tokens), Align::Right, Style::new());
    let share_area = Rect { x: right.saturating_sub(SHARE), width: SHARE.min(area.width), ..area };
    put(buffer, share_area, &share(part.tokens, all), Align::Right, Style::new());
}

#[cfg(test)]
mod tests;
