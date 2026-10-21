use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use super::context::{Found, PaneContext};
use super::sampler::{Reading, Sampler, Update};
use super::state::RowId;
use super::terminal::Events;
use super::{App, Flow, run_loop};
use crate::cornercase::fake::FakeRunner;
use crate::cornercase::{Client, Snapshot};
use crate::error::Result;
use crate::procs::fake::{FakeKiller, FakeSource};
use crate::procs::{Pid, Process, Sample};
use crate::test_util::fixture;
use crate::ui::{ContextView, Sort, ViewTab};
use crate::usage;

pub(super) const STATUS: &str = fixture!("cornercase/status-0.12.173.json");
const BOOT: &str = "4242-1700000000000000000";
const MB: u64 = 1_000_000;
const INTERVAL: Duration = Duration::from_secs(2);

pub(super) fn process(pid: Pid, parent: Pid, pane: u64, memory: u64, cpu_seconds: u64) -> Process {
    Process {
        pid,
        ppid: Some(parent),
        name: format!("proc{pid}"),
        memory: Some(memory * MB),
        cpu_time: Duration::from_secs(cpu_seconds),
        pane: Some(pane),
        server: Some(BOOT.to_owned()),
        ..Process::default()
    }
}

pub(super) fn first_sample(taken: Instant) -> Sample {
    let processes = vec![
        process(100, 10, 1, 5, 0),
        process(101, 100, 1, 400, 0),
        process(200, 10, 2, 5, 0),
        process(201, 200, 2, 80, 0),
        process(300, 10, 7, 5, 0),
    ];
    Sample { taken, processes }
}

fn second_sample(taken: Instant) -> Sample {
    let processes = vec![
        process(100, 10, 1, 5, 0),
        process(101, 100, 1, 400, 1),
        process(200, 10, 2, 5, 0),
        process(202, 200, 2, 90, 0),
        process(300, 10, 7, 5, 0),
    ];
    Sample { taken: taken + Duration::from_secs(1), processes }
}

pub(super) fn fake_sampler() -> Sampler<FakeRunner, FakeSource> {
    let now = Instant::now();
    let runner = FakeRunner::default().answer(&["status", "--json"], STATUS);
    Sampler::new(Client::new(runner), FakeSource::new([first_sample(now), second_sample(now)]))
}

pub(super) fn reading(sample: Sample) -> Reading {
    let snapshot = Snapshot::parse(STATUS).expect("parse the status fixture");
    Reading { snapshot, sample, cpu: std::collections::HashMap::new(), context: None }
}

fn app_with(sample: Sample, sort: Option<usage::Sort>) -> App {
    app_killing_with(sample, sort, FakeKiller::default())
}

pub(super) fn app_killing_with(sample: Sample, sort: Option<usage::Sort>, killer: FakeKiller) -> App {
    let mut app = App::new(sort, INTERVAL, Instant::now(), Box::new(killer));
    app.update(Update::Reading(Ok(reading(sample))), Instant::now());
    app
}

fn labels(app: &App) -> Vec<String> {
    app.view().rows.into_iter().map(|row| row.label).collect()
}

pub(super) fn index_of(app: &App, id: RowId) -> usize {
    app.lines.iter().position(|line| line.id == id).expect("the row is visible")
}

pub(super) fn draw(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal.draw(|frame| app.draw(frame)).expect("draw");
    terminal.backend().buffer().clone()
}

pub(super) fn find(buffer: &Buffer, text: &str) -> (u16, u16) {
    (0..buffer.area.height)
        .find_map(|y| {
            let line: String = (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect();
            let at = line.find(text)?;
            Some((u16::try_from(line[..at].chars().count()).expect("narrow line"), y))
        })
        .expect("the text is on screen")
}

pub(super) fn click(column: u16, row: u16) -> Event {
    mouse(MouseEventKind::Down(MouseButton::Left), column, row)
}

pub(super) fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent { kind, column, row, modifiers: KeyModifiers::NONE })
}

pub(super) fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::from(code))
}

mod ticking {
    use super::*;

