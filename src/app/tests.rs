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
pub(super) use crate::test_util::find;
use crate::test_util::fixture;
use crate::ui::{Sort, ViewTab};
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
    unfold_all(&mut app);
    app
}

pub(super) fn unfold_all(app: &mut App) {
    while let Some(line) = app.lines.iter().find(|line| line.row.folded == Some(true)) {
        app.state.fold(line.id, false);
        app.relayout();
    }
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

    fn ticked_once(sampler: &mut Sampler<FakeRunner, FakeSource>) -> App {
        let mut app = App::new(None, INTERVAL, Instant::now(), Box::new(FakeKiller::default()));
        app.update(Update::Reading(sampler.tick()), Instant::now());
        unfold_all(&mut app);
        app
    }

    #[test]
    fn a_tick_with_fakes_gives_the_tree_in_cornercase_order() {
        let mut sampler = fake_sampler();
        let app = ticked_once(&mut sampler);
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
        let mut app = ticked_once(&mut sampler);
        assert_eq!(app.view().header.cpu, None);
        app.update(Update::Reading(sampler.tick()), Instant::now());
        let claude = index_of(&app, RowId::Process { pane: 1, pid: 101 });
        assert_eq!(app.view().rows[claude].cpu, Some(100.0));
    }

    #[test]
    fn each_reading_adds_to_the_history_as_a_share_of_the_machine() {
        let mut sampler = fake_sampler();
        let mut app = ticked_once(&mut sampler);
        app.machine = super::super::Machine { cpus: 4, memory: Some(8_000 * MB) };
        app.update(Update::Reading(sampler.tick()), Instant::now());
        let graphs = app.view().graphs;
        assert_eq!(graphs.cpu, [25.0]);
        assert_eq!(graphs.memory, [495 * MB, 505 * MB]);
        assert_eq!((graphs.points, graphs.row_points, graphs.machine_memory), (1800, 30, Some(8_000 * MB)));
        let claude = index_of(&app, RowId::Process { pane: 1, pid: 101 });
        app.state.page = claude + 1;
        assert_eq!(app.view().rows[claude].cpu_history, [None, Some(100.0)]);
    }

    #[test]
    fn only_the_rows_on_screen_carry_their_history() {
        let mut sampler = fake_sampler();
        let mut app = ticked_once(&mut sampler);
        app.update(Update::Reading(sampler.tick()), Instant::now());
        let claude = index_of(&app, RowId::Process { pane: 1, pid: 101 });
        app.state.page = claude;
        assert_eq!(app.view().rows[claude].cpu_history, Vec::<Option<f64>>::new());
        app.state.scroll = 1;
        assert_eq!(app.view().rows[claude].cpu_history.len(), 2);
    }

    #[test]
    fn a_sort_change_adds_nothing_to_the_history() {
        let mut app = app_with(first_sample(Instant::now()), None);
        app.handle(&key(KeyCode::Char('s')), Instant::now());
        assert_eq!(app.view().graphs.memory.len(), 1);
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
        assert!(!app.check_stale(start + Duration::from_secs(6)));
        assert_eq!(app.view().header.error, None);
        assert!(app.check_stale(start + Duration::from_secs(7)));
        assert_eq!(app.view().header.error.as_deref(), Some("cornercase has not answered for 7s"));
    }

    #[test]
    fn the_stale_message_asks_for_a_draw_only_when_its_seconds_change() {
        let start = Instant::now();
        let mut app = App::new(None, INTERVAL, start, Box::new(FakeKiller::default()));
        assert!(app.check_stale(start + Duration::from_secs(7)));
        assert!(!app.check_stale(start + Duration::from_millis(7500)));
        assert!(app.check_stale(start + Duration::from_secs(8)));
    }

    #[test]
    fn a_config_warning_shows_until_the_first_input() {
        let mut app = app_with(first_sample(Instant::now()), None);
        app.notice = Some("bad config".to_owned());
        assert_eq!(app.view().header.error.as_deref(), Some("bad config"));
        app.handle(&key(KeyCode::Down), Instant::now());
        assert_eq!(app.view().header.error, None);
    }
}

mod refreshing {
    use super::*;

