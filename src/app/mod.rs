mod agents;
mod context;
mod history;
mod input;
mod kill;
mod sampler;
mod state;
mod terminal;
mod view;

use std::io;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use ratatui::backend::Backend;
use ratatui::crossterm::event::{Event, MouseButton, MouseEventKind};
use ratatui::{Frame, Terminal};

use agents::Agent;
use history::History;
use input::{Flow, Lists};
use kill::Prompt;
use sampler::{Reading, Sampler, Tick, Update};
use state::State;
use terminal::{Crossterm, Events, Screen};
use view::Line;

use crate::config::{self, Config, Loaded};
use crate::context::Sessions;
use crate::cornercase::{Client, CommandRunner};
use crate::error::Result;
use crate::procs::{self, Killer};
use crate::ui::{self, AgentRow, ContextDetail, ContextView, Graphs, Header, Hit, Hits, View, ViewTab};
use crate::usage::{self, Tree};

const POLL: Duration = Duration::from_millis(250);
const STALE_TICKS: u32 = 3;
const STALE_MIN: Duration = Duration::from_secs(5);
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

pub(crate) fn run_tui() -> Result<()> {
    let Loaded { config, warnings } = config::load_system();
    let sampler = Sampler::new(Client::new(CommandRunner::from_env()), procs::system())
        .with_contexts(Sessions::default(), std::env::var_os("HOME"));
    let running = sampler::spawn(sampler, config.interval);
    let sort = Some(state::from_config(config.sort));
    let mut app = App::new(sort, config.interval, Instant::now(), Box::new(procs::Signal));
    app.notice = warnings.into_iter().next();
    app.panes = Some(running.panes);
    app.memory_threshold = config.memory_threshold;
    app.machine = Machine { cpus: procs::cpus(), memory: procs::machine_memory() };
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
    let mut changed = true;
    loop {
        for update in updates.try_iter() {
            app.update(update, Instant::now());
            changed = true;
        }
        changed |= app.check_stale(Instant::now());
        if changed {
            terminal.draw(|frame| app.draw(frame)).map_err(io::Error::other)?;
        }
        changed = false;
        let Some(event) = events.next(POLL)?.filter(|event| !moved(event)) else { continue };
        if app.handle(&event, Instant::now()) == Flow::Quit {
            return Ok(());
        }
        changed = true;
    }
}

