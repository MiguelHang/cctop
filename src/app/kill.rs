use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};

use super::App;
use super::input::Flow;
use super::state::RowId;
use crate::error::Error;
use crate::procs::{Pid, Process, Target};
use crate::ui::{Button, Dialog, Hit};

const ONLY_PROCESSES: &str = "select a process to kill it";
const ENDED: &str = "that process has ended";
const ITSELF: &str = "cctop does not kill itself or its process group";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Prompt {
    pub(super) dialog: Dialog,
    started: u64,
}

impl App {
    pub(super) fn ask_kill(&mut self) {
        match self.prompt_for_selected() {
            Ok(prompt) => self.prompt = Some(prompt),
            Err(why) => self.notice = Some(why.to_owned()),
        }
    }

    pub(super) fn answer(&mut self, event: &Event) -> Flow {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => return self.answer_key(*key),
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                if let Some(Hit::Button(button)) = self.hits.at(mouse.column, mouse.row) {
                    self.confirm(button);
                }
            }
            _ => {}
        }
        Flow::Continue
    }

    fn answer_key(&mut self, key: KeyEvent) -> Flow {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Flow::Quit,
            KeyCode::Esc => self.prompt = None,
            KeyCode::Enter => {
                if let Some(focus) = self.prompt.as_ref().map(|prompt| prompt.dialog.focus) {
                    self.confirm(focus);
                }
            }
            KeyCode::Left | KeyCode::BackTab => self.move_focus(false),
            KeyCode::Right | KeyCode::Tab => self.move_focus(true),
            _ => {}
        }
        Flow::Continue
    }

    fn move_focus(&mut self, forward: bool) {
        if let Some(prompt) = &mut self.prompt {
            prompt.dialog.focus = next_button(prompt.dialog.focus, forward);
        }
    }

    fn confirm(&mut self, button: Button) {
        let Some(Prompt { dialog, started }) = self.prompt.take() else { return };
        if button == Button::Cancel {
            return;
        }
        let Some(process) = self.still_running(&dialog, started) else {
            self.notice = Some(ENDED.to_owned());
            return;
        };
        if self.is_cctop(process) {
            self.notice = Some(ITSELF.to_owned());
            return;
        }
        let target = Target { pid: process.pid, started, pgid: process.pgid };
        let sent = if button == Button::Group { self.killer.term_group(target) } else { self.killer.term(target) };
        match sent {
            Ok(()) => {}
            Err(Error::Ended) => self.notice = Some(ENDED.to_owned()),
            Err(error) => self.notice = Some(format!("cannot kill {}: {error}", dialog.process)),
        }
    }

    fn still_running(&self, dialog: &Dialog, started: u64) -> Option<&Process> {
        self.processes()
            .iter()
            .find(|process| process.pid == dialog.pid && process.started == started && process.pgid == dialog.pgid)
    }

    fn is_cctop(&self, process: &Process) -> bool {
        let own: Pid = std::process::id();
        process.pid == own || self.cctop_group.is_none_or(|group| group == process.pgid)
    }

    fn prompt_for_selected(&self) -> Result<Prompt, &'static str> {
        let Some(RowId::Process { pane, pid }) = self.state.selected else { return Err(ONLY_PROCESSES) };
        let processes = self.processes();
        let process = processes.iter().find(|process| process.pid == pid).ok_or(ENDED)?;
        if self.is_cctop(process) {
            return Err(ITSELF);
        }
        let dialog = Dialog {
            process: self.label(RowId::Process { pane, pid }).unwrap_or_else(|| process.name.clone()),
            pid,
            pgid: process.pgid,
            pane: self.label(RowId::Pane(pane)).unwrap_or_else(|| format!("%{pane}")),
            focus: Button::default(),
        };
        Ok(Prompt { dialog, started: process.started })
    }

    fn processes(&self) -> &[Process] {
        self.reading.as_ref().map(|reading| reading.sample.processes.as_slice()).unwrap_or_default()
    }

    fn label(&self, id: RowId) -> Option<String> {
        self.lines.iter().find(|line| line.id == id).map(|line| line.row.label.clone())
    }
}

fn next_button(focus: Button, forward: bool) -> Button {
    match (focus, forward) {
        (Button::Cancel, true) | (Button::Group, false) => Button::Process,
        (Button::Process, true) | (Button::Cancel, false) => Button::Group,
        (Button::Group, true) | (Button::Process, false) => Button::Cancel,
    }
}

#[cfg(test)]
mod tests;
