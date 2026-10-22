use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::{Color, Modifier};
use rstest::rstest;

use super::scroll_max;
use crate::test_util::find;
use crate::ui::tests::line;
use crate::ui::{
    AgentRow, ContextDetail, ContextPart, ContextPercent, ContextSession, ContextView, Header, Hit, Hits, View,
    ViewTab, render,
};

const PANE: &str = "shopfront › search-bar › %149";

fn part(label: &str, tokens: u64) -> ContextPart {
    ContextPart { label: label.to_owned(), tokens, path: false }
}

fn file(label: &str, tokens: u64) -> ContextPart {
    ContextPart { path: true, ..part(label, tokens) }
}

fn session() -> ContextSession {
    ContextSession {
        pane: PANE.to_owned(),
        total: Some(60_000),
        percent: Some(ContextPercent { used: 30.0, window: 200_000, cornercase: 31.0 }),
        fixed: vec![part("System prompt", 9_000), file("AGENTS.md", 5_000), file("DESIGN.md", 1_000)],
        dynamic: vec![file("src/main.rs", 30_000), part("$ cargo test --locked", 10_000), part("User prompts", 5_000)],
        scroll: 0,
    }
}

fn agents() -> Vec<AgentRow> {
    vec![
        AgentRow { label: PANE.to_owned(), percent: Some(30.0) },
        AgentRow { label: "checkout-flow › main › %155".to_owned(), percent: Some(11.0) },
        AgentRow { label: "cctop › main › %233".to_owned(), percent: None },
    ]
}

fn view(detail: ContextDetail) -> View {
    View {
        tab: ViewTab::Context,
        header: Header { memory: Some(1 << 30), ..Header::default() },
        context: ContextView { agents: agents(), marked: Some(0), detail },
        ..View::default()
    }
}

fn draw_view(view: &View, width: u16, height: u16) -> (Terminal<TestBackend>, Hits) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    let mut hits = Hits::default();
    terminal.draw(|frame| hits = render(frame, view)).expect("draw");
    (terminal, hits)
}

fn draw(detail: ContextDetail, width: u16, height: u16) -> Terminal<TestBackend> {
    draw_view(&view(detail), width, height).0
}

mod layout {
    use super::*;

    #[test]
    fn draws_the_agents_and_the_breakdown_at_120_by_40() {
        insta::assert_snapshot!(draw(ContextDetail::Session(session()), 120, 40).backend());
    }

    #[test]
    fn draws_the_agents_and_the_breakdown_at_80_by_24() {
        insta::assert_snapshot!(draw(ContextDetail::Session(session()), 80, 24).backend());
    }

    #[test]
    fn stacks_the_agents_over_one_list_at_40_columns() {
        insta::assert_snapshot!(draw(ContextDetail::Session(session()), 40, 24).backend());
    }

    #[test]
    fn the_title_has_the_place_the_total_and_its_share_of_the_window() {
        let terminal = draw(ContextDetail::Session(session()), 120, 20);
        let top = line(terminal.backend().buffer(), 0);
        assert!(top.contains(&format!(" {PANE} ")), "{top}");
        assert!(top.contains(" 60.0k · 30% "), "{top}");
    }

    #[test]
    fn without_a_total_says_the_parts_are_estimates() {
        let detail = ContextDetail::Session(ContextSession { total: None, percent: None, ..session() });
        let top = line(draw(detail, 120, 10).backend().buffer(), 0);
        assert!(top.contains("estimates only"), "{top}");
    }

    #[test]
    fn the_bar_splits_fixed_dynamic_and_free_against_the_window() {
        let terminal = draw(ContextDetail::Session(session()), 120, 20);
        let buffer = terminal.backend().buffer();
        let colors: Vec<Color> = [0, 5, 6, 22, 23, 70].iter().map(|&x| buffer[(41 + x, 1)].fg).collect();
        assert_eq!(colors[..2], [Color::Cyan, Color::Cyan]);
        assert_eq!(colors[2..4], [Color::Magenta, Color::Magenta]);
        assert_eq!(colors[4..], [Color::DarkGray, Color::DarkGray]);
    }

    #[test]
    fn the_legend_names_each_part_and_cornercase_s_percent() {
        let terminal = draw(ContextDetail::Session(session()), 120, 20);
        let legend = line(terminal.backend().buffer(), 2);
        assert!(legend.contains("█ fixed ≈15.0k  ▓ dynamic ≈45.0k  ░ free 140k  cornercase 31%"), "{legend}");
    }

    #[test]
    fn long_paths_keep_their_file_name() {
        let long = format!("{}/AGENTS.md", "deep/".repeat(30));
        let detail = ContextDetail::Session(ContextSession { fixed: vec![file(&long, 100)], ..session() });
        let terminal = draw(detail, 120, 10);
        let (_, y) = find(terminal.backend().buffer(), "/AGENTS.md");
        assert!(line(terminal.backend().buffer(), y).contains("││…"), "{}", line(terminal.backend().buffer(), y));
    }

    #[test]
    fn empty_lists_show_a_dash() {
        let detail = ContextDetail::Session(ContextSession { fixed: Vec::new(), ..session() });
        let terminal = draw(detail, 120, 10);
        let (x, y) = find(terminal.backend().buffer(), "├─ fixed ≈0 ");
        assert_eq!(terminal.backend().buffer()[(x + 1, y + 1)].symbol(), "—");
    }

