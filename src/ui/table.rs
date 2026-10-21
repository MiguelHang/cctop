use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use super::hits::{Hit, Hits};
use super::text::{Align, put};
use super::view::{Row, RowKind, Sort, View};
use crate::format;

const GAP: u16 = 1;
const NAME_MIN: u16 = 16;
const INDENT: u16 = 2;
const ARROW: u16 = 2;
const SORTED: &str = " ▼";
const OPTIONAL: [(Sort, u16); 5] =
    [(Sort::Memory, 9), (Sort::Cpu, 6), (Sort::Agent, 8), (Sort::Status, 8), (Sort::Context, 9)];

const HOT: Color = Color::Yellow;

pub(super) fn draw(buffer: &mut Buffer, area: Rect, view: &View, hits: &mut Hits) {
    if area.height == 0 {
        return;
    }
    let columns = columns(area);
    draw_header(buffer, &columns, view.sort, area.y, hits);
    let body = Rect { y: area.y + 1, height: area.height - 1, ..area };
    let first = view.scroll.min(view.rows.len());
    let visible = view.rows.iter().enumerate().skip(first).take(usize::from(body.height));
    for (y, (index, row)) in (body.y..).zip(visible) {
        let line = Line { columns: &columns, area: Rect { y, height: 1, ..body }, index };
        draw_row(buffer, &line, row, view.selected == Some(index));
        push_row_hits(hits, &line, row);
    }
}

fn columns(area: Rect) -> Vec<(Sort, Rect)> {
    let mut widths = Vec::new();
    let mut name = area.width;
    for (sort, width) in OPTIONAL {
        let rest = name.saturating_sub(width + GAP);
        if rest < NAME_MIN {
            break;
        }
        name = rest;
        widths.push((sort, width));
    }
    let mut columns = vec![(Sort::Name, Rect { width: name, height: 1, ..area })];
    let mut x = area.x + name;
    for (sort, width) in widths {
        columns.push((sort, Rect { x: x + GAP, y: area.y, width, height: 1 }));
        x += GAP + width;
    }
    columns
}

fn draw_header(buffer: &mut Buffer, columns: &[(Sort, Rect)], sorted: Sort, y: u16, hits: &mut Hits) {
    let style = Style::new().add_modifier(Modifier::BOLD);
    for &(sort, rect) in columns {
        let rect = Rect { y, ..rect };
        let marker = if sort == sorted { SORTED } else { "" };
        put(buffer, rect, &format!("{}{marker}", title(sort)), align(sort), style);
        hits.push(rect, Hit::Sort(sort));
    }
}

fn title(sort: Sort) -> &'static str {
    match sort {
        Sort::Name => "Name",
        Sort::Memory => "Memory",
        Sort::Cpu => "CPU",
        Sort::Agent => "Agent",
        Sort::Status => "Status",
        Sort::Context => "Context",
    }
}

fn align(sort: Sort) -> Align {
    match sort {
        Sort::Memory | Sort::Cpu | Sort::Context => Align::Right,
        Sort::Name | Sort::Agent | Sort::Status => Align::Left,
    }
}

struct Line<'a> {
    columns: &'a [(Sort, Rect)],
    area: Rect,
    index: usize,
}

fn draw_row(buffer: &mut Buffer, line: &Line<'_>, row: &Row, selected: bool) {
    buffer.set_style(line.area, row_style(row, selected));
    for &(sort, rect) in line.columns {
        let rect = Rect { y: line.area.y, ..rect };
        put(buffer, rect, &cell(row, sort), align(sort), Style::new());
    }
}

fn row_style(row: &Row, selected: bool) -> Style {
    let mut style = match row.kind {
        RowKind::Project => Style::new().add_modifier(Modifier::BOLD),
        RowKind::Workspace => Style::new().fg(Color::Cyan),
        RowKind::Tab => Style::new().fg(Color::Blue),
        RowKind::Pane | RowKind::Process => Style::new(),
    };
    if row.hot {
        style = style.fg(HOT);
    }
    if selected {
        style = style.add_modifier(Modifier::REVERSED);
    }
    style
}

fn cell(row: &Row, sort: Sort) -> String {
    match sort {
        Sort::Name => name(row),
        Sort::Memory => format::estimated(format::or_missing(row.memory, format::bytes), row.estimated),
        Sort::Cpu => format::or_missing(row.cpu, format::percent),
        Sort::Agent | Sort::Status | Sort::Context => pane_cell(row, sort),
    }
}

fn pane_cell(row: &Row, sort: Sort) -> String {
    if row.kind != RowKind::Pane {
        return String::new();
    }
    match sort {
        Sort::Agent => format::or_missing(row.agent.clone(), |agent| agent),
        Sort::Status => format::or_missing(row.status.clone(), |status| status),
        _ => format::or_missing(row.context, format::percent),
    }
}

fn name(row: &Row) -> String {
    let indent = " ".repeat(usize::from(row.depth.saturating_mul(INDENT)));
    let arrow = match row.folded {
        Some(true) => "▸ ",
        Some(false) => "▾ ",
        None => "  ",
    };
    format!("{indent}{arrow}{}", row.label)
}

fn push_row_hits(hits: &mut Hits, line: &Line<'_>, row: &Row) {
    if row.folded.is_some() {
        let name = line.columns[0].1;
        let x = name.x.saturating_add(row.depth.saturating_mul(INDENT));
        let width = ARROW.min(name.right().saturating_sub(x));
        hits.push(Rect { x, y: line.area.y, width, height: 1 }, Hit::Fold(line.index));
    }
    hits.push(line.area, Hit::Row(line.index));
}
