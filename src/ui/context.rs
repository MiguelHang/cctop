mod breakdown;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use super::chart::{bar, heat};
use super::hits::{Hit, Hits};
use super::panel::{self, Label};
use super::text::{Align, fit_place, put, width};
use super::view::{AgentRow, ContextDetail, View, ViewTab};
use super::{draw_tabs, summary};
use crate::format;

const SIDE_BY_SIDE: u16 = 70;
const LIST_MIN: u16 = 30;
const LIST_MAX: u16 = 44;
const MARKER: &str = "▶ ";
const MARKER_WIDTH: u16 = 2;
const BAR: u16 = 5;
const PERCENT: u16 = 4;
const BAR_FROM: u16 = 32;
const HELP: &str = "↑↓ choose";

pub(super) fn draw(buffer: &mut Buffer, area: Rect, view: &View, hits: &mut Hits) {
    let (list, detail) = boxes(area, view.context.agents.len());
    draw_agents(buffer, list, view, hits);
    let inner = panel::draw(buffer, detail);
    draw_title(buffer, panel::top(detail), view, hits);
    let message = match &view.context.detail {
        ContextDetail::Session(session) => return breakdown::draw(buffer, detail, session),
        ContextDetail::NoAgent => "No Claude agent in cornercase's panes.".to_owned(),
        ContextDetail::Reading { pane } => format!("Reading the Claude session of {pane}…"),
        ContextDetail::NoClaude { pane } => format!("{pane} runs no claude process."),
        ContextDetail::NoSession { pane, pid } => format!("No Claude session found for claude (pid {pid}) in {pane}."),
    };
    put(buffer, Rect { height: inner.height.min(1), ..inner }, &message, Align::Left, Style::new());
}

fn draw_title(buffer: &mut Buffer, top: Rect, view: &View, hits: &mut Hits) {
    let right = match &view.header.error {
        Some(_) => summary(view, 0, false),
        None => Label::new(breakdown::summary(&view.context.detail), Style::new()),
    };
    let reserved = if right.text.is_empty() { 0 } else { width(&right.text).saturating_add(3) };
    let room = panel::room(top, top.x).saturating_sub(reserved);
    let title = fit_place(&title(&view.context.detail), room);
    let from = panel::left(buffer, top, &[Label::new(title, Style::new().add_modifier(Modifier::BOLD))], hits);
    if view.header.error.is_some() || width(&right.text) <= panel::room(top, from) {
        panel::right(buffer, top, from, &right);
    }
}

pub(crate) fn scroll_max(view: &View, area: Rect) -> usize {
    let ContextDetail::Session(session) = &view.context.detail else { return 0 };
    let (_, detail) = boxes(area, view.context.agents.len());
    breakdown::scroll_max(session, detail)
}

fn boxes(area: Rect, agents: usize) -> (Rect, Rect) {
    if area.width >= SIDE_BY_SIDE {
        let width = (area.width / 3).clamp(LIST_MIN, LIST_MAX);
        return (Rect { width, ..area }, Rect { x: area.x + width, width: area.width - width, ..area });
    }
    let wanted = u16::try_from(agents).unwrap_or(u16::MAX).saturating_add(2);
    let height = wanted.clamp(3, (area.height / 3).max(3)).min(area.height);
    (Rect { height, ..area }, Rect { y: area.y + height, height: area.height - height, ..area })
}

fn title(detail: &ContextDetail) -> String {
    match detail {
        ContextDetail::NoAgent => String::new(),
        ContextDetail::Reading { pane } | ContextDetail::NoClaude { pane } | ContextDetail::NoSession { pane, .. } => {
            pane.clone()
        }
        ContextDetail::Session(session) => session.pane.clone(),
    }
}

fn draw_agents(buffer: &mut Buffer, area: Rect, view: &View, hits: &mut Hits) {
    let inner = panel::draw(buffer, area);
    draw_tabs(buffer, panel::top(area), ViewTab::Context, hits);
    panel::left(buffer, panel::bottom(area), &[Label::new(HELP, Style::new())], hits);
    let agents = &view.context.agents;
    if agents.is_empty() {
        put(buffer, Rect { height: inner.height.min(1), ..inner }, "—", Align::Left, Style::new());
        return;
    }
    let shown = usize::from(inner.height);
    let first = view.context.marked.map_or(0, |marked| (marked + 1).saturating_sub(shown));
    for (y, (index, agent)) in (inner.y..inner.bottom()).zip(agents.iter().enumerate().skip(first)) {
        let line = Rect { y, height: 1, ..inner };
        draw_agent(buffer, line, agent, view.context.marked == Some(index));
        hits.push(line, Hit::Agent(index));
    }
}

fn draw_agent(buffer: &mut Buffer, line: Rect, agent: &AgentRow, marked: bool) {
    let style = if marked { Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD) } else { Style::new() };
    buffer.set_style(line, style);
    if marked {
        put(buffer, line, MARKER, Align::Left, Style::new());
    }
    let numbers = if line.width >= BAR_FROM { BAR + 1 + PERCENT } else { PERCENT };
    let label_width = line.width.saturating_sub(MARKER_WIDTH + numbers + 1);
    let label = Rect { x: line.x + MARKER_WIDTH.min(line.width), width: label_width, ..line };
    put(buffer, label, &fit_place(&agent.label, label_width), Align::Left, Style::new());
    let percent = Rect { x: line.right().saturating_sub(PERCENT), width: PERCENT.min(line.width), ..line };
    put(buffer, percent, &format::or_missing(agent.percent, format::percent), Align::Right, Style::new());
    if line.width >= BAR_FROM {
        let fraction = agent.percent.unwrap_or(0.0) / 100.0;
        let area = Rect { x: line.right() - PERCENT - 1 - BAR, width: BAR, ..line };
        bar(buffer, area, fraction, heat(fraction, 0.5, 0.8));
    }
}

#[cfg(test)]
mod tests;
