use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use rstest::rstest;

use super::{Button, ContextView, Dialog, Graphs, Header, Hit, Hits, Row, RowKind, Sort, View, ViewTab, render};
use crate::test_util::find;
pub(super) use crate::test_util::screen_line as line;

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

fn wave(points: usize, peak: f64) -> Vec<f64> {
    (0..points).map(|n| peak * f64::from(u32::try_from(n % 10).expect("small")) / 9.0).collect()
}

fn agent_pane() -> Row {
    Row {
        agent: Some("claude".to_owned()),
        status: Some("working".to_owned()),
        context: Some(42.0),
        cpu: Some(312.2),
        cpu_history: wave(30, 312.0).into_iter().map(Some).collect(),
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
        Row {
            cpu_history: wave(12, 90.0).into_iter().map(Some).collect(),
            ..hot(row(6, RowKind::Process, "cargo test --locked", 4 * GIB))
        },
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
        dialog: None,
        graphs: Graphs {
            cpu: wave(1200, 40.0),
            memory: (0..1200).map(|n| 4 * GIB + n * MIB * 2).collect(),
            points: 1800,
            machine_memory: Some(32 * GIB),
            row_points: 30,
        },
        memory_threshold: 4 * GIB,
    }
}

fn draw(view: &View, width: u16, height: u16) -> (Buffer, Hits) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    let mut hits = Hits::default();
    terminal.draw(|frame| hits = render(frame, view)).expect("draw");
    (terminal.backend().buffer().clone(), hits)
}

fn snapshot(view: &View, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal.draw(|frame| drop(render(frame, view))).expect("draw");
    terminal
}

fn text(buffer: &Buffer) -> String {
    (0..buffer.area.height).map(|y| line(buffer, y)).collect::<Vec<_>>().join("\n")
}

fn is_braille(symbol: &str) -> bool {
    symbol.chars().any(|character| ('\u{2801}'..='\u{28ff}').contains(&character))
}

mod layout {
    use super::*;

    #[test]
    fn draws_the_graphs_bars_and_sparklines_at_120_by_40() {
        insta::assert_snapshot!(snapshot(&sample_view(), 120, 40).backend());
    }

    #[test]
    fn draws_the_graphs_bars_and_sparklines_at_80_by_24() {
        insta::assert_snapshot!(snapshot(&sample_view(), 80, 24).backend());
    }

