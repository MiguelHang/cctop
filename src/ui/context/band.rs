use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use super::super::chart::fill_level;
use super::super::hits::Hits;
use super::super::panel::{self, Label};
use super::super::text::{Align, fit_place, put, width};
use super::super::view::{ContextDetail, ContextSession, View};
use super::Sums;
use crate::format;

const LOADED_CELL: &str = "▓";
const CONVERSATION_CELL: &str = "▒";
const FREE_CELL: &str = "░";
const FREE: Color = Color::DarkGray;
const PERCENT: u16 = 4;
const GAP: u16 = 2;
const BAR_MIN: u16 = 10;
const UNKNOWN: &str = "Context used —";

pub(super) fn draw(buffer: &mut Buffer, area: Rect, view: &View) {
    let inner = panel::draw(buffer, area);
    let top = panel::top(area);
    let title = fit_place(&view.context.title, panel::room(top, top.x));
    panel::left(buffer, top, &[Label::new(title, Style::new().add_modifier(Modifier::BOLD))], &mut Hits::default());
    if inner.is_empty() {
        return;
    }
    let first = Rect { height: 1, ..inner };
    let ContextDetail::Session(session) = &view.context.detail else {
        put(buffer, first, UNKNOWN, Align::Left, Style::new());
        return;
    };
    let sums = Sums::of(session);
    let color = session.percent.map_or(Color::Reset, |percent| fill_level(percent.used));
    let bar = draw_used(buffer, first, session, color);
    if inner.height >= 2 {
        draw_bar(buffer, Rect { y: inner.y + 1, height: 1, ..inner }, session, &sums, color);
    } else if let Some(bar) = bar {
        draw_bar(buffer, bar, session, &sums, color);
    }
    panel::left(buffer, panel::bottom(area), &legend(session, &sums, color), &mut Hits::default());
}

pub(super) fn short(session: &ContextSession) -> String {
    match (session.total, session.percent) {
        (Some(total), Some(percent)) => {
            format!(
                "{} of {} · {}",
                format::tokens(total),
                format::tokens(percent.window),
                format::percent(percent.used)
            )
        }
        (Some(total), None) => format::tokens(total),
        (None, _) => "estimates only".to_owned(),
    }
}

fn used(session: &ContextSession) -> String {
    match (session.total, session.percent) {
        (Some(total), Some(percent)) => {
            format!("Context used {} of {} tokens", format::tokens(total), format::tokens(percent.window))
        }
        (Some(total), None) => format!("Context used {} tokens", format::tokens(total)),
        (None, _) => "Context used: unknown until Claude's next reply".to_owned(),
    }
}

fn draw_used(buffer: &mut Buffer, line: Rect, session: &ContextSession, color: Color) -> Option<Rect> {
    let text = used(session);
    put(buffer, line, &text, Align::Left, Style::new());
    let mut end = line.right();
    if let Some(percent) = session.percent {
        let rect = Rect { x: line.right().saturating_sub(PERCENT), width: PERCENT.min(line.width), ..line };
        let style = Style::new().fg(color).add_modifier(Modifier::BOLD);
        put(buffer, rect, &format::percent(percent.used), Align::Right, style);
        end = rect.x.saturating_sub(GAP);
    }
    let start = line.x.saturating_add(width(&text)).saturating_add(GAP);
    (end.saturating_sub(start) >= BAR_MIN).then(|| Rect { x: start, width: end - start, ..line })
}

fn draw_bar(buffer: &mut Buffer, area: Rect, session: &ContextSession, sums: &Sums, color: Color) {
    let Some(all) = session.percent.map(|percent| percent.window).filter(|window| *window > 0) else {
        return;
    };
    let loaded = share_of(area.width, sums.loaded, all);
    let used = share_of(area.width, sums.used(), all);
    for x in 0..area.width {
        let (symbol, color) = match x {
            _ if x < loaded => (LOADED_CELL, color),
            _ if x < used => (CONVERSATION_CELL, color),
            _ => (FREE_CELL, FREE),
        };
        buffer.set_string(area.x + x, area.y, symbol, Style::new().fg(color));
    }
}

fn share_of(width: u16, part: u64, all: u64) -> u16 {
    let cells = (u128::from(width) * u128::from(part) + u128::from(all) / 2) / u128::from(all);
    u16::try_from(cells).unwrap_or(width).min(width)
}

fn legend(session: &ContextSession, sums: &Sums, color: Color) -> Vec<Label> {
    let style = Style::new().fg(color);
    let cell = |symbol: &str| if session.percent.is_some() { format!("{symbol} ") } else { String::new() };
    let mut labels = vec![
        Label::new(format!("{}loaded on every message {}", cell(LOADED_CELL), format::tokens(sums.loaded)), style),
        Label::new(
            format!("{}this conversation {}", cell(CONVERSATION_CELL), format::tokens(sums.conversation)),
            style,
        ),
    ];
    if let Some(percent) = session.percent {
        let free = percent.window.saturating_sub(session.total.unwrap_or(sums.used()));
        labels.push(Label::new(format!("{FREE_CELL} free {}", format::tokens(free)), Style::new().fg(FREE)));
    }
    labels
}
