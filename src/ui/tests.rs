use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use rstest::rstest;

use super::{ContextView, Header, Hit, Hits, Row, RowKind, Sort, View, ViewTab, render};

const GIB: u64 = 1024 * 1024 * 1024;
const MIB: u64 = 1024 * 1024;

fn row(depth: u16, kind: RowKind, label: &str, memory: u64) -> Row {
    Row { depth, kind, label: label.to_owned(), memory: Some(memory), cpu: Some(1.5), ..Row::default() }
}

fn hot(row: Row) -> Row {
    Row { hot: true, ..row }
}

fn open(row: Row) -> Row {
    Row { folded: Some(false), ..row }
}

fn agent_pane() -> Row {
    Row {
        agent: Some("claude".to_owned()),
        status: Some("working".to_owned()),
        context: Some(42.0),
        cpu: Some(312.2),
        ..hot(open(row(3, RowKind::Pane, "%3 claude", 5 * GIB)))
    }
}

fn sample_rows() -> Vec<Row> {
    vec![
        open(row(0, RowKind::Project, "cctop", 6 * GIB)),
        Row { folded: Some(true), ..row(1, RowKind::Workspace, "main", 300 * MIB) },
        open(row(1, RowKind::Workspace, "task/009-ui-processes-view (worktree)", 5 * GIB)),
        open(row(2, RowKind::Tab, "build", 5 * GIB)),
        agent_pane(),
        hot(open(row(4, RowKind::Process, "zsh", 4 * MIB))),
        hot(open(row(5, RowKind::Process, "claude", 700 * MIB))),
        hot(row(6, RowKind::Process, "cargo test --locked", 4 * GIB)),
        Row { estimated: true, cpu: None, ..row(6, RowKind::Process, "rust-analyzer", 200 * MIB) },
        Row { folded: Some(true), ..row(0, RowKind::Project, "notes", 120 * MIB) },
    ]
}

fn sample_view() -> View {
    View {
        tab: ViewTab::Processes,
        rows: sample_rows(),
        scroll: 0,
        selected: Some(8),
        sort: Sort::Memory,
        header: Header { memory: Some(6 * GIB + 120 * MIB), cpu: Some(314.0), error: None },
        context: ContextView::default(),
    }
}

fn draw(view: &View, width: u16, height: u16) -> (Buffer, Hits) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    let mut hits = Hits::default();
    terminal.draw(|frame| hits = render(frame, view)).expect("draw");
    (terminal.backend().buffer().clone(), hits)
}

pub(super) fn line(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect()
}

mod layout {
    use super::*;