fn moved(event: &Event) -> bool {
    matches!(event, Event::Mouse(mouse) if mouse.kind == MouseEventKind::Moved)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Machine {
    cpus: usize,
    memory: Option<u64>,
}

pub(crate) struct App {
    state: State,
    reading: Option<Reading>,
    tree: Tree,
    lines: Vec<Line>,
    agents: Vec<Agent>,
    agent_panes: Vec<u64>,
    history: History,
    machine: Machine,
    memory_threshold: u64,
    clicked: Option<(usize, Instant)>,
    hits: Hits,
    error: Option<String>,
    notice: Option<String>,
    interval: Duration,
    answered: Instant,
    panes: Option<Sender<Option<u64>>>,
    asked: Option<u64>,
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
            agents: Vec::new(),
            agent_panes: Vec::new(),
            history: History::new(interval),
            machine: Machine { cpus: 1, memory: None },
            memory_threshold: Config::default().memory_threshold,
            clicked: None,
            hits: Hits::default(),
            error: None,
            notice: None,
            interval,
            answered: now,
            panes: None,
            asked: None,
            prompt: None,
            killer,
            cctop_group: procs::own_group(),
        }
    }

    fn update(&mut self, update: Update, now: Instant) {
        match update {
            Update::Reading(reading) => self.read(reading, now),
            Update::Context(context) => {
                if let Some(reading) = &mut self.reading {
                    reading.context = context;
                }
            }
        }
    }

    fn read(&mut self, reading: Tick, now: Instant) {
        self.answered = now;
        match reading {
            Ok(reading) => {
                self.reading = Some(reading);
                self.error = None;
                self.rebuild();
                self.history.record(&self.tree);
            }
            Err(error) => self.error = Some(error),
        }
        self.ask_context();
    }

    fn check_stale(&mut self, now: Instant) -> bool {
        let quiet = now.saturating_duration_since(self.answered);
        if quiet <= (self.interval * STALE_TICKS).max(STALE_MIN) {
            return false;
        }
        let error = Some(format!("cornercase has not answered for {}s", quiet.as_secs()));
        let changed = self.error != error;
        self.error = error;
        changed
    }

    fn handle(&mut self, event: &Event, now: Instant) -> Flow {
        if self.prompt.is_some() {
            return self.answer(event);
        }
        let (sort, tab) = (self.state.sort, self.state.tab);
        let double = tab == ViewTab::Processes && self.double_click(event, now);
        let lists = Lists { lines: &self.lines, agents: &self.agent_panes };
        let flow = input::handle(&mut self.state, &lists, &self.hits, event);
        if matches!(event, Event::Key(_) | Event::Mouse(_)) {
            self.notice = None;
        }
        if double {
            self.state.tab = ViewTab::Context;
        }
        if tab == ViewTab::Processes && self.state.tab == ViewTab::Context {
            self.state.agent = agents::candidate(&self.agents, self.state.selected);
        }
        if self.state.sort == sort {
            self.relayout();
        } else {
            self.rebuild();
        }
        self.ask_context();
        if flow == Flow::Kill {
            self.ask_kill();
            return Flow::Continue;
        }
        flow
    }

    fn double_click(&mut self, event: &Event, now: Instant) -> bool {
        let Event::Mouse(mouse) = event else { return false };
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return false;
        }
        let Some(Hit::Row(index)) = self.hits.at(mouse.column, mouse.row) else {
            self.clicked = None;
            return false;
        };
        let again =
            self.clicked.is_some_and(|(at, when)| at == index && now.saturating_duration_since(when) <= DOUBLE_CLICK);
        self.clicked = if again { None } else { Some((index, now)) };
        again && self.lines.get(index).is_some_and(|line| agents::under(&self.agents, line.id).is_some())
    }

    fn context_pane(&self) -> Option<u64> {
        match self.state.tab {
            ViewTab::Context => self.state.agent,
            ViewTab::Processes => self.state.selected.and_then(|id| agents::under(&self.agents, id)),
        }
    }

    fn ask_context(&mut self) {
        let pane = self.context_pane();
        if pane == self.asked {
            return;
        }
        self.asked = pane;
        self.state.context_scroll = 0;
        if let Some(panes) = &self.panes {
            let _ = panes.send(pane);
        }
    }

    fn context_view(&self) -> ContextView {
        let pane = self.context_pane();
        let marked = self.agents.iter().position(|agent| Some(agent.pane) == pane);
        let agent = marked.and_then(|at| self.agents.get(at));
        let node = agent.and_then(|agent| Some((context::pane_node(&self.tree, agent.pane)?, agent.label.as_str())));
        let found = self.reading.as_ref().and_then(|reading| reading.context.as_ref());
        let agents = self
            .agents
            .iter()
            .map(|agent| AgentRow { label: agent.label.clone(), percent: agent.percent.map(f64::from) })
            .collect();
        ContextView { agents, marked, detail: context::view(node, found, self.state.context_scroll) }
    }

    fn rebuild(&mut self) {
        self.tree = self
            .reading
            .as_ref()
            .map(|reading| usage::build(&reading.snapshot, &reading.sample, &reading.cpu, self.state.sort))
            .unwrap_or_default();
        self.agents = agents::agents(&self.tree);
        self.agent_panes = self.agents.iter().map(|agent| agent.pane).collect();
        let listed = self.agents.iter().any(|agent| Some(agent.pane) == self.state.agent);
        if self.state.tab == ViewTab::Context && !listed {
            self.state.agent = self.agents.first().map(|agent| agent.pane);
        }
        self.relayout();
    }

    fn relayout(&mut self) {
        self.lines = view::lines(&self.tree, &self.state.folded);
        self.state.reconcile(&self.lines);
    }

    #[cfg(test)]
    fn view(&self) -> View {
        self.view_with(self.context_view())
    }

    fn view_with(&self, context: ContextView) -> View {
        let error = self.error.clone().or_else(|| self.notice.clone());
        let header = Header { memory: self.tree.totals.memory, cpu: self.tree.totals.cpu, error };
        let dialog = self.prompt.as_ref().map(|prompt| prompt.dialog.clone());
        let cpus = f64::from(u32::try_from(self.machine.cpus.max(1)).unwrap_or(u32::MAX));
        let graphs = Graphs {
            cpu: self.history.cpu_share(cpus),
            memory: self.history.memory(),
            points: self.history.total_points(),
            machine_memory: self.machine.memory,
            row_points: self.history.row_points(),
        };
        let memory_threshold = self.memory_threshold;
        View {
            dialog,
            graphs,
            memory_threshold,
            ..view::view(&self.state, &self.lines, &self.history, header, context)
        }
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        self.state.page = ui::list_height(frame.area()).max(1);
        self.state.reconcile(&self.lines);
        self.ask_context();
        let mut view = self.view_with(self.context_view());
        let most = ui::context_scroll_max(&view, frame.area());
        self.state.context_scroll = self.state.context_scroll.min(most);
        if let ContextDetail::Session(session) = &mut view.context.detail {
            session.scroll = self.state.context_scroll;
        }
        self.hits = ui::render(frame, &view);
    }
}

#[cfg(test)]
mod tests;
