use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use super::chart::{braille, resample};
use super::hits::Hits;
use super::panel::{self, Label};
use super::view::{Graphs, View};
use crate::format;

const MIN_WIDTH: u16 = 60;
const SHORT: u16 = 16;
const TALL: u16 = 30;
const SPAN: &str = "-60m";
const FULL_CPU: f64 = 100.0;

pub(super) fn height(area: Rect) -> u16 {
    if area.width < MIN_WIDTH || area.height < SHORT {
        0
    } else if area.height < TALL {
        3
    } else {
        4
    }
}

pub(super) fn draw(buffer: &mut Buffer, area: Rect, view: &View) {
    let half = area.width / 2;
    let left = Rect { width: half, ..area };
    let right = Rect { x: area.x + half, width: area.width - half, ..area };
    let graphs = &view.graphs;
    let cpu = format::or_missing(graphs.cpu.last().copied(), format::percent);
    draw_box(buffer, left, &Series { name: "cpu", value: cpu, points: &graphs.cpu, max: FULL_CPU }, graphs.points);
    let memory: Vec<f64> = graphs.memory.iter().map(|&bytes| as_f64(bytes)).collect();
    let peak = memory.iter().fold(0.0_f64, |most, value| most.max(*value));
    let max = graphs.machine_memory.map_or(peak, as_f64);
    let value = memory_title(view.header.memory, graphs);
    draw_box(buffer, right, &Series { name: "mem", value, points: &memory, max }, graphs.points);
}

fn memory_title(memory: Option<u64>, graphs: &Graphs) -> String {
    let used = format::or_missing(memory, format::bytes);
    match graphs.machine_memory {
        Some(machine) => format!("{used} / {}", format::bytes(machine)),
        None => used,
    }
}

#[expect(clippy::cast_precision_loss, reason = "a graph has a few dozen levels")]
fn as_f64(bytes: u64) -> f64 {
    bytes as f64
}

struct Series<'a> {
    name: &'static str,
    value: String,
    points: &'a [f64],
    max: f64,
}

fn draw_box(buffer: &mut Buffer, area: Rect, series: &Series<'_>, capacity: usize) {
    let inner = panel::draw(buffer, area);
    let top = panel::top(area);
    let name = Label::new(series.name, Style::new().add_modifier(Modifier::BOLD));
    let from = panel::left(buffer, top, &[name], &mut Hits::default());
    panel::right(buffer, top, from, &Label::new(series.value.clone(), Style::new()));
    panel::right(buffer, panel::bottom(area), area.x, &Label::new(SPAN, Style::new()));
    let values = resample(series.points, usize::from(inner.width) * 2, capacity);
    braille(buffer, inner, &values, series.max);
}