    #[test]
    fn draws_a_tree_with_an_agent_pane_at_100_by_20() {
        let mut terminal = Terminal::new(TestBackend::new(100, 20)).expect("test terminal");
        terminal.draw(|frame| drop(render(frame, &sample_view()))).expect("draw");
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn draws_only_name_memory_and_cpu_at_40_columns() {
        let mut terminal = Terminal::new(TestBackend::new(40, 8)).expect("test terminal");
        terminal.draw(|frame| drop(render(frame, &sample_view()))).expect("draw");
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn shows_the_error_instead_of_the_totals() {
        let view = View {
            header: Header { error: Some("cornercase is not running".to_owned()), ..Header::default() },
            ..sample_view()
        };
        let (buffer, _) = draw(&view, 100, 4);
        assert!(line(&buffer, 0).trim_end().ends_with("cornercase is not running"));
        assert_eq!(buffer[(98, 0)].fg, Color::Red);
    }

    #[test]
    fn the_context_tab_draws_no_process_rows() {
        let view = View { tab: ViewTab::Context, ..sample_view() };
        let (buffer, hits) = draw(&view, 100, 20);
        assert!((2..20).all(|y| line(&buffer, y).trim().is_empty()));
        assert_eq!(hits.at(5, 2), None);
    }

    #[test]
    fn scrolling_starts_at_the_first_visible_row() {
        let view = View { scroll: 7, ..sample_view() };
        let (buffer, _) = draw(&view, 100, 20);
        assert!(line(&buffer, 2).contains("cargo test --locked"));
    }

    #[test]
    fn missing_values_show_a_dash_and_estimates_an_approximation() {
        let (buffer, _) = draw(&sample_view(), 100, 20);
        let analyzer = line(&buffer, 10);
        assert!(analyzer.contains("≈200 MB"), "{analyzer}");
        assert!(analyzer.contains('—'), "{analyzer}");
    }

    #[test]
    fn only_panes_fill_agent_status_and_context() {
        let (buffer, _) = draw(&sample_view(), 100, 20);
        assert_eq!(line(&buffer, 2).chars().skip(72).collect::<String>().trim(), "");
        assert!(line(&buffer, 6).ends_with("claude   working        42%"));
    }
}

mod styles {
    use super::*;

    #[test]
    fn the_selected_row_is_reversed_across_the_whole_width() {
        let (buffer, _) = draw(&sample_view(), 100, 20);
        assert!([0, 50, 99].iter().all(|&x| buffer[(x, 10)].modifier.contains(Modifier::REVERSED)));
        assert!(!buffer[(20, 9)].modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn the_hot_path_is_highlighted_and_other_processes_are_not() {
        let (buffer, _) = draw(&sample_view(), 100, 20);
        assert!((6..=9).all(|y| buffer[(20, y)].fg == Color::Yellow));
        assert_ne!(buffer[(20, 10)].fg, Color::Yellow);
        assert_ne!(buffer[(20, 3)].fg, Color::Yellow);
    }

    #[test]
    fn the_active_tab_is_reversed() {
        let (buffer, _) = draw(&View { tab: ViewTab::Context, ..sample_view() }, 100, 20);
        assert!(buffer[(13, 0)].modifier.contains(Modifier::REVERSED));
        assert!(!buffer[(2, 0)].modifier.contains(Modifier::REVERSED));
    }
}

mod hits {
    use super::*;

    #[rstest]
    #[case::processes_tab(2, 0, Some(Hit::Tab(ViewTab::Processes)))]
    #[case::context_tab(13, 0, Some(Hit::Tab(ViewTab::Context)))]
    #[case::summary_is_not_clickable(90, 0, None)]
    #[case::name_column(10, 1, Some(Hit::Sort(Sort::Name)))]
    #[case::memory_column(60, 1, Some(Hit::Sort(Sort::Memory)))]
    #[case::cpu_column(68, 1, Some(Hit::Sort(Sort::Cpu)))]
    #[case::agent_column(75, 1, Some(Hit::Sort(Sort::Agent)))]
    #[case::status_column(85, 1, Some(Hit::Sort(Sort::Status)))]
    #[case::context_column(95, 1, Some(Hit::Sort(Sort::Context)))]
    #[case::project_arrow(0, 2, Some(Hit::Fold(0)))]
    #[case::project_label(5, 2, Some(Hit::Row(0)))]
    #[case::pane_arrow(7, 6, Some(Hit::Fold(4)))]
    #[case::pane_before_its_arrow(5, 6, Some(Hit::Row(4)))]
    #[case::leaf_process_has_no_arrow(12, 9, Some(Hit::Row(7)))]
    #[case::memory_cell_selects_the_row(60, 6, Some(Hit::Row(4)))]
    #[case::below_the_last_row(5, 13, None)]
    fn point_at_what_is_under_the_mouse(#[case] column: u16, #[case] row: u16, #[case] expected: Option<Hit>) {
        let (_, hits) = draw(&sample_view(), 100, 20);
        assert_eq!(hits.at(column, row), expected);
    }

    #[test]
    fn rows_keep_their_index_when_scrolled() {
        let (_, hits) = draw(&View { scroll: 4, ..sample_view() }, 100, 20);
        assert_eq!(hits.at(7, 2), Some(Hit::Fold(4)));
        assert_eq!(hits.at(30, 3), Some(Hit::Row(5)));
    }
}

mod narrow {
    use super::*;

    #[test]
    fn long_names_are_cut_at_the_end_without_moving_columns() {
        let long = "x".repeat(200);
        let view = View { rows: vec![row(0, RowKind::Process, &long, 512 * MIB)], selected: None, ..sample_view() };
        let (buffer, _) = draw(&view, 100, 4);
        let text = line(&buffer, 2);
        assert_eq!(text.chars().nth(54), Some('…'));
        assert_eq!(text.chars().nth(55), Some(' '));
        assert!(text.contains(" 512 MB "), "{text}");
    }

    #[test]
    fn deep_rows_draw_without_panicking() {
        let deep = Row { folded: Some(false), ..row(u16::MAX, RowKind::Process, "deep", MIB) };
        let (_, hits) = draw(&View { rows: vec![deep], selected: None, ..sample_view() }, 100, 4);
        assert_eq!(hits.at(5, 2), Some(Hit::Row(0)));
    }

    #[rstest]
    #[case::both_totals(" Processes  Context  mem 6.1 GB  cpu 314% ", 42)]
    #[case::drops_the_cpu_whole(" Processes  Context          mem 6.1 GB", 40)]
    #[case::drops_both_totals(" Processes  Context", 28)]
    fn totals_that_do_not_fit_are_left_out_whole(#[case] expected: &str, #[case] width: u16) {
        let (buffer, _) = draw(&sample_view(), width, 2);
        assert_eq!(line(&buffer, 0).trim_end(), expected.trim_end());
    }

    #[rstest]
    #[case::empty(0, 0)]
    #[case::one_cell(1, 1)]
    #[case::only_the_tabs(40, 1)]
    #[case::tiny(10, 3)]
    #[case::minimum_width(40, 20)]
    fn small_terminals_draw_without_panicking(#[case] width: u16, #[case] height: u16) {
        let view = View { scroll: 100, ..sample_view() };
        draw(&view, width, height);
    }
}
