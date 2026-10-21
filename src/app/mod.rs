mod input;
mod kill;
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
use kill::Prompt;
use sampler::{Reading, Sampler, Update};
use state::State;
use terminal::{Crossterm, Events, Screen};
use view::Line;

use crate::config::{self, Loaded};
use crate::cornercase::{Client, CommandRunner};
use crate::error::Result;
use crate::procs::{self, Killer};
use crate::ui::{self, Header, Hits, View};
use crate::usage::{self, Tree};

const POLL: Duration = Duration::from_millis(250);
const STALE_TICKS: u32 = 3;
const STALE_MIN: Duration = Duration::from_secs(5);

pub(crate) fn run_tui() -> Result<()> {
    let Loaded { config, warnings } = config::load_system();
    let sampler = Sampler::new(Client::new(CommandRunner::from_env()), procs::system());
    let running = sampler::spawn(sampler, config.interval);
    let sort = Some(state::from_config(config.sort));
    let mut app = App::new(sort, config.interval, Instant::now(), Box::new(procs::Signal));
    app.notice = warnings.into_iter().next();
    let mut screen = Screen::enter()?;
    run_loop(&mut screen.terminal, &mut app, &mut Crossterm, &running.updates)
}

fn run_loop<B>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    events: &mut impl Events,
    updates: &Receiver<Update>,
) -> Result<()>
where
    B: Backend,
    B::Error: Send + Sync + 'static,
{
    loop {
        for update in updates.try_iter() {
            app.update(update, Instant::now());
        }
        app.check_stale(Instant::now());
        terminal.draw(|frame| app.draw(frame)).map_err(io::Error::other)?;
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
    prompt: Option<Prompt>,
    killer: Box<dyn Killer>,
    cctop_group: Option<procs::Pid>,
}

impl App {
    fn new(sort: Option<usage::Sort>, interval: Duration, now: Instant, killer: Box<dyn Killer>) -> Self {
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
            prompt: None,
            killer,
            cctop_group: procs::own_group(),
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
        if self.prompt.is_some() {
            return self.answer(event);
        }
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
        if flow == Flow::Kill {
            self.ask_kill();
            return Flow::Continue;
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
        let dialog = self.prompt.as_ref().map(|prompt| prompt.dialog.clone());
        View { dialog, ..view::view(&self.state, &self.lines, header) }
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        self.state.page = usize::from(frame.area().height.saturating_sub(2)).max(1);
        self.state.reconcile(&self.lines);
        self.hits = ui::render(frame, &self.view());
    }
}

#[cfg(test)]
mod tests;
