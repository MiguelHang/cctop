use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use super::chart::{bar, heat, resample, sparkline};
use super::hits::{Hit, Hits};
use super::panel::{self, Label};
use super::text::{Align, put};
use super::view::{Row, RowKind, Sort, View, ViewTab};
use super::{draw_tabs, summary};
use crate::format;

const GAP: u16 = 1;
const NAME_MIN: u16 = 16;
const NAME_BESIDE_PANES: u16 = 30;
const INDENT: u16 = 2;
const ARROW: u16 = 2;
const SORTED: &str = " ▼";
const FULL_CORE: f64 = 100.0;
const HELP: [&str; 6] = ["↑↓ move", "⏎ fold", "Tab view", "s sort", "k kill", "q/Esc close"];
const HOT: Color = Color::Yellow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    Name,
    Bar,
    Memory,
    Cpu,
    Spark,
    Agent,
    Status,
    Context,
}

const OPTIONAL: [(Column, u16, u16); 7] = [
    (Column::Memory, 9, NAME_MIN),
    (Column::Cpu, 6, NAME_MIN),
    (Column::Bar, 12, NAME_MIN),
    (Column::Spark, 12, NAME_MIN),
    (Column::Context, 9, NAME_BESIDE_PANES),
    (Column::Agent, 8, NAME_BESIDE_PANES),
    (Column::Status, 8, NAME_BESIDE_PANES),
];
const ORDER: [Column; 7] =
    [Column::Bar, Column::Memory, Column::Cpu, Column::Spark, Column::Agent, Column::Status, Column::Context];

pub(super) fn draw(buffer: &mut Buffer, area: Rect, view: &View, totals: bool, hits: &mut Hits) {
    let inner = panel::draw(buffer, area);
    let top = panel::top(area);
    let from = draw_tabs(buffer, top, ViewTab::Processes, hits);
    panel::right(buffer, top, from, &summary(view, panel::room(top, from), totals));
    panel::left(buffer, panel::bottom(area), &[Label::new(help(area.width), Style::new())], hits);
    if inner.height == 0 {
        return;
    }
    let columns = columns(inner);
    draw_header(buffer, &columns, view.sort, inner.y, hits);
    let body = Rect { y: inner.y + 1, height: inner.height - 1, ..inner };
    let first = view.scroll.min(view.rows.len());
    let visible = view.rows.iter().enumerate().skip(first).take(usize::from(body.height));
    for (y, (index, row)) in (body.y..).zip(visible) {
        let line = Line { columns: &columns, area: Rect { y, height: 1, ..body }, index };
        draw_row(buffer, &line, row, view);
        push_row_hits(hits, &line, row);
    }
}

fn help(width: u16) -> String {
    let room = usize::from(width.saturating_sub(6));
    let mut shown = String::new();
    for item in HELP {
        let next = if shown.is_empty() { item.to_owned() } else { format!("{shown}  {item}") };
        if next.chars().count() > room {
            break;
        }
        shown = next;
    }
    shown
}

fn columns(area: Rect) -> Vec<(Column, Rect)> {
    let mut chosen = Vec::new();
    let mut name = area.width;
    for (column, width, least) in OPTIONAL {
        let rest = name.saturating_sub(width + GAP);
        if rest < least {
            break;
        }
        name = rest;
        chosen.push((column, width));
    }
    let mut columns = vec![(Column::Name, Rect { width: name, height: 1, ..area })];
    let mut x = area.x + name;
    for column in ORDER {
        if let Some(&(_, width)) = chosen.iter().find(|(chosen, _)| *chosen == column) {
            columns.push((column, Rect { x: x + GAP, y: area.y, width, height: 1 }));
            x += GAP + width;
        }
    }
    columns
}

fn sort_of(column: Column) -> Sort {
    match column {
        Column::Name => Sort::Name,
        Column::Bar | Column::Memory => Sort::Memory,
        Column::Cpu | Column::Spark => Sort::Cpu,
        Column::Agent => Sort::Agent,
        Column::Status => Sort::Status,
        Column::Context => Sort::Context,
    }
}