    #[test]
    fn draws_only_name_memory_and_cpu_at_40_columns() {
        let terminal = snapshot(&sample_view(), 40, 12);
        let shown = text(terminal.backend().buffer());
        assert!(!shown.contains("CPU 60s") && !shown.contains('▁'), "{shown}");
        assert!(!shown.chars().any(|character| is_braille(&character.to_string())), "{shown}");
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn a_short_terminal_hides_the_graphs_before_the_table() {
        let (buffer, _) = draw(&sample_view(), 100, 15);
        assert!(line(&buffer, 0).starts_with("╭─ Processes "), "{}", line(&buffer, 0));
        assert!(line(&buffer, 0).contains("mem 6.1 GB  cpu 314%"));
    }

    #[test]
    fn the_graphs_show_the_machine_share_and_the_last_hour() {
        let (buffer, _) = draw(&sample_view(), 100, 20);
        let top = line(&buffer, 0);
        assert!(top.contains(" cpu ") && top.contains(" 40% "), "{top}");
        assert!(top.contains(" mem ") && top.contains(" 6.1 GB / 32.0 GB "), "{top}");
        assert!(line(&buffer, 2).contains(" -60m "));
        assert!(is_braille(buffer[(48, 1)].symbol()));
        assert!(!line(&buffer, 3).contains("mem 6.1 GB"));
    }

    #[test]
    fn without_the_machine_memory_the_graph_scales_to_its_peak() {
        let mut view = sample_view();
        view.graphs.machine_memory = None;
        let (buffer, _) = draw(&view, 100, 20);
        assert!(line(&buffer, 0).contains(" 6.1 GB ─╮"), "{}", line(&buffer, 0));
        assert_eq!(buffer[(98, 1)].symbol(), "⣿");
    }

    #[test]
    fn shows_the_error_on_the_border_of_the_table() {
        let view = View {
            header: Header { error: Some("cornercase is not running".to_owned()), ..Header::default() },
            ..sample_view()
        };
        let (buffer, _) = draw(&view, 100, 20);
        let (x, y) = find(&buffer, "cornercase is not running");
        assert_eq!((y, buffer[(x, y)].fg), (3, Color::Red));
    }

    #[test]
    fn the_keys_are_on_the_bottom_border() {
        let (buffer, _) = draw(&sample_view(), 100, 20);
        assert!(line(&buffer, 19).starts_with("╰─ ↑↓ move  ⏎ fold  Tab view  s sort  k kill  q/Esc close ─"));
        let (narrow, _) = draw(&sample_view(), 40, 12);
        assert!(line(&narrow, 11).starts_with("╰─ ↑↓ move  ⏎ fold  Tab view  s sort ─"), "{}", line(&narrow, 11));
    }

    #[test]
    fn the_context_tab_draws_no_process_rows() {
        let view = View { tab: ViewTab::Context, ..sample_view() };
        let (buffer, hits) = draw(&view, 100, 20);
        assert!(!text(&buffer).contains("cargo test"));
        assert!(!matches!(hits.at(50, 5), Some(Hit::Row(_))));
    }

    #[test]
    fn scrolling_starts_at_the_first_visible_row() {
        let view = View { scroll: 7, ..sample_view() };
        let (buffer, _) = draw(&view, 100, 20);
        assert!(line(&buffer, 5).contains("cargo test --locked"));
    }

    #[test]
    fn missing_values_show_a_dash_and_estimates_an_approximation() {
        let (buffer, _) = draw(&sample_view(), 100, 20);
        let (_, y) = find(&buffer, "rust-analyzer");
        let analyzer = line(&buffer, y);
        assert!(analyzer.contains("≈200 MB"), "{analyzer}");
        assert!(analyzer.contains('—'), "{analyzer}");
    }

    #[test]
    fn only_panes_fill_agent_status_and_context() {
        let (buffer, _) = draw(&sample_view(), 120, 20);
        let (x, _) = find(&buffer, "Agent");
        let (_, project) = find(&buffer, "cctop");
        assert_eq!(line(&buffer, project).chars().skip(usize::from(x)).collect::<String>().trim(), "│");
        let (_, pane) = find(&buffer, "%3 claude");
        assert!(line(&buffer, pane).ends_with("claude   working        42%│"), "{}", line(&buffer, pane));
    }
}

mod bars_and_sparklines {
    use super::*;

    #[test]
    fn the_memory_bar_is_the_share_of_the_total_colored_by_the_threshold() {
        let (buffer, _) = draw(&sample_view(), 120, 20);
        let (_, pane) = find(&buffer, "%3 claude");
        let (_, shell) = find(&buffer, "zsh");
        let (x, _) = find(&buffer, "████");
        assert_eq!(buffer[(x, pane)].fg, Color::Red);
        assert_eq!(buffer[(x + 8, pane)].symbol(), "█");
        assert_eq!(buffer[(x + 9, pane)].symbol(), "▊");
        assert_eq!(buffer[(x + 11, pane)].symbol(), "░");
        assert_eq!(buffer[(x, shell)].symbol(), "░");
        let (_, claude) = find(&buffer, "▾ claude ");
        assert_eq!(buffer[(x, claude)].fg, Color::Green);
    }

    #[test]
    fn the_sparkline_draws_the_last_minute_with_the_newest_on_the_right() {
        let (buffer, _) = draw(&sample_view(), 120, 20);
        let (x, header) = find(&buffer, "CPU 60s");
        let (_, cargo) = find(&buffer, "cargo test");
        let spark: String = (x..x + 12).map(|x| buffer[(x, cargo)].symbol()).collect();
        assert!(spark.starts_with("       "), "{spark}");
        assert_ne!(spark.trim(), "", "{spark}");
        assert!(header > 0);
    }
}

mod styles {
    use super::*;