    #[test]
    fn a_tick_with_fakes_gives_the_tree_in_cornercase_order() {
        let mut sampler = fake_sampler();
        let mut app = App::new(None, INTERVAL, Instant::now(), Box::new(FakeKiller::default()));
        app.update(Update::Reading(sampler.tick()), Instant::now());
        let expected = [
            "notes",
            "main",
            "claude",
            "%1 claude",
            "proc100",
            "proc101",
            "%2 cargo",
            "proc200",
            "proc201",
            "logs",
            "feature-x (worktree)",
            "claude",
            "%7 claude",
            "proc300",
            "shop",
            "main",
            "shell",
            "%5 zsh",
        ];
        assert_eq!(labels(&app), expected);
        assert_eq!(app.view().header.memory, Some(495 * MB));
    }

    #[test]
    fn the_second_tick_brings_cpu() {
        let mut sampler = fake_sampler();
        let mut app = App::new(None, INTERVAL, Instant::now(), Box::new(FakeKiller::default()));
        app.update(Update::Reading(sampler.tick()), Instant::now());
        assert_eq!(app.view().header.cpu, None);
        app.update(Update::Reading(sampler.tick()), Instant::now());
        let claude = index_of(&app, RowId::Process { pane: 1, pid: 101 });
        assert_eq!(app.view().rows[claude].cpu, Some(100.0));
    }

    #[test]
    fn a_failed_status_shows_the_error_and_keeps_the_last_tree() {
        let now = Instant::now();
        let mut app = app_with(first_sample(now), None);
        app.update(Update::Reading(Err("cornercase: server not running".to_owned())), now);
        let view = app.view();
        assert_eq!(view.header.error.as_deref(), Some("cornercase: server not running"));
        assert_eq!(view.rows.len(), 18);
        app.update(Update::Reading(Ok(reading(second_sample(now)))), now);
        assert_eq!(app.view().header.error, None);
    }

    #[test]
    fn a_status_that_never_answers_is_reported_after_three_intervals() {
        let start = Instant::now();
        let mut app = App::new(None, INTERVAL, start, Box::new(FakeKiller::default()));
        app.check_stale(start + Duration::from_secs(6));
        assert_eq!(app.view().header.error, None);
        app.check_stale(start + Duration::from_secs(7));
        assert_eq!(app.view().header.error.as_deref(), Some("cornercase has not answered for 7s"));
    }

    #[test]
    fn a_config_warning_shows_until_the_first_input() {
        let mut app = app_with(first_sample(Instant::now()), None);
        app.notice = Some("bad config".to_owned());
        assert_eq!(app.view().header.error.as_deref(), Some("bad config"));
        app.handle(&key(KeyCode::Down));
        assert_eq!(app.view().header.error, None);
    }
}

mod refreshing {
    use super::*;

    #[test]
    fn selection_and_folds_survive_a_refresh_with_other_processes() {
        let now = Instant::now();
        let mut app = app_with(first_sample(now), None);
        app.state.toggle(RowId::Pane(1));
        app.relayout();
        let cargo = index_of(&app, RowId::Pane(2));
        app.state.select(&app.lines, cargo);
        app.update(Update::Reading(Ok(reading(second_sample(now)))), now);
        assert_eq!(app.state.selected, Some(RowId::Pane(2)));
        assert_eq!(app.view().selected, Some(index_of(&app, RowId::Pane(2))));
        let pane = index_of(&app, RowId::Pane(1));
        assert_eq!(app.view().rows[pane].folded, Some(true));
        assert_eq!(app.lines[pane + 1].id, RowId::Pane(2));
    }

    #[test]
    fn a_selected_process_that_ends_hands_the_selection_to_the_row_in_its_place() {
        let now = Instant::now();
        let mut app = app_with(first_sample(now), None);
        let gone = index_of(&app, RowId::Process { pane: 2, pid: 201 });
        app.state.select(&app.lines, gone);
        app.update(Update::Reading(Ok(reading(second_sample(now)))), now);
        assert_eq!(app.state.selected, Some(RowId::Process { pane: 2, pid: 202 }));
    }
}

mod mouse_and_keys {
    use super::*;