    #[test]
    fn selection_and_folds_survive_a_refresh_with_other_processes() {
        let now = Instant::now();
        let mut app = app_with(first_sample(now), None);
        app.state.fold(RowId::Pane(1), true);
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

mod starting_folded {
    use super::*;

    fn folded_app(sample: Sample) -> App {
        let mut app = App::new(None, INTERVAL, Instant::now(), Box::new(FakeKiller::default()));
        app.update(Update::Reading(Ok(reading(sample))), Instant::now());
        app
    }

    fn unfold(app: &mut App, ids: &[RowId]) {
        for id in ids {
            app.state.fold(*id, false);
        }
        app.relayout();
    }

    fn folded(app: &App, id: RowId) -> Option<bool> {
        app.view().rows[index_of(app, id)].folded
    }

    const TO_CARGO: [RowId; 5] =
        [RowId::Project(1), RowId::Workspace(1), RowId::Tab(1), RowId::Pane(2), RowId::Process { pane: 2, pid: 200 }];

    #[test]
    fn the_first_draw_shows_only_the_projects() {
        let mut app = folded_app(first_sample(Instant::now()));
        assert_eq!(labels(&app), ["notes", "shop"]);
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("test terminal");
        terminal.draw(|frame| app.draw(frame)).expect("draw");
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn an_unfolded_project_shows_its_workspaces_folded_and_a_reading_keeps_it_open() {
        let now = Instant::now();
        let mut app = folded_app(first_sample(now));
        unfold(&mut app, &[RowId::Project(1)]);
        assert_eq!(labels(&app), ["notes", "main", "feature-x (worktree)", "shop"]);
        assert_eq!(folded(&app, RowId::Workspace(1)), Some(true));
        app.update(Update::Reading(Ok(reading(second_sample(now)))), now);
        assert_eq!(labels(&app), ["notes", "main", "feature-x (worktree)", "shop"]);
    }

    #[test]
    fn a_new_process_with_children_under_an_open_pane_arrives_folded() {
        let now = Instant::now();
        let mut app = folded_app(first_sample(now));
        unfold(&mut app, &TO_CARGO);
        let mut next = second_sample(now);
        next.processes.push(process(203, 202, 2, 10, 0));
        app.update(Update::Reading(Ok(reading(next))), now);
        assert_eq!(folded(&app, RowId::Process { pane: 2, pid: 202 }), Some(true));
        assert!(!labels(&app).contains(&"proc203".to_owned()));
    }

    #[test]
    fn a_folded_process_on_the_hot_path_stays_hot() {
        let mut app = folded_app(first_sample(Instant::now()));
        unfold(&mut app, &[RowId::Project(1), RowId::Workspace(1), RowId::Tab(1), RowId::Pane(1)]);
        let shell = index_of(&app, RowId::Process { pane: 1, pid: 100 });
        assert_eq!(app.view().rows[shell].folded, Some(true));
        assert!(app.view().rows[shell].hot);
    }

    #[test]
    fn folding_above_the_selection_selects_the_folded_ancestor() {
        let mut app = folded_app(first_sample(Instant::now()));
        unfold(&mut app, &TO_CARGO);
        let cargo = index_of(&app, RowId::Process { pane: 2, pid: 201 });
        app.state.select(&app.lines, cargo);
        app.state.fold(RowId::Workspace(1), true);
        app.relayout();
        assert_eq!(app.state.selected, Some(RowId::Workspace(1)));
    }

    #[test]
    fn a_double_click_on_a_folded_project_opens_its_claude() {
        let mut app = folded_app(first_sample(Instant::now()));
        let (x, y) = find(&draw(&mut app, 100, 30), "▸ notes");
        let start = Instant::now();
        app.handle(&click(x + 4, y), start);
        draw(&mut app, 100, 30);
        app.handle(&click(x + 4, y), start + Duration::from_millis(300));
        assert_eq!(app.state.tab, ViewTab::Context);
        assert_eq!(app.state.agent, Some(1));
    }

    #[test]
    fn entering_context_from_a_folded_project_marks_its_claude() {
        let mut app = folded_app(first_sample(Instant::now()));
        let notes = index_of(&app, RowId::Project(1));
        app.state.select(&app.lines, notes);
        app.handle(&key(KeyCode::Tab), Instant::now());
        assert_eq!(app.state.agent, Some(1));
    }
}

mod mouse_and_keys {
    use super::*;

    #[test]
    fn clicking_the_cpu_header_sorts_by_cpu() {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (x, y) = find(&draw(&mut app, 100, 30), "CPU");
        app.handle(&click(x, y), Instant::now());
        assert_eq!(app.state.sort, Some(usage::Sort::Cpu));
        assert_eq!(app.view().sort, Sort::Cpu);
    }

    #[test]
    fn s_cycles_memory_cpu_and_cornercase_order() {
        let mut app = app_with(first_sample(Instant::now()), Some(usage::Sort::Memory));
        assert_eq!(labels(&app)[0], "notes");
        app.handle(&key(KeyCode::Char('s')), Instant::now());
        assert_eq!(app.view().sort, Sort::Cpu);
        app.handle(&key(KeyCode::Char('s')), Instant::now());
        assert_eq!(app.view().sort, Sort::Name);
        app.handle(&key(KeyCode::Char('s')), Instant::now());
        assert_eq!(app.view().sort, Sort::Memory);
    }

    #[test]
    fn clicking_the_arrow_folds_and_unfolds() {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (x, y) = find(&draw(&mut app, 100, 30), "▾ %1 claude");
        app.handle(&click(x, y), Instant::now());
        assert!(!labels(&app).contains(&"proc100".to_owned()));
        let (x, y) = find(&draw(&mut app, 100, 30), "▸ %1 claude");
        app.handle(&click(x, y), Instant::now());
        assert!(labels(&app).contains(&"proc100".to_owned()));
    }

    #[test]
    fn clicking_a_row_selects_it_and_a_tab_switches_view() {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (_, y) = find(&draw(&mut app, 100, 30), "%2 cargo");
        app.handle(&click(40, y), Instant::now());
        assert_eq!(app.state.selected, Some(RowId::Pane(2)));
        let (x, y) = find(&draw(&mut app, 100, 30), "Context ");
        app.handle(&click(x, y), Instant::now());
        assert_eq!(app.view().tab, ViewTab::Context);
    }

    #[test]
    fn the_wheel_scrolls() {
        let mut app = app_with(first_sample(Instant::now()), None);
        draw(&mut app, 100, 8);
        app.handle(&mouse(MouseEventKind::ScrollDown, 10, 4), Instant::now());
        assert_eq!(app.view().scroll, 3);
        app.handle(&mouse(MouseEventKind::ScrollUp, 10, 4), Instant::now());
        assert_eq!(app.view().scroll, 0);
    }

    #[test]
    fn the_context_tab_leaves_the_process_list_alone() {
        let mut app = app_with(first_sample(Instant::now()), Some(usage::Sort::Memory));
        let (_, y) = find(&draw(&mut app, 100, 30), "%2 cargo");
        app.handle(&click(40, y), Instant::now());
        let unfolded = app.state.unfolded.clone();
        app.handle(&key(KeyCode::Tab), Instant::now());
        draw(&mut app, 100, 8);
        for event in [key(KeyCode::Down), key(KeyCode::Enter), key(KeyCode::Char('s'))] {
            app.handle(&event, Instant::now());
        }
        app.handle(&mouse(MouseEventKind::ScrollDown, 10, 4), Instant::now());
        app.handle(&click(40, y), Instant::now());
        assert_eq!(app.state.selected, Some(RowId::Pane(2)));
        assert_eq!(app.state.sort, Some(usage::Sort::Memory));
        assert_eq!(app.view().scroll, 0);
        assert_eq!(app.state.unfolded, unfolded);
        let (x, y) = find(&draw(&mut app, 100, 8), "Processes ");
        app.handle(&click(x, y), Instant::now());
        assert_eq!(app.view().tab, ViewTab::Processes);
    }
}

mod context_view {
    use super::*;
    use crate::context::breakdown::{Breakdown, Estimate, Part, PartKind};
    use crate::ui::ContextDetail;

    const SECOND: &str = "notes › feature-x › %7";

    fn with_context(sample: Sample, context: PaneContext) -> App {
        let mut app = App::new(None, INTERVAL, Instant::now(), Box::new(FakeKiller::default()));
        app.update(Update::Reading(Ok(Reading { context: Some(context), ..reading(sample) })), Instant::now());
        unfold_all(&mut app);
        app
    }

    fn select(app: &mut App, id: RowId) {
        let index = index_of(app, id);
        app.state.select(&app.lines, index);
        app.handle(&Event::FocusGained, Instant::now());
    }

    fn enter(app: &mut App) {
        app.handle(&key(KeyCode::Tab), Instant::now());
    }

    #[test]
    fn the_processes_selection_asks_the_sampler_for_its_agent_once() {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (panes, asked) = mpsc::channel();
        app.panes = Some(panes);
        select(&mut app, RowId::Pane(7));
        select(&mut app, RowId::Process { pane: 7, pid: 300 });
        select(&mut app, RowId::Pane(2));
        select(&mut app, RowId::Project(2));
        select(&mut app, RowId::Tab(3));
        assert_eq!(asked.try_iter().collect::<Vec<_>>(), [Some(7), None, Some(7)]);
    }

    #[test]
    fn processes_without_a_claude_selected_reads_no_session_until_context_opens() {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (panes, asked) = mpsc::channel();
        app.panes = Some(panes);
        draw(&mut app, 100, 30);
        assert_eq!(asked.try_iter().collect::<Vec<_>>(), []);
        enter(&mut app);
        assert_eq!(asked.try_iter().collect::<Vec<_>>(), [Some(1)]);
    }

    #[test]
    fn entering_with_nothing_selected_marks_the_agent_with_most_context() {
        let mut app = app_with(first_sample(Instant::now()), None);
        enter(&mut app);
        assert_eq!(app.state.agent, Some(1));
        let context = app.view().context;
        assert_eq!((context.marked, context.agents.len()), (Some(0), 2));
        assert_eq!(context.detail, ContextDetail::Reading { pane: "notes › main › %1".to_owned() });
    }

    #[test]
    fn entering_with_a_tab_selected_marks_its_claude() {
        let mut app = app_with(first_sample(Instant::now()), None);
        select(&mut app, RowId::Tab(3));
        enter(&mut app);
        assert_eq!(app.state.agent, Some(7));
        assert_eq!(app.view().context.marked, Some(1));
    }

    #[test]
    fn entering_from_a_row_without_claude_marks_the_most_context() {
        let mut app = app_with(first_sample(Instant::now()), None);
        select(&mut app, RowId::Pane(2));
        enter(&mut app);
        assert_eq!(app.state.agent, Some(1));
    }

    #[test]
    fn a_click_on_an_agent_changes_the_breakdown() {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (panes, asked) = mpsc::channel();
        app.panes = Some(panes);
        enter(&mut app);
        let (x, y) = find(&draw(&mut app, 100, 30), "%7");
        app.handle(&click(x, y), Instant::now());
        assert_eq!(app.state.agent, Some(7));
        assert_eq!(app.view().context.detail, ContextDetail::Reading { pane: SECOND.to_owned() });
        assert_eq!(asked.try_iter().last(), Some(Some(7)));
    }

    #[test]
    fn arrows_move_the_mark_down_the_list() {
        let mut app = app_with(first_sample(Instant::now()), None);
        enter(&mut app);
        app.handle(&key(KeyCode::Down), Instant::now());
        assert_eq!(app.state.agent, Some(7));
        assert_eq!(app.state.selected, None);
    }

    #[test]
    fn a_marked_agent_that_goes_away_hands_the_mark_to_the_first() {
        let mut app = app_with(first_sample(Instant::now()), None);
        enter(&mut app);
        app.state.agent = Some(99);
        app.update(Update::Reading(Ok(reading(first_sample(Instant::now())))), Instant::now());
        assert_eq!(app.state.agent, Some(1));
    }

    #[test]
    fn a_context_answer_keeps_the_error_and_the_last_answer_time() {
        let start = Instant::now();
        let mut app = with_context(first_sample(start), PaneContext { pane: 7, found: Found::NoClaude });
        app.update(Update::Reading(Err("cornercase: server not running".to_owned())), start);
        select(&mut app, RowId::Pane(7));
        app.update(Update::Context(Some(PaneContext { pane: 7, found: Found::NoSession(300) })), start + INTERVAL * 9);
        assert_eq!(app.answered, start);
        assert_eq!(app.view().header.error.as_deref(), Some("cornercase: server not running"));
        enter(&mut app);
        assert_eq!(app.view().context.detail, ContextDetail::NoSession { pane: SECOND.to_owned(), pid: 300 });
    }

    #[test]
    fn a_selection_moved_outside_the_input_is_asked_on_the_next_draw() {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (panes, asked) = mpsc::channel();
        app.panes = Some(panes);
        let index = index_of(&app, RowId::Pane(7));
        app.state.select(&app.lines, index);
        draw(&mut app, 100, 30);
        assert_eq!(asked.try_iter().collect::<Vec<_>>(), [Some(7)]);
    }

    #[test]
    fn a_context_of_another_agent_shows_as_being_read_and_processes_keeps_the_selection() {
        let mut app = with_context(first_sample(Instant::now()), PaneContext { pane: 7, found: Found::NoClaude });
        select(&mut app, RowId::Pane(7));
        enter(&mut app);
        assert_eq!(app.view().context.detail, ContextDetail::NoClaude { pane: SECOND.to_owned() });
        app.handle(&key(KeyCode::Up), Instant::now());
        assert_eq!(app.view().context.detail, ContextDetail::Reading { pane: "notes › main › %1".to_owned() });
        app.handle(&key(KeyCode::Tab), Instant::now());
        assert_eq!(app.view().tab, ViewTab::Processes);
        assert_eq!(app.state.selected, Some(RowId::Pane(7)));
    }

    #[test]
    fn the_wheel_scrolls_the_breakdown_up_to_its_end() {
        let part = |n: u64| Part { kind: PartKind::File(format!("/f{n}")), tokens: Estimate(n) };
        let breakdown = Breakdown { total: Some(100), fixed: Vec::new(), dynamic: (1..=10).map(part).collect() };
        let mut app =
            with_context(first_sample(Instant::now()), PaneContext { pane: 1, found: Found::Session(101, breakdown) });
        enter(&mut app);
        app.handle(&mouse(MouseEventKind::ScrollDown, 50, 6), Instant::now());
        assert_eq!(app.state.context_scroll, 3);
        draw(&mut app, 100, 8);
        assert_eq!(app.state.context_scroll, 3);
        draw(&mut app, 100, 30);
        assert_eq!(app.state.context_scroll, 0);
    }
}

mod double_click {
    use super::*;

    const SOON: Duration = Duration::from_millis(300);

    fn clicked_twice(on: &str, gap: Duration) -> App {
        let mut app = app_with(first_sample(Instant::now()), None);
        let (x, y) = find(&draw(&mut app, 100, 30), on);
        let start = Instant::now();
        app.handle(&click(x + 4, y), start);
        draw(&mut app, 100, 30);
        app.handle(&click(x + 4, y), start + gap);
        app
    }

    #[test]
    fn on_a_pane_with_claude_opens_its_context() {
        let app = clicked_twice("%7 claude", SOON);
        assert_eq!(app.state.tab, ViewTab::Context);
        assert_eq!(app.state.agent, Some(7));
    }

    #[test]
    fn too_slow_is_two_clicks() {
        let app = clicked_twice("%7 claude", Duration::from_secs(1));
        assert_eq!(app.state.tab, ViewTab::Processes);
        assert_eq!(app.state.selected, Some(RowId::Pane(7)));
    }

    #[test]
    fn on_a_pane_without_claude_only_selects_it() {
        let app = clicked_twice("%2 cargo", SOON);
        assert_eq!(app.state.tab, ViewTab::Processes);
        assert_eq!(app.state.selected, Some(RowId::Pane(2)));
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
        find(terminal.backend().buffer(), "▸ notes");
        assert!(events.0.is_empty());
    }

    #[test]
    fn draws_again_only_after_an_update_or_an_input() {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("test terminal");
        let mut app = App::new(None, INTERVAL, Instant::now(), Box::new(FakeKiller::default()));
        let (_send, updates) = mpsc::channel();
        let script = [None, None, Some(key(KeyCode::Down)), None, Some(key(KeyCode::Char('q')))];
        run_loop(&mut terminal, &mut app, &mut Scripted(VecDeque::from(script)), &updates).expect("the loop ends");
        assert_eq!(terminal.get_frame().count(), 2);
    }

    #[test]
    fn a_mouse_moving_without_a_button_does_not_draw_again() {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("test terminal");
        let mut app = App::new(None, INTERVAL, Instant::now(), Box::new(FakeKiller::default()));
        let (_send, updates) = mpsc::channel();
        let moved = Some(mouse(MouseEventKind::Moved, 10, 4));
        let script = [None, moved.clone(), moved, None, Some(key(KeyCode::Char('q')))];
        run_loop(&mut terminal, &mut app, &mut Scripted(VecDeque::from(script)), &updates).expect("the loop ends");
        assert_eq!(terminal.get_frame().count(), 1);
    }

    #[test]
    fn esc_quits_without_a_dialog() {
        let mut app = App::new(None, INTERVAL, Instant::now(), Box::new(FakeKiller::default()));
        assert_eq!(app.handle(&key(KeyCode::Esc), Instant::now()), Flow::Quit);
    }

    #[test]
    fn ctrl_c_quits_too() {
        let mut app = App::new(None, INTERVAL, Instant::now(), Box::new(FakeKiller::default()));
        let event = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(app.handle(&event, Instant::now()), Flow::Quit);
    }
}