    #[rstest]
    #[case::empty(0, 0)]
    #[case::one_line(40, 1)]
    #[case::tiny(10, 3)]
    #[case::narrow(20, 12)]
    #[case::short_and_wide(120, 4)]
    fn small_terminals_draw_without_panicking(#[case] width: u16, #[case] height: u16) {
        draw(ContextDetail::Session(ContextSession { scroll: 100, ..session() }), width, height);
    }
}

mod agents_list {
    use super::*;

    #[test]
    fn the_marked_agent_is_reversed_with_its_bar_and_percent() {
        let terminal = draw(ContextDetail::Session(session()), 120, 20);
        let buffer = terminal.backend().buffer();
        assert!(line(buffer, 1).starts_with("│▶ search-bar › %149 "), "{}", line(buffer, 1));
        assert!(line(buffer, 1).contains("30%│"));
        assert!(buffer[(5, 1)].modifier.contains(Modifier::REVERSED));
        assert!(!buffer[(5, 2)].modifier.contains(Modifier::REVERSED));
        assert!(line(buffer, 3).contains(" —│"), "{}", line(buffer, 3));
    }

    #[test]
    fn a_box_too_narrow_for_both_tabs_keeps_the_active_one() {
        let terminal = draw(ContextDetail::Session(session()), 20, 12);
        let top = line(terminal.backend().buffer(), 0);
        assert!(top.contains(" Context ") && !top.contains("Processes"), "{top}");
    }

    #[test]
    fn a_click_on_an_agent_names_it() {
        let (terminal, hits) = draw_view(&view(ContextDetail::Session(session())), 120, 20);
        let (x, y) = find(terminal.backend().buffer(), "%155");
        assert_eq!(hits.at(x, y), Some(Hit::Agent(1)));
        assert_eq!(hits.at(x, y + 5), None);
    }

    #[test]
    fn the_list_scrolls_to_keep_the_mark_on_screen() {
        let many: Vec<AgentRow> =
            (0..30).map(|n| AgentRow { label: format!("p › w › %{n}"), percent: Some(1.0) }).collect();
        let mut shown = view(ContextDetail::NoAgent);
        shown.context = ContextView { agents: many, marked: Some(25), detail: ContextDetail::NoAgent };
        let (terminal, hits) = draw_view(&shown, 120, 12);
        let (x, y) = find(terminal.backend().buffer(), "%25");
        assert_eq!(y, 10);
        assert_eq!(hits.at(x, y), Some(Hit::Agent(25)));
    }

    #[test]
    fn the_tabs_are_on_its_top_border() {
        let (terminal, hits) = draw_view(&view(ContextDetail::NoAgent), 120, 20);
        let (x, y) = find(terminal.backend().buffer(), "Processes");
        assert_eq!(hits.at(x, y), Some(Hit::Tab(ViewTab::Processes)));
        assert!(terminal.backend().buffer()[(x + 12, y)].modifier.contains(Modifier::REVERSED));
    }
}

mod empty_states {
    use super::*;

    #[rstest]
    #[case::no_agent("no_agent", ContextDetail::NoAgent)]
    #[case::reading("reading", ContextDetail::Reading { pane: PANE.to_owned() })]
    #[case::pane_without_claude("pane_without_claude", ContextDetail::NoClaude { pane: PANE.to_owned() })]
    #[case::session_not_found("session_not_found", ContextDetail::NoSession { pane: PANE.to_owned(), pid: 4242 })]
    fn each_has_its_text(#[case] name: &str, #[case] detail: ContextDetail) {
        insta::assert_snapshot!(name, draw(detail, 100, 6).backend());
    }

    #[test]
    fn an_error_takes_the_right_of_the_title() {
        let mut shown = view(ContextDetail::Session(session()));
        shown.header.error = Some("cornercase: server not running".to_owned());
        let (terminal, _) = draw_view(&shown, 120, 10);
        let (x, y) = find(terminal.backend().buffer(), "cornercase: server not running");
        assert_eq!((y, terminal.backend().buffer()[(x, y)].fg), (0, Color::Red));
    }
}

mod scrolling {
    use super::*;

    fn area(width: u16, height: u16) -> ratatui::layout::Rect {
        ratatui::layout::Rect::new(0, 0, width, height)
    }

    #[test]
    fn the_wheel_moves_the_lists_and_keeps_the_header() {
        let detail = ContextDetail::Session(ContextSession { scroll: 2, ..session() });
        let terminal = draw(detail, 120, 7);
        let buffer = terminal.backend().buffer();
        assert!(line(buffer, 3).contains(" fixed ≈15.0k "), "{}", line(buffer, 3));
        assert!(line(buffer, 4).contains("AGENTS.md"), "{}", line(buffer, 4));
        assert!(line(buffer, 4).contains("$ cargo test"), "{}", line(buffer, 4));
    }

    #[test]
    fn stops_when_the_last_part_is_on_screen() {
        let shown = view(ContextDetail::Session(session()));
        assert_eq!(scroll_max(&shown, area(120, 30)), 0);
        assert_eq!(scroll_max(&shown, area(120, 7)), 1);
        assert_eq!(scroll_max(&shown, area(40, 12)), 7 - 3);
        assert_eq!(scroll_max(&view(ContextDetail::NoAgent), area(120, 7)), 0);
    }
}