    #[test]
    fn clicking_the_cpu_header_sorts_by_cpu() {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (x, y) = find(&draw(&mut app, 100, 30), "CPU");
        app.handle(&click(x, y));
        assert_eq!(app.state.sort, Some(usage::Sort::Cpu));
        assert_eq!(app.view().sort, Sort::Cpu);
    }

    #[test]
    fn s_cycles_memory_cpu_and_cornercase_order() {
        let mut app = app_with(first_sample(Instant::now()), Some(usage::Sort::Memory));
        assert_eq!(labels(&app)[0], "notes");
        app.handle(&key(KeyCode::Char('s')));
        assert_eq!(app.view().sort, Sort::Cpu);
        app.handle(&key(KeyCode::Char('s')));
        assert_eq!(app.view().sort, Sort::Name);
        app.handle(&key(KeyCode::Char('s')));
        assert_eq!(app.view().sort, Sort::Memory);
    }

    #[test]
    fn clicking_the_arrow_folds_and_unfolds() {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (x, y) = find(&draw(&mut app, 100, 30), "▾ %1 claude");
        app.handle(&click(x, y));
        assert!(!labels(&app).contains(&"proc100".to_owned()));
        let (x, y) = find(&draw(&mut app, 100, 30), "▸ %1 claude");
        app.handle(&click(x, y));
        assert!(labels(&app).contains(&"proc100".to_owned()));
    }

    #[test]
    fn clicking_a_row_selects_it_and_a_tab_switches_view() {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (_, y) = find(&draw(&mut app, 100, 30), "%2 cargo");
        app.handle(&click(40, y));
        assert_eq!(app.state.selected, Some(RowId::Pane(2)));
        let (x, y) = find(&draw(&mut app, 100, 30), "Context ");
        app.handle(&click(x, y));
        assert_eq!(app.view().tab, ViewTab::Context);
    }

    #[test]
    fn the_wheel_scrolls() {
        let mut app = app_with(first_sample(Instant::now()), None);
        draw(&mut app, 100, 8);
        app.handle(&mouse(MouseEventKind::ScrollDown, 10, 4));
        assert_eq!(app.view().scroll, 3);
        app.handle(&mouse(MouseEventKind::ScrollUp, 10, 4));
        assert_eq!(app.view().scroll, 0);
    }

    #[test]
    fn the_context_tab_leaves_the_process_list_alone() {
        let mut app = app_with(first_sample(Instant::now()), Some(usage::Sort::Memory));
        let (_, y) = find(&draw(&mut app, 100, 30), "%2 cargo");
        app.handle(&click(40, y));
        app.handle(&key(KeyCode::Tab));
        draw(&mut app, 100, 8);
        for event in [key(KeyCode::Down), key(KeyCode::Enter), key(KeyCode::Char('s'))] {
            app.handle(&event);
        }
        app.handle(&mouse(MouseEventKind::ScrollDown, 10, 4));
        app.handle(&click(40, y));
        assert_eq!(app.state.selected, Some(RowId::Pane(2)));
        assert_eq!(app.state.sort, Some(usage::Sort::Memory));
        assert_eq!(app.view().scroll, 0);
        assert!(app.state.folded.is_empty());
        let (x, y) = find(&draw(&mut app, 100, 8), "Processes ");
        app.handle(&click(x, y));
        assert_eq!(app.view().tab, ViewTab::Processes);
    }
}

mod context_view {
    use super::*;

    fn with_context(sample: Sample, context: PaneContext) -> App {
        let mut app = App::new(None, INTERVAL, Instant::now(), Box::new(FakeKiller::default()));
        app.update(Update::Reading(Ok(Reading { context: Some(context), ..reading(sample) })), Instant::now());
        app
    }

    fn select(app: &mut App, id: RowId) {
        let index = index_of(app, id);
        app.state.select(&app.lines, index);
        app.handle(&Event::FocusGained);
    }

    #[test]
    fn selecting_a_row_asks_the_sampler_for_its_pane_once() {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (panes, asked) = mpsc::channel();
        app.panes = Some(panes);
        select(&mut app, RowId::Pane(2));
        select(&mut app, RowId::Process { pane: 2, pid: 201 });
        select(&mut app, RowId::Pane(1));
        select(&mut app, RowId::Project(1));
        assert_eq!(asked.try_iter().collect::<Vec<_>>(), [Some(2), Some(1), None]);
    }