    #[test]
    fn the_selected_row_is_reversed_across_the_table() {
        let (buffer, _) = draw(&sample_view(), 100, 20);
        let (_, y) = find(&buffer, "rust-analyzer");
        assert!([1, 50, 98].iter().all(|&x| buffer[(x, y)].modifier.contains(Modifier::REVERSED)));
        assert!(!buffer[(0, y)].modifier.contains(Modifier::REVERSED));
        assert!(!buffer[(20, y - 1)].modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn the_hot_path_is_highlighted_and_other_processes_are_not() {
        let (buffer, _) = draw(&sample_view(), 100, 20);
        let (_, pane) = find(&buffer, "%3 claude");
        assert!((pane..pane + 4).all(|y| buffer[(20, y)].fg == Color::Yellow));
        assert_ne!(buffer[(20, pane + 4)].fg, Color::Yellow);
        assert_ne!(buffer[(20, pane - 1)].fg, Color::Yellow);
    }

    #[rstest]
    #[case::green(42.0, Color::Green)]
    #[case::yellow(65.0, Color::Yellow)]
    #[case::red(90.0, Color::Red)]
    fn the_context_column_takes_the_level_colour(#[case] context: f64, #[case] color: Color) {
        let mut view = sample_view();
        view.rows[4].context = Some(context);
        let (buffer, _) = draw(&view, 120, 40);
        let (x, y) = find(&buffer, &format!("{context:.0}%"));
        assert_eq!(buffer[(x, y)].fg, color);
    }

    #[test]
    fn the_active_tab_is_reversed() {
        let (buffer, _) = draw(&sample_view(), 100, 20);
        let (processes, y) = find(&buffer, "Processes");
        let (context, _) = find(&buffer, "Context");
        assert!(buffer[(processes, y)].modifier.contains(Modifier::REVERSED));
        assert!(!buffer[(context, y)].modifier.contains(Modifier::REVERSED));
    }
}

mod hits {
    use super::*;

    fn hit_on(text: &str, offset: u16) -> Option<Hit> {
        let (buffer, hits) = draw(&sample_view(), 120, 20);
        let (x, y) = find(&buffer, text);
        hits.at(x + offset, y)
    }

    #[rstest]
    #[case::processes_tab("Processes", 0, Some(Hit::Tab(ViewTab::Processes)))]
    #[case::context_tab("Context ", 0, Some(Hit::Tab(ViewTab::Context)))]
    #[case::the_graphs_are_not_clickable(" cpu ", 2, None)]
    #[case::name_column("Name", 0, Some(Hit::Sort(Sort::Name)))]
    #[case::memory_column("Memory", 0, Some(Hit::Sort(Sort::Memory)))]
    #[case::cpu_column("CPU ", 0, Some(Hit::Sort(Sort::Cpu)))]
    #[case::the_sparkline_column_sorts_by_cpu("CPU 60s", 4, Some(Hit::Sort(Sort::Cpu)))]
    #[case::agent_column("Agent", 0, Some(Hit::Sort(Sort::Agent)))]
    #[case::status_column("Status", 0, Some(Hit::Sort(Sort::Status)))]
    #[case::context_column("Status", 9, Some(Hit::Sort(Sort::Context)))]
    #[case::project_arrow("▾ cctop", 0, Some(Hit::Fold(0)))]
    #[case::project_label("▾ cctop", 3, Some(Hit::Row(0)))]
    #[case::pane_arrow("▾ %3 claude", 1, Some(Hit::Fold(4)))]
    #[case::pane_before_its_arrow("▾ %3 claude", 0, Some(Hit::Fold(4)))]
    #[case::leaf_process_has_no_arrow("cargo test", 0, Some(Hit::Row(7)))]
    #[case::the_bottom_border("q/Esc", 0, None)]
    fn point_at_what_is_under_the_mouse(#[case] text: &str, #[case] offset: u16, #[case] expected: Option<Hit>) {
        assert_eq!(hit_on(text, offset), expected);
    }

    #[test]
    fn the_border_and_a_line_below_the_last_row_are_not_rows() {
        let (buffer, hits) = draw(&sample_view(), 120, 20);
        let (_, last) = find(&buffer, "▸ notes");
        assert_eq!(hits.at(0, last), None);
        assert_eq!(hits.at(10, last + 1), None);
    }

    #[test]
    fn rows_keep_their_index_when_scrolled() {
        let (buffer, hits) = draw(&View { scroll: 4, ..sample_view() }, 100, 20);
        let (x, y) = find(&buffer, "▾ %3 claude");
        assert_eq!(hits.at(x, y), Some(Hit::Fold(4)));
        assert_eq!(hits.at(30, y + 1), Some(Hit::Row(5)));
    }
}

mod narrow {
    use super::*;

    #[test]
    fn long_names_are_cut_at_the_end_without_moving_columns() {
        let long = "x".repeat(200);
        let view = View { rows: vec![row(0, RowKind::Process, &long, 512 * MIB)], selected: None, ..sample_view() };
        let (buffer, _) = draw(&view, 100, 8);
        let shown = line(&buffer, 2);
        let cut = shown.find('…').expect("cut");
        assert_eq!(shown[cut..].chars().nth(1), Some(' '));
        assert!(shown.contains(" 512 MB "), "{shown}");
    }

    #[test]
    fn deep_rows_draw_without_panicking() {
        let deep = Row { folded: Some(false), ..row(u16::MAX, RowKind::Process, "deep", MIB) };
        let (_, hits) = draw(&View { rows: vec![deep], selected: None, ..sample_view() }, 100, 4);
        assert_eq!(hits.at(5, 2), Some(Hit::Row(0)));
    }

    #[rstest]
    #[case::both_totals("╭─ Processes ─ Context ─ mem 6.1 GB  cpu 314% ─╮", 48)]
    #[case::drops_the_cpu_whole("╭─ Processes ─ Context ─────── mem 6.1 GB ─╮", 44)]
    #[case::drops_both_totals("╭─ Processes ─ Context ───────────╮", 35)]
    fn totals_that_do_not_fit_are_left_out_whole(#[case] expected: &str, #[case] width: u16) {
        let (buffer, _) = draw(&sample_view(), width, 3);
        assert_eq!(line(&buffer, 0), expected);
    }

    #[rstest]
    #[case::empty(0, 0)]
    #[case::one_cell(1, 1)]
    #[case::one_line(40, 1)]
    #[case::tiny(10, 3)]
    #[case::minimum_width(40, 20)]
    #[case::graphs_with_one_line_of_table(60, 16)]
    fn small_terminals_draw_without_panicking(#[case] width: u16, #[case] height: u16) {
        let view = View { scroll: 100, ..sample_view() };
        draw(&view, width, height);
        draw(&View { tab: ViewTab::Context, ..view }, width, height);
    }
}

mod dialog {
    use super::*;

    fn asking() -> View {
        let dialog = Dialog {
            process: "cargo test --locked".to_owned(),
            pid: 4242,
            pgid: 4200,
            pane: "%3 claude".to_owned(),
            focus: Button::Process,
        };
        View { dialog: Some(dialog), ..sample_view() }
    }

    #[test]
    fn names_the_process_its_pid_and_its_pane_over_the_tree() {
        insta::assert_snapshot!(snapshot(&asking(), 100, 20).backend());
    }

    #[rstest]
    #[case::cancel(28, 11, Some(Hit::Button(Button::Cancel)))]
    #[case::process(38, 11, Some(Hit::Button(Button::Process)))]
    #[case::group(50, 11, Some(Hit::Button(Button::Group)))]
    #[case::the_dialog_text(30, 8, None)]
    #[case::a_row_outside_the_dialog(5, 5, None)]
    #[case::a_tab(4, 3, None)]
    fn only_its_buttons_can_be_clicked(#[case] column: u16, #[case] row: u16, #[case] expected: Option<Hit>) {
        let (_, hits) = draw(&asking(), 100, 20);
        assert_eq!(hits.at(column, row), expected);
    }

    #[test]
    fn the_focused_button_is_reversed() {
        let (buffer, _) = draw(&asking(), 100, 20);
        assert!(buffer[(38, 11)].modifier.contains(Modifier::REVERSED));
        assert!(!buffer[(28, 11)].modifier.contains(Modifier::REVERSED));
    }

    #[rstest]
    #[case::empty(0, 0)]
    #[case::narrow(12, 20)]
    #[case::short(100, 4)]
    fn small_terminals_draw_it_without_panicking(#[case] width: u16, #[case] height: u16) {
        draw(&asking(), width, height);
    }
}
