use std::io::stdout;
use std::time::Duration;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::crossterm::execute;

use crate::error::Result;

pub(crate) trait Events {
    fn next(&mut self, timeout: Duration) -> Result<Option<Event>>;
}

pub(crate) struct Crossterm;

impl Events for Crossterm {
    fn next(&mut self, timeout: Duration) -> Result<Option<Event>> {
        if event::poll(timeout)? { Ok(Some(event::read()?)) } else { Ok(None) }
    }
}

pub(crate) struct Screen {
    pub(super) terminal: DefaultTerminal,
}

impl Screen {
    pub(super) fn enter() -> Result<Self> {
        let terminal = ratatui::try_init()?;
        let screen = Self { terminal };
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            release_mouse();
            hook(info);
        }));
        execute!(stdout(), EnableMouseCapture)?;
        Ok(screen)
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        release_mouse();
        ratatui::restore();
    }
}

fn release_mouse() {
    let _ = execute!(stdout(), DisableMouseCapture);
}
