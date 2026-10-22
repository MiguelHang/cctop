use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

const BRAILLE: u32 = 0x2800;
const DOTS: u16 = 4;
const LEFT_DOTS: [u32; 4] = [0x40, 0x04, 0x02, 0x01];
const RIGHT_DOTS: [u32; 4] = [0x80, 0x20, 0x10, 0x08];
const SPARKS: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
const EIGHTHS: [&str; 8] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
const FULL: &str = "█";
const EMPTY: &str = "░";
const EMPTY_COLOR: Color = Color::DarkGray;

pub(super) fn heat(fraction: f64, warm: f64, hot: f64) -> Color {
    if fraction >= hot {
        Color::Red
    } else if fraction >= warm {
        Color::Yellow
    } else {
        Color::Green
    }
}

pub(super) fn resample<T: Copy + Into<Option<f64>>>(points: &[T], slots: usize, capacity: usize) -> Vec<Option<f64>> {
    let capacity = capacity.max(1);
    let newest: Vec<Option<f64>> = points.iter().rev().take(capacity).map(|point| (*point).into()).collect();
    let mut values: Vec<Option<f64>> = (0..slots)
        .map(|slot| {
            let start = slot * capacity / slots;
            let end = ((slot + 1) * capacity / slots).max(start + 1).min(newest.len());
            average(newest.get(start..end).unwrap_or_default())
        })
        .collect();
    values.reverse();
    values
}

#[expect(clippy::cast_precision_loss, reason = "a slot averages a few thousand points at most")]
fn average(values: &[Option<f64>]) -> Option<f64> {
    let read: Vec<f64> = values.iter().flatten().copied().collect();
    (!read.is_empty()).then(|| read.iter().sum::<f64>() / read.len() as f64)
}

pub(super) fn braille(buffer: &mut Buffer, area: Rect, values: &[Option<f64>], max: f64) {
    let levels = area.height.saturating_mul(DOTS);
    for (x, pair) in (area.x..area.right()).zip(values.chunks(2)) {
        let left = level(pair[0], max, levels);
        let right = level(pair.get(1).copied().flatten(), max, levels);
        let peak = pair.iter().flatten().fold(0.0_f64, |most, value| most.max(*value));
        let style = Style::new().fg(heat(peak / max, 0.5, 0.8));
        for row in 0..area.height {
            let floor = (area.height - 1 - row) * DOTS;
            let dots = cell(&LEFT_DOTS, left.saturating_sub(floor)) | cell(&RIGHT_DOTS, right.saturating_sub(floor));
            let symbol = if dots == 0 { ' ' } else { char::from_u32(BRAILLE + dots).unwrap_or(' ') };
            buffer.set_string(x, area.y + row, symbol.to_string(), style);
        }
    }
}

fn cell(dots: &[u32; 4], filled: u16) -> u32 {
    dots.iter().take(usize::from(filled)).fold(0, |bits, dot| bits | dot)
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to the levels first")]
fn level(value: Option<f64>, max: f64, levels: u16) -> u16 {
    let Some(value) = value.filter(|value| *value > 0.0 && max > 0.0) else { return 0 };
    let level = (value / max * f64::from(levels)).round().clamp(1.0, f64::from(levels));
    level as u16
}

pub(super) fn sparkline(buffer: &mut Buffer, area: Rect, values: &[Option<f64>], max: f64, hot: f64) {
    let top = u16::try_from(SPARKS.len() - 1).unwrap_or(1);
    for (x, value) in (area.x..area.right()).zip(values) {
        let level = level(*value, max, top);
        let style = Style::new().fg(heat(value.unwrap_or(0.0) / hot, 0.5, 1.0));
        buffer.set_string(x, area.y, SPARKS[usize::from(level)], style);
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "clamped to the bar first"
)]
pub(super) fn bar(buffer: &mut Buffer, area: Rect, fraction: f64, color: Color) {
    if area.is_empty() {
        return;
    }
    let cells = usize::from(area.width);
    let eighths = (fraction.clamp(0.0, 1.0) * (cells * 8) as f64).round() as usize;
    let mut filled = FULL.repeat(eighths / 8);
    filled.push_str(EIGHTHS[eighths % 8]);
    let used = eighths / 8 + usize::from(!eighths.is_multiple_of(8));
    buffer.set_stringn(area.x, area.y, &filled, cells, Style::new().fg(color));
    let rest = EMPTY.repeat(cells.saturating_sub(used));
    let x = area.x + u16::try_from(used).unwrap_or(area.width);
    buffer.set_stringn(x, area.y, rest, cells.saturating_sub(used), Style::new().fg(EMPTY_COLOR));
}

#[cfg(test)]
mod tests;
