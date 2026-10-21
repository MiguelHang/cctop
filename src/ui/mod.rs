mod hits;
mod table;
mod text;
mod view;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

pub(crate) use hits::{Hit, Hits};
pub(crate) use view::{Header, Row, RowKind, Sort, View, ViewTab};

use text::{Align, put, width};

use crate::format;

const TABS: [(ViewTab, &str); 2] = [(ViewTab::Processes, " Processes "), (ViewTab::Context, " Context ")];

pub(crate) fn render(frame: &mut Frame<'_>, view: &View) -> Hits {
    let area = frame.area();
    let buffer = frame.buffer_mut();
    let mut hits = Hits::default();
    if area.height == 0 {
        return hits;
    }
    let tabs_end = draw_tabs(buffer, area, view.tab, &mut hits);
    draw_summary(buffer, Rect { x: tabs_end, width: area.right().saturating_sub(tabs_end), height: 1, ..area }, view);
    let body = Rect { y: area.y + 1, height: area.height - 1, ..area };
    if view.tab == ViewTab::Processes {
        table::draw(buffer, body, view, &mut hits);
    }
    hits
}

fn draw_tabs(buffer: &mut Buffer, area: Rect, active: ViewTab, hits: &mut Hits) -> u16 {
    let mut x = area.x;
    for (tab, label) in TABS {
        let rect = Rect { x, width: width(label).min(area.right().saturating_sub(x)), height: 1, ..area };
        let style =
            if tab == active { Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD) } else { Style::new() };
        put(buffer, rect, label, Align::Left, style);
        hits.push(rect, Hit::Tab(tab));
        x = rect.right();
    }
    x
}

fn draw_summary(buffer: &mut Buffer, area: Rect, view: &View) {
    let area = Rect { x: area.x.saturating_add(1), width: area.width.saturating_sub(2), ..area };
    let (text, style) = match &view.header.error {
        Some(error) => (error.clone(), Style::new().fg(Color::Red)),
        None => (summary(view, area.width), Style::new()),
    };
    put(buffer, area, &text, Align::Right, style);
}

fn summary(view: &View, max: u16) -> String {
    let memory = format!("mem {}", format::or_missing(view.header.memory, format::bytes));
    let cpu = format!("cpu {}", format::or_missing(view.header.cpu, format::percent));
    [format!("{memory}  {cpu}"), memory].into_iter().find(|text| width(text) <= max).unwrap_or_default()
}

#[cfg(test)]
mod tests;
