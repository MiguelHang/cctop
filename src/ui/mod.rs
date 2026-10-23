mod chart;
mod context;
mod dialog;
mod graphs;
mod hits;
mod panel;
mod table;
mod text;
mod view;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

pub(crate) use context::scroll_max as context_scroll_max;
pub(crate) use dialog::{Button, Dialog};
pub(crate) use hits::{Hit, Hits};
pub(crate) use view::{
    AgentRow, ContextDetail, ContextGroup, ContextPart, ContextPercent, ContextSession, ContextView, Graphs, Header,
    Row, RowKind, Sort, View, ViewTab,
};

use panel::Label;
use text::width;

use crate::format;

const TAB_ROOM: u16 = 3;
const TABS: [(ViewTab, &str); 2] = [(ViewTab::Processes, "Processes"), (ViewTab::Context, "Context")];

pub(crate) fn render(frame: &mut Frame<'_>, view: &View) -> Hits {
    let area = frame.area();
    let buffer = frame.buffer_mut();
    let mut hits = Hits::default();
    if area.is_empty() {
        return hits;
    }
    let (band, below) = split(area);
    match view.tab {
        ViewTab::Processes => {
            if !band.is_empty() {
                graphs::draw(buffer, band, view);
            }
            table::draw(buffer, below, view, band.is_empty(), &mut hits);
        }
        ViewTab::Context => {
            if !band.is_empty() {
                context::draw_band(buffer, band, view);
            }
            context::draw(buffer, below, view, band.is_empty(), &mut hits);
        }
    }
    if let Some(dialog) = &view.dialog {
        hits = dialog::draw(buffer, area, dialog);
    }
    hits
}

pub(crate) fn list_height(area: Rect) -> usize {
    usize::from(area.height.saturating_sub(graphs::height(area)).saturating_sub(3))
}

fn split(area: Rect) -> (Rect, Rect) {
    let band = graphs::height(area);
    (Rect { height: band, ..area }, Rect { y: area.y + band, height: area.height - band, ..area })
}

fn keys(width: u16, items: &[&str]) -> String {
    let room = usize::from(width.saturating_sub(6));
    let mut shown = String::new();
    for item in items {
        let next = if shown.is_empty() { (*item).to_owned() } else { format!("{shown}  {item}") };
        if next.chars().count() > room {
            break;
        }
        shown = next;
    }
    shown
}

fn draw_tabs(buffer: &mut Buffer, line: Rect, active: ViewTab, hits: &mut Hits) -> u16 {
    let both = TABS.iter().map(|(_, text)| width(text).saturating_add(TAB_ROOM)).fold(TAB_ROOM, u16::saturating_add);
    let labels: Vec<Label> = TABS
        .into_iter()
        .filter(|(tab, _)| both <= line.width || *tab == active)
        .map(|(tab, text)| {
            let style = if tab == active {
                Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD)
            } else {
                Style::new()
            };
            Label { hit: Some(Hit::Tab(tab)), ..Label::new(text, style) }
        })
        .collect();
    panel::left(buffer, line, &labels, hits)
}

fn summary(view: &View, max: u16, totals: bool) -> Label {
    if let Some(error) = &view.header.error {
        return Label::new(error.clone(), Style::new().fg(Color::Red));
    }
    if !totals {
        return Label::new(String::new(), Style::new());
    }
    let memory = format!("mem {}", format::or_missing(view.header.memory, format::bytes));
    let cpu = format!("cpu {}", format::or_missing(view.header.cpu, format::percent));
    let text = [format!("{memory}  {cpu}"), memory].into_iter().find(|text| width(text) <= max).unwrap_or_default();
    Label::new(text, Style::new())
}

#[cfg(test)]
mod tests;
