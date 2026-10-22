use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use std::time::Instant;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use rstest::rstest;

use super::fake::Counting;
use super::{Contexts, Found, PaneContext, Source, is_claude, pane_node, percent, read, view};
use crate::context::Sessions;
use crate::context::breakdown::{Accumulator, Breakdown, Estimate, Part, PartKind};
use crate::context::fake::start;
use crate::context::record::parse;
use crate::cornercase::Snapshot;
use crate::procs::{Pid, Process, Sample};
use crate::test_util::{TempDir, fixture};
use crate::ui::{self, AgentRow, ContextDetail, ContextPercent, ContextView, Header, View, ViewTab};
use crate::usage::{self, Tree};

const STATUS: &str = fixture!("cornercase/status-0.12.173.json");
const SESSION: &str = fixture!("claude/2.1.306/session.jsonl");
const BOOT: &str = "4242-1700000000000000000";
const SESSION_ID: &str = "00000000-0000-4000-8000-000000000001";
const LABEL: &str = "notes › main › %1";

fn process(pid: Pid, parent: Pid, pane: Option<u64>, name: &str) -> Process {
    Process {
        pid,
        ppid: Some(parent),
        name: name.to_owned(),
        args: vec![name.to_owned()],
        memory: Some(1),
        pane,
        server: pane.map(|_| BOOT.to_owned()),
        ..Process::default()
    }
}

fn claude_in(config: Option<&Path>, pid: Pid, parent: Pid) -> Process {
    Process { claude_config_dir: config.map(Path::to_path_buf), ..process(pid, parent, None, "claude") }
}

fn sample(config: Option<&Path>) -> Sample {
    let processes = vec![
        process(100, 1, Some(1), "zsh"),
        claude_in(config, 101, 100),
        process(200, 1, Some(2), "cargo"),
        process(300, 1, Some(7), "zsh"),
        claude_in(config, 301, 300),
        process(500, 1, Some(5), "zsh"),
    ];
    Sample { taken: Instant::now(), processes }
}

fn snapshot() -> Snapshot {
    Snapshot::parse(STATUS).expect("parse the status fixture")
}

fn start_session(config: &Path, pid: Pid) {
    start(config, pid, SESSION_ID, SESSION);
}

fn expected() -> Breakdown {
    let mut accumulator = Accumulator::default();
    SESSION.lines().map(parse).for_each(|record| accumulator.push(record));
    accumulator.breakdown()
}

fn read_pane(contexts: &mut impl Contexts, sample: &Sample, pane: u64, home: Option<&OsString>) -> PaneContext {
    let snapshot = snapshot();
    let source = Source { snapshot: &snapshot, sample, home };
    read(contexts, &source, Some(pane)).expect("a pane was asked for")
}

fn tree(sample: &Sample) -> Tree {
    usage::build(&snapshot(), sample, &HashMap::new(), None)
}

mod finding_claude {
    use super::*;

    fn named(name: &str, args: &[&str]) -> Process {
        Process {
            name: name.to_owned(),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            ..Process::default()
        }
    }

    #[rstest]
    #[case::by_name(named("claude", &["claude"]), true)]
    #[case::by_its_path(named("2.1.306", &["/home/user/.local/bin/claude", "--resume"]), true)]
    #[case::run_by_node(named("node", &["node", "/usr/bin/claude"]), true)]
    #[case::an_argument_of_another_program(named("vim", &["vim", "claude"]), false)]
    #[case::node_running_something_else(named("node", &["node", "server.js"]), false)]
    #[case::no_arguments(named("zsh", &[]), false)]
    fn by_name_or_argv(#[case] process: Process, #[case] expected: bool) {
        assert_eq!(is_claude(&process), expected);
    }

    #[test]
    fn takes_the_first_claude_of_the_pane_parents_first() {
        let config = TempDir::new();
        let mut processes = sample(Some(config.path())).processes;
        processes.push(claude_in(Some(config.path()), 102, 101));
        let mut counting = Counting::default();
        read_pane(&mut counting, &Sample { taken: Instant::now(), processes }, 1, None);
        assert_eq!(counting.asked, [vec![101]]);
    }
}

mod reading {
    use super::*;

    #[test]
    fn gives_the_breakdown_of_the_claude_in_the_pane() {
        let config = TempDir::new();
        start_session(config.path(), 101);
        let context = read_pane(&mut Sessions::default(), &sample(Some(config.path())), 1, None);
        assert_eq!(context, PaneContext { pane: 1, found: Found::Session(101, expected()) });
    }

    #[test]
    fn falls_back_to_the_claude_folder_in_home() {
        let home = TempDir::new();
        start_session(&home.path().join(".claude"), 101);
        let home_var = OsString::from(home.path());
        let context = read_pane(&mut Sessions::default(), &sample(None), 1, Some(&home_var));
        assert!(matches!(context.found, Found::Session(101, _)), "{context:?}");
    }

    #[test]
    fn a_pane_without_claude_reads_nothing() {
        let mut counting = Counting::default();
        let context = read_pane(&mut counting, &sample(None), 2, None);
        assert_eq!(context.found, Found::NoClaude);
        assert_eq!(counting.asked, [Vec::<Pid>::new()]);
    }

    #[test]
    fn a_claude_without_a_session_file_has_no_session() {
        let config = TempDir::new();
        let context = read_pane(&mut Sessions::default(), &sample(Some(config.path())), 1, None);
        assert_eq!(context.found, Found::NoSession(101));
    }

    #[test]
    fn only_the_selected_pane_is_read() {
        let config = TempDir::new();
        let mut counting = Counting::default();
        read_pane(&mut counting, &sample(Some(config.path())), 7, None);
        assert_eq!(counting.asked, [vec![301]]);
    }

