use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Clear, Padding, Widget};

use super::hits::{Hit, Hits};
use super::text::{Align, put, width};
use crate::procs::Pid;

const WIDTH: u16 = 48;
const HEIGHT: u16 = 7;
const TITLE: &str = " Kill ";
const BUTTONS: [(Button, &str); 3] =
    [(Button::Cancel, "[Cancel]"), (Button::Process, "[Process]"), (Button::Group, "[Group]")];
const BUTTON_GAP: u16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Button {
    Cancel,
    #[default]
    Process,
    Group,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Dialog {
    pub(crate) process: String,
    pub(crate) pid: Pid,
    pub(crate) pgid: Pid,
    pub(crate) pane: String,
    pub(crate) focus: Button,
}

pub(super) fn draw(buffer: &mut Buffer, area: Rect, dialog: &Dialog) -> Hits {
    let mut hits = Hits::default();
    let rect = centered(area);
    if rect.is_empty() {
        return hits;
    }
    Clear.render(rect, buffer);
    let block = Block::bordered().title(TITLE).padding(Padding::horizontal(1));
    let inner = block.inner(rect);
    block.render(rect, buffer);
    let texts =
        [dialog.process.clone(), format!("pid {}, group {}", dialog.pid, dialog.pgid), format!("pane {}", dialog.pane)];
    for (y, text) in (inner.y..inner.bottom()).zip(texts) {
        put(buffer, Rect { y, height: 1, ..inner }, &text, Align::Left, Style::new());
    }
    if inner.height == HEIGHT - 2 {
        draw_buttons(buffer, Rect { y: inner.bottom() - 1, height: 1, ..inner }, dialog.focus, &mut hits);
    }
    hits
}

fn centered(area: Rect) -> Rect {
    let width = WIDTH.min(area.width);
    let height = HEIGHT.min(area.height);
    Rect { x: area.x + (area.width - width) / 2, y: area.y + (area.height - height) / 2, width, height }
}

fn draw_buttons(buffer: &mut Buffer, line: Rect, focus: Button, hits: &mut Hits) {
    let mut x = line.x;
    for (button, label) in BUTTONS {
        let rect = Rect { x, width: width(label).min(line.right().saturating_sub(x)), ..line };
        let style =
            if button == focus { Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD) } else { Style::new() };
        put(buffer, rect, label, Align::Left, style);
        hits.push(rect, Hit::Button(button));
        x = rect.right().saturating_add(BUTTON_GAP).min(line.right());
    }
}