fn draw_header(buffer: &mut Buffer, columns: &[(Column, Rect)], sorted: Sort, y: u16, hits: &mut Hits) {
    let style = Style::new().add_modifier(Modifier::BOLD);
    for &(column, rect) in columns {
        let rect = Rect { y, ..rect };
        let is_sorted = sort_of(column) == sorted && !matches!(column, Column::Bar | Column::Spark);
        let marker = if is_sorted { SORTED } else { "" };
        put(buffer, rect, &format!("{}{marker}", title(column)), align(column), style);
        hits.push(rect, Hit::Sort(sort_of(column)));
    }
}

fn title(column: Column) -> &'static str {
    match column {
        Column::Name => "Name",
        Column::Bar => "",
        Column::Memory => "Memory",
        Column::Cpu => "CPU",
        Column::Spark => "CPU 60s",
        Column::Agent => "Agent",
        Column::Status => "Status",
        Column::Context => "Context",
    }
}

fn align(column: Column) -> Align {
    match column {
        Column::Memory | Column::Cpu | Column::Context => Align::Right,
        Column::Name | Column::Bar | Column::Spark | Column::Agent | Column::Status => Align::Left,
    }
}

struct Line<'a> {
    columns: &'a [(Column, Rect)],
    area: Rect,
    index: usize,
}

fn draw_row(buffer: &mut Buffer, line: &Line<'_>, row: &Row, view: &View) {
    buffer.set_style(line.area, row_style(row, view.selected == Some(line.index)));
    let context_shown = line.columns.iter().any(|(column, _)| *column == Column::Context);
    for &(column, rect) in line.columns {
        let rect = Rect { y: line.area.y, ..rect };
        match column {
            Column::Name => put(buffer, rect, &name(row, !context_shown), Align::Left, Style::new()),
            Column::Bar => memory_bar(buffer, rect, row, view),
            Column::Spark => cpu_sparkline(buffer, rect, row, view.graphs.row_points),
            _ => put(buffer, rect, &cell(row, column), align(column), Style::new()),
        }
    }
}

#[expect(clippy::cast_precision_loss, reason = "a bar has a few dozen steps")]
fn memory_bar(buffer: &mut Buffer, area: Rect, row: &Row, view: &View) {
    let (Some(memory), Some(total)) = (row.memory, view.header.memory.filter(|total| *total > 0)) else { return };
    let threshold = view.memory_threshold.max(1) as f64;
    bar(buffer, area, memory as f64 / total as f64, heat(memory as f64 / threshold, 0.5, 1.0));
}

fn cpu_sparkline(buffer: &mut Buffer, area: Rect, row: &Row, points: usize) {
    let values = resample(&row.cpu_history, usize::from(area.width), points);
    let max = values.iter().flatten().fold(FULL_CORE, |most, value| most.max(*value));
    sparkline(buffer, area, &values, max, FULL_CORE);
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

fn cell(row: &Row, column: Column) -> String {
    match column {
        Column::Memory => format::estimated(format::or_missing(row.memory, format::bytes), row.estimated),
        Column::Cpu => format::or_missing(row.cpu, format::percent),
        Column::Agent | Column::Status | Column::Context => pane_cell(row, column),
        Column::Name | Column::Bar | Column::Spark => String::new(),
    }
}

fn pane_cell(row: &Row, column: Column) -> String {
    if row.kind != RowKind::Pane {
        return String::new();
    }
    match column {
        Column::Agent => format::or_missing(row.agent.clone(), |agent| agent),
        Column::Status => format::or_missing(row.status.clone(), |status| status),
        _ => format::or_missing(row.context, format::percent),
    }
}

fn name(row: &Row, with_context: bool) -> String {
    let indent = " ".repeat(usize::from(row.depth.saturating_mul(INDENT)));
    let arrow = match row.folded {
        Some(true) => "▸ ",
        Some(false) => "▾ ",
        None => "  ",
    };
    match row.context.filter(|_| with_context && row.kind == RowKind::Pane) {
        Some(context) => format!("{indent}{arrow}{} · {}", row.label, format::percent(context)),
        None => format!("{indent}{arrow}{}", row.label),
    }
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
