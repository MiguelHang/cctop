mod band;
mod conversation;
mod loaded;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use super::chart::{bar, fill_level};
use super::hits::{Hit, Hits};
use super::panel::{self, Label};
use super::text::{Align, fit, fit_place, put, width};
use super::view::{AgentRow, ContextDetail, ContextSession, View, ViewTab};
use super::{draw_tabs, keys, split, summary};
use crate::format;

const SIDE_BY_SIDE: u16 = 70;
const LIST_MIN: u16 = 30;
const LIST_MAX: u16 = 44;
const MARKER: &str = "▶ ";
const MARKER_WIDTH: u16 = 2;
const BAR: u16 = 5;
const PERCENT: u16 = 4;
const BAR_FROM: u16 = 32;
const AGENTS_HELP: &str = "↑↓ choose agent";
const SORTED: &str = "sorted by context used";
const HELP: [&str; 3] = ["⏎ open/fold all", "Tab view", "q/Esc close"];
const EMPTY_HELP: [&str; 2] = ["Tab view", "q/Esc close"];
const TITLE: &str = "what fills this conversation";

pub(super) struct Sums {
    pub(super) loaded: u64,
    pub(super) conversation: u64,
}

impl Sums {
    pub(super) fn of(session: &ContextSession) -> Self {
        let loaded = session.loaded.iter().map(|part| part.tokens).sum();
        let conversation = session.groups.iter().map(|group| group.tokens).sum();
        Self { loaded, conversation }
    }

    pub(super) fn used(&self) -> u64 {
        self.loaded + self.conversation
    }
}

pub(super) fn draw_band(buffer: &mut Buffer, area: Rect, view: &View) {
    band::draw(buffer, area, view);
}

pub(super) fn draw(buffer: &mut Buffer, area: Rect, view: &View, compact: bool, hits: &mut Hits) {
    let (list, detail) = boxes(area, view.context.agents.len());
    draw_agents(buffer, list, view, hits);
    let inner = panel::draw(buffer, detail);
    draw_title(buffer, panel::top(detail), view, compact);
    let help: &[&str] = if matches!(view.context.detail, ContextDetail::Session(_)) { &HELP } else { &EMPTY_HELP };
    panel::left(buffer, panel::bottom(detail), &[Label::new(keys(detail.width, help), Style::new())], hits);
    let message = match &view.context.detail {
        ContextDetail::Session(session) => return conversation::draw(buffer, detail, session, hits),
        ContextDetail::NoAgent => "No Claude agent in cornercase's panes.".to_owned(),
        ContextDetail::Reading { pane } => format!("Reading the Claude session of {pane}…"),
        ContextDetail::NoClaude { pane } => format!("{pane} runs no claude process."),
        ContextDetail::NoSession { pane, pid } => format!("No Claude session found for claude (pid {pid}) in {pane}."),
    };
    put(buffer, Rect { height: inner.height.min(1), ..inner }, &message, Align::Left, Style::new());
}

fn draw_title(buffer: &mut Buffer, top: Rect, view: &View, compact: bool) {
    let session = match &view.context.detail {
        ContextDetail::Session(session) => Some(session),
        _ => None,
    };
    let right = match (&view.header.error, session) {
        (Some(_), _) => summary(view, 0, false),
        (None, Some(session)) if compact => Label::new(band::short(session), Style::new()),
        (None, _) => Label::new(String::new(), Style::new()),
    };
    let title = match session {
        Some(session) => format!("{TITLE} ({})", format::tokens(Sums::of(session).conversation)),
        None => TITLE.to_owned(),
    };
    let reserved = if right.text.is_empty() { 0 } else { width(&right.text).saturating_add(3) };
    let title = fit(&title, panel::room(top, top.x).saturating_sub(reserved));
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let from = panel::left(buffer, top, &[Label::new(title, bold)], &mut Hits::default());
    if view.header.error.is_some() || width(&right.text) <= panel::room(top, from) {
        panel::right(buffer, top, from, &right);
    }
}

pub(crate) fn scroll_max(view: &View, area: Rect) -> usize {
    let ContextDetail::Session(session) = &view.context.detail else { return 0 };
    let (_, below) = split(area);
    let (_, detail) = boxes(below, view.context.agents.len());
    conversation::scroll_max(session, detail)
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

fn draw_agents(buffer: &mut Buffer, area: Rect, view: &View, hits: &mut Hits) {
    let inner = panel::draw(buffer, area);
    draw_tabs(buffer, panel::top(area), ViewTab::Context, hits);
    panel::left(buffer, panel::bottom(area), &[Label::new(AGENTS_HELP, Style::new())], hits);
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
    if agents.len() < shown {
        let last = Rect { y: inner.bottom() - 1, height: 1, ..inner };
        put(buffer, last, SORTED, Align::Left, Style::new().fg(Color::DarkGray));
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
    let color = agent.percent.map_or(Color::Reset, fill_level);
    put(buffer, percent, &format::or_missing(agent.percent, format::percent), Align::Right, Style::new().fg(color));
    if line.width >= BAR_FROM {
        let fraction = agent.percent.unwrap_or(0.0) / 100.0;
        let area = Rect { x: line.right() - PERCENT - 1 - BAR, width: BAR, ..line };
        bar(buffer, area, fraction, color);
    }
}

#[cfg(test)]
mod tests;