    #[test]
    fn a_context_answer_keeps_the_error_and_the_last_answer_time() {
        let start = Instant::now();
        let mut app = with_context(first_sample(start), PaneContext { pane: 2, found: Found::NoClaude });
        app.update(Update::Reading(Err("cornercase: server not running".to_owned())), start);
        select(&mut app, RowId::Pane(2));
        app.update(Update::Context(Some(PaneContext { pane: 2, found: Found::NoSession(201) })), start + INTERVAL * 9);
        assert_eq!(app.answered, start);
        assert_eq!(app.view().header.error.as_deref(), Some("cornercase: server not running"));
        app.handle(&key(KeyCode::Tab));
        assert_eq!(app.view().context, ContextView::NoSession { pane: "%2 cargo".to_owned(), pid: 201 });
    }

    #[test]
    fn a_selection_moved_outside_the_input_is_asked_on_the_next_draw() {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (panes, asked) = mpsc::channel();
        app.panes = Some(panes);
        let index = index_of(&app, RowId::Pane(2));
        app.state.select(&app.lines, index);
        draw(&mut app, 100, 30);
        assert_eq!(asked.try_iter().collect::<Vec<_>>(), [Some(2)]);
    }

    #[test]
    fn shows_the_selected_pane_and_processes_keeps_the_selection() {
        let mut app = with_context(first_sample(Instant::now()), PaneContext { pane: 2, found: Found::NoClaude });
        select(&mut app, RowId::Pane(2));
        app.handle(&key(KeyCode::Tab));
        assert_eq!(app.view().context, ContextView::NoClaude { pane: "%2 cargo".to_owned() });
        select(&mut app, RowId::Pane(1));
        assert_eq!(app.view().context, ContextView::Reading { pane: "%1 claude".to_owned() });
        app.handle(&key(KeyCode::Tab));
        assert_eq!(app.view().tab, ViewTab::Processes);
        assert_eq!(app.state.selected, Some(RowId::Pane(1)));
    }

    #[test]
    fn the_wheel_scrolls_the_breakdown_up_to_its_end() {
        let breakdown =
            crate::context::breakdown::Breakdown { total: Some(10), fixed: Vec::new(), dynamic: Vec::new() };
        let mut app =
            with_context(first_sample(Instant::now()), PaneContext { pane: 1, found: Found::Session(101, breakdown) });
        select(&mut app, RowId::Pane(1));
        app.handle(&key(KeyCode::Tab));
        app.handle(&mouse(MouseEventKind::ScrollDown, 10, 8));
        assert_eq!(app.state.context_scroll, 3);
        draw(&mut app, 100, 8);
        assert_eq!(app.state.context_scroll, 3);
        draw(&mut app, 100, 30);
        assert_eq!(app.state.context_scroll, 0);
        assert_eq!(app.state.selected, Some(RowId::Pane(1)));
    }
}

mod event_loop {
    use super::*;

    struct Scripted(VecDeque<Option<Event>>);

    impl Events for Scripted {
        fn next(&mut self, _: Duration) -> Result<Option<Event>> {
            Ok(self.0.pop_front().expect("the script ends with q"))
        }
    }

    #[test]
    fn draws_the_updates_and_q_quits() {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("test terminal");
        let mut app = App::new(None, INTERVAL, Instant::now(), Box::new(FakeKiller::default()));
        let (send, updates) = mpsc::channel();
        send.send(Update::Reading(Ok(reading(first_sample(Instant::now()))))).expect("send the update");
        let mut events = Scripted(VecDeque::from([None, Some(key(KeyCode::Char('q')))]));
        run_loop(&mut terminal, &mut app, &mut events, &updates).expect("the loop ends cleanly");
        find(terminal.backend().buffer(), "%1 claude");
        assert!(events.0.is_empty());
    }

    #[test]
    fn ctrl_c_quits_too() {
        let mut app = App::new(None, INTERVAL, Instant::now(), Box::new(FakeKiller::default()));
        let event = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(app.handle(&event), Flow::Quit);
    }
}
