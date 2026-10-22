use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use super::state::State;
use super::view::Line;
use crate::ui::{Hit, Hits, ViewTab};

const WHEEL: isize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    Continue,
    Quit,
    Kill,
}

pub(crate) struct Lists<'a> {
    pub(super) lines: &'a [Line],
    pub(super) agents: &'a [u64],
}

pub(super) fn handle(state: &mut State, lists: &Lists<'_>, hits: &Hits, event: &Event) -> Flow {
    match event {
        Event::Key(key) if key.kind != KeyEventKind::Release => key_press(state, lists, *key),
        Event::Mouse(mouse) => self::mouse(state, lists, hits, *mouse),
        _ => Flow::Continue,
    }
}

fn key_press(state: &mut State, lists: &Lists<'_>, key: KeyEvent) -> Flow {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Flow::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Flow::Quit,
        KeyCode::Tab | KeyCode::BackTab => state.next_tab(),
        KeyCode::Char('k') if state.tab == ViewTab::Processes => return Flow::Kill,
        _ if state.tab == ViewTab::Processes => list_key(state, lists.lines, key.code),
        KeyCode::Up => state.step_agent(lists.agents, -1),
        KeyCode::Down => state.step_agent(lists.agents, 1),
        _ => {}
    }
    Flow::Continue
}

fn list_key(state: &mut State, lines: &[Line], code: KeyCode) {
    let page = isize::try_from(state.page.max(1)).unwrap_or(isize::MAX);
    match code {
        KeyCode::Char('s') => state.next_sort(),
        KeyCode::Up => state.step(lines, -1),
        KeyCode::Down => state.step(lines, 1),
        KeyCode::PageUp => state.step(lines, -page),
        KeyCode::PageDown => state.step(lines, page),
        KeyCode::Enter => fold_selected(state, lines, None),
        KeyCode::Left => fold_selected(state, lines, Some(true)),
        KeyCode::Right => fold_selected(state, lines, Some(false)),
        _ => {}
    }
}

fn fold_selected(state: &mut State, lines: &[Line], fold: Option<bool>) {
    let Some(line) = state.index(lines).and_then(|at| lines.get(at)) else { return };
    if line.row.folded.is_none() {
        return;
    }
    match fold {
        Some(fold) => state.fold(line.id, fold),
        None => state.toggle(line.id),
    }
}

fn mouse(state: &mut State, lists: &Lists<'_>, hits: &Hits, mouse: MouseEvent) -> Flow {
    if state.tab != ViewTab::Processes {
        context_mouse(state, lists.agents, hits, mouse);
        return Flow::Continue;
    }
    let lines = lists.lines;
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => click(state, lines, hits.at(mouse.column, mouse.row)),
        MouseEventKind::Down(MouseButton::Right) => return right_click(state, lines, hits.at(mouse.column, mouse.row)),
        MouseEventKind::ScrollDown => state.scroll_by(lines, WHEEL),
        MouseEventKind::ScrollUp => state.scroll_by(lines, -WHEEL),
        _ => {}
    }
    Flow::Continue
}

fn right_click(state: &mut State, lines: &[Line], hit: Option<Hit>) -> Flow {
    let Some(Hit::Row(index) | Hit::Fold(index)) = hit else { return Flow::Continue };
    state.select(lines, index);
    Flow::Kill
}

fn context_mouse(state: &mut State, agents: &[u64], hits: &Hits, mouse: MouseEvent) {
    match (mouse.kind, hits.at(mouse.column, mouse.row)) {
        (MouseEventKind::Down(MouseButton::Left), Some(Hit::Tab(tab))) => state.tab = tab,
        (MouseEventKind::Down(MouseButton::Left), Some(Hit::Agent(index))) => {
            if let Some(&pane) = agents.get(index) {
                state.agent = Some(pane);
            }
        }
        (MouseEventKind::ScrollDown, _) => state.context_scroll = state.context_scroll.saturating_add_signed(WHEEL),
        (MouseEventKind::ScrollUp, _) => state.context_scroll = state.context_scroll.saturating_add_signed(-WHEEL),
        _ => {}
    }
}

fn click(state: &mut State, lines: &[Line], hit: Option<Hit>) {
    match hit {
        Some(Hit::Tab(tab)) => state.tab = tab,
        Some(Hit::Sort(column)) => state.sort_by(column),
        Some(Hit::Row(index)) => state.select(lines, index),
        Some(Hit::Fold(index)) => {
            if let Some(line) = lines.get(index) {
                state.toggle(line.id);
            }
        }
        Some(Hit::Button(_) | Hit::Agent(_)) | None => {}
    }
}

#[cfg(test)]
mod tests;
