mod input;
mod sampler;
mod state;
mod terminal;
mod view;

use std::io;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use ratatui::backend::Backend;
use ratatui::crossterm::event::Event;
use ratatui::{Frame, Terminal};

use input::Flow;
use sampler::{Reading, Sampler, Update};
use state::State;
use terminal::{Crossterm, Events, Screen};
use view::Line;

use crate::config::{Config, Loaded};
use crate::cornercase::{Client, CommandRunner};
use crate::error::Result;
use crate::paths;
use crate::ui::{self, Header, Hits, View};
use crate::usage::{self, Tree};

const POLL: Duration = Duration::from_millis(250);
const STALE_TICKS: u32 = 3;
const STALE_MIN: Duration = Duration::from_secs(5);

pub(crate) fn run_tui() -> Result<()> {
    let Loaded { config, warnings } = match paths::config_file(paths::system_env) {
        Some(path) => Config::load(&path),
        None => Loaded { config: Config::default(), warnings: Vec::new() },
    };
    let sampler = Sampler::new(Client::new(CommandRunner::from_env()), source());
    let running = sampler::spawn(sampler, config.interval);
    let mut app = App::new(Some(state::from_config(config.sort)), config.interval, Instant::now());
    app.notice = warnings.into_iter().next();
    let mut screen = Screen::enter()?;
    run_loop(&mut screen.terminal, &mut app, &mut Crossterm, &running.updates)
}

#[cfg(target_os = "linux")]
fn source() -> crate::procs::linux::LinuxSource {
    crate::procs::linux::LinuxSource::new()
}

#[cfg(target_os = "macos")]
fn source() -> crate::procs::macos::MacosSource {
    crate::procs::macos::MacosSource::new()
}

fn run_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    events: &mut impl Events,
    updates: &Receiver<Update>,
) -> Result<()> {
    loop {
        for update in updates.try_iter() {
            app.update(update, Instant::now());
        }
        app.check_stale(Instant::now());
        terminal.draw(|frame| app.draw(frame)).map_err(|error| io::Error::other(error.to_string()))?;
        if let Some(event) = events.next(POLL)?
            && app.handle(&event) == Flow::Quit
        {
            return Ok(());
        }
    }
}

pub(crate) struct App {
    state: State,
    reading: Option<Reading>,
    tree: Tree,
    lines: Vec<Line>,
    hits: Hits,
    error: Option<String>,
    notice: Option<String>,
    interval: Duration,
    answered: Instant,
}

impl App {
    fn new(sort: Option<usage::Sort>, interval: Duration, now: Instant) -> Self {
        Self {
            state: State::new(sort),
            reading: None,
            tree: Tree::default(),
            lines: Vec::new(),
            hits: Hits::default(),
            error: None,
            notice: None,
            interval,
            answered: now,
        }
    }

    fn update(&mut self, update: Update, now: Instant) {
        self.answered = now;
        match update {
            Ok(reading) => {
                self.reading = Some(reading);
                self.error = None;
                self.rebuild();
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn check_stale(&mut self, now: Instant) {
        let quiet = now.saturating_duration_since(self.answered);
        if quiet > (self.interval * STALE_TICKS).max(STALE_MIN) {
            self.error = Some(format!("cornercase has not answered for {}s", quiet.as_secs()));
        }
    }

    fn handle(&mut self, event: &Event) -> Flow {
        let sort = self.state.sort;
        let flow = input::handle(&mut self.state, &self.lines, &self.hits, event);
        if matches!(event, Event::Key(_) | Event::Mouse(_)) {
            self.notice = None;
        }
        if self.state.sort == sort {
            self.relayout();
        } else {
            self.rebuild();
        }
        flow
    }

    fn rebuild(&mut self) {
        self.tree = self
            .reading
            .as_ref()
            .map(|reading| usage::build(&reading.snapshot, &reading.sample, &reading.cpu, self.state.sort))
            .unwrap_or_default();
        self.relayout();
    }

    fn relayout(&mut self) {
        self.lines = view::lines(&self.tree, &self.state.folded);
        self.state.reconcile(&self.lines);
    }

    fn view(&self) -> View {
        let error = self.error.clone().or_else(|| self.notice.clone());
        let header = Header { memory: self.tree.totals.memory, cpu: self.tree.totals.cpu, error };
        view::view(&self.state, &self.lines, header)
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        self.state.page = usize::from(frame.area().height.saturating_sub(2)).max(1);
        self.state.reconcile(&self.lines);
        self.hits = ui::render(frame, &self.view());
    }
}

#[cfg(test)]
mod tests;