    #[test]
    fn no_selected_pane_reads_nothing_and_drops_the_readers() {
        let mut counting = Counting::default();
        let snapshot = snapshot();
        let sample = sample(None);
        let source = Source { snapshot: &snapshot, sample: &sample, home: None };
        assert_eq!(read(&mut counting, &source, None), None);
        assert_eq!(counting.asked, [Vec::<Pid>::new()]);
    }
}

mod viewing {
    use super::*;

    fn session_view(found: Found) -> ContextDetail {
        session_view_showing(found, None)
    }

    fn session_view_showing(found: Found, shown: Option<u16>) -> ContextDetail {
        let sample = sample(None);
        let mut tree = tree(&sample);
        let workspaces = tree.projects.iter_mut().flat_map(|project| &mut project.workspaces);
        let panes = workspaces.flat_map(|workspace| &mut workspace.tabs).flat_map(|tab| &mut tab.panes);
        for pane in panes.filter(|pane| pane.id == 1 && shown.is_some()) {
            pane.context_percent = shown;
        }
        view(pane_node(&tree, 1).map(|pane| (pane, LABEL)), Some(&PaneContext { pane: 1, found }), 0)
    }

    fn part(kind: PartKind, tokens: u64) -> Part {
        Part { kind, tokens: Estimate(tokens) }
    }

    #[test]
    fn draws_the_fixture_session_with_estimates_and_a_real_total() {
        let view = View {
            tab: ViewTab::Context,
            header: Header { memory: Some(1 << 30), cpu: Some(12.0), error: None },
            context: ContextView {
                agents: vec![
                    AgentRow { label: LABEL.to_owned(), percent: Some(23.0) },
                    AgentRow { label: "notes › feature-x › %7".to_owned(), percent: Some(7.0) },
                ],
                marked: Some(0),
                detail: session_view_showing(Found::Session(101, expected()), Some(23)),
            },
            ..View::default()
        };
        let mut terminal = Terminal::new(TestBackend::new(90, 30)).expect("test terminal");
        terminal.draw(|frame| drop(ui::render(frame, &view))).expect("draw");
        insta::assert_snapshot!(terminal.backend());
    }

    #[rstest]
    #[case::no_claude(Found::NoClaude, ContextDetail::NoClaude { pane: LABEL.to_owned() })]
    #[case::no_session(Found::NoSession(101), ContextDetail::NoSession { pane: LABEL.to_owned(), pid: 101 })]
    fn empty_states_name_the_pane(#[case] found: Found, #[case] expected: ContextDetail) {
        assert_eq!(session_view(found), expected);
    }

    #[test]
    fn no_agent_says_so() {
        assert_eq!(view(None, None, 0), ContextDetail::NoAgent);
    }

    #[test]
    fn a_context_of_another_pane_is_still_being_read() {
        let sample = sample(None);
        let tree = tree(&sample);
        let other = PaneContext { pane: 7, found: Found::NoClaude };
        let pane = pane_node(&tree, 1).map(|pane| (pane, LABEL));
        assert_eq!(view(pane, Some(&other), 0), ContextDetail::Reading { pane: LABEL.to_owned() });
    }

    #[test]
    fn paths_inside_the_pane_folder_are_relative_and_commands_one_line() {
        let breakdown = Breakdown {
            total: None,
            fixed: vec![part(PartKind::Instructions("/home/user/src/notes/AGENTS.md".to_owned()), 10)],
            dynamic: vec![
                part(PartKind::File("/etc/hosts".to_owned()), 5),
                part(PartKind::Command("cargo test \\\n  --locked".to_owned()), 3),
            ],
        };
        let ContextDetail::Session(session) = session_view(Found::Session(101, breakdown)) else {
            panic!("a session view")
        };
        let labels: Vec<(&str, bool)> =
            session.fixed.iter().chain(&session.dynamic).map(|part| (part.label.as_str(), part.path)).collect();
        assert_eq!(labels, [("AGENTS.md", true), ("/etc/hosts", true), ("$ cargo test \\ --locked", false)]);
        assert_eq!(session.percent, None);
    }
}

mod window {
    use super::*;

    #[rstest]
    #[case::small_session_on_a_1m_window(60_000, 6, 1_000_000)]
    #[case::small_session_on_a_200k_window(60_000, 30, 200_000)]
    #[case::past_200k_is_always_1m(300_000, 100, 1_000_000)]
    fn is_the_one_closer_to_cornercase(#[case] total: u64, #[case] shown: u16, #[case] window: u64) {
        let percent = percent(total, shown).expect("a window");
        assert_eq!(percent.window, window);
        assert_eq!(percent.cornercase, f64::from(shown));
    }

    #[test]
    fn past_every_window_shows_no_percent() {
        assert_eq!(percent(2_000_000, 100), None::<ContextPercent>);
    }

    #[test]
    fn the_session_shows_both_percentages() {
        let sample = sample(None);
        let tree = tree(&sample);
        let breakdown = Breakdown { total: Some(84_000), fixed: Vec::new(), dynamic: Vec::new() };
        let found = PaneContext { pane: 1, found: Found::Session(101, breakdown) };
        let pane = pane_node(&tree, 1).map(|pane| (pane, LABEL));
        let ContextDetail::Session(session) = view(pane, Some(&found), 0) else { panic!("a session") };
        let percent = session.percent.expect("cornercase gives 42%");
        assert_eq!((percent.window, percent.used, percent.cornercase), (200_000, 42.0, 42.0));
    }
}
