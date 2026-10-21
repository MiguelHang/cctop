use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use rstest::rstest;

use super::scroll_max;
use crate::ui::tests::line;
use crate::ui::{ContextPart, ContextPercent, ContextSession, ContextView, Header, View, ViewTab, render};

fn part(label: &str, tokens: u64) -> ContextPart {
    ContextPart { label: label.to_owned(), tokens, path: false }
}

fn file(label: &str, tokens: u64) -> ContextPart {
    ContextPart { path: true, ..part(label, tokens) }
}

fn session() -> ContextSession {
    ContextSession {
        pane: "%1 claude".to_owned(),
        pid: 4242,
        total: Some(60_000),
        percent: Some(ContextPercent { used: 6.0, window: 1_000_000, cornercase: 6.0 }),
        fixed: vec![part("System prompt", 9_000), file("AGENTS.md", 5_000), file("DESIGN.md", 1_000)],
        dynamic: vec![file("src/main.rs", 30_000), part("$ cargo test --locked", 10_000), part("User prompts", 5_000)],
        scroll: 0,
    }
}

fn view(context: ContextView) -> View {
    View {
        tab: ViewTab::Context,
        header: Header { memory: Some(1 << 30), ..Header::default() },
        context,
        ..View::default()
    }
}

fn draw(context: ContextView, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal.draw(|frame| drop(render(frame, &view(context)))).expect("draw");
    terminal
}

mod layout {
    use super::*;

    #[test]
    fn draws_the_total_the_bar_and_both_parts_at_80_by_20() {
        insta::assert_snapshot!(draw(ContextView::Session(session()), 80, 20).backend());
    }

    #[test]
    fn without_a_total_says_the_parts_are_estimates() {
        let context = ContextView::Session(ContextSession { total: None, percent: None, ..session() });
        let terminal = draw(context, 80, 6);
        assert!(line(terminal.backend().buffer(), 2).starts_with("Total —  no reply since the last compaction"));
    }

    #[test]
    fn the_bar_splits_fixed_and_dynamic_by_their_share() {
        let terminal = draw(ContextView::Session(session()), 80, 6);
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(19, 3)].fg, Color::Cyan);
        assert_eq!(buffer[(20, 3)].fg, Color::Magenta);
    }

    #[test]
    fn long_paths_keep_their_file_name() {
        let long = format!("{}/AGENTS.md", "deep/".repeat(30));
        let context = ContextView::Session(ContextSession { fixed: vec![file(&long, 100)], ..session() });
        let text = line(draw(context, 60, 10).backend().buffer(), 7);
        assert!(text.trim_start().starts_with('…'), "{text}");
        assert!(text.contains("/AGENTS.md"), "{text}");
    }

    #[test]
    fn empty_lists_show_a_dash() {
        let context = ContextView::Session(ContextSession { fixed: Vec::new(), ..session() });
        assert_eq!(line(draw(context, 60, 10).backend().buffer(), 7).trim(), "—");
    }

    #[rstest]
    #[case::empty(0, 0)]
    #[case::only_the_tabs(40, 1)]
    #[case::tiny(10, 3)]
    #[case::narrow(20, 12)]
    fn small_terminals_draw_without_panicking(#[case] width: u16, #[case] height: u16) {
        draw(ContextView::Session(ContextSession { scroll: 100, ..session() }), width, height);
    }
}

mod empty_states {
    use super::*;

    #[rstest]
    #[case::no_pane_selected("no_pane_selected", ContextView::NoPane)]
    #[case::reading("reading", ContextView::Reading { pane: "%1 claude".to_owned() })]
    #[case::pane_without_claude("pane_without_claude", ContextView::NoClaude { pane: "%5 zsh".to_owned() })]
    #[case::session_not_found(
        "session_not_found",
        ContextView::NoSession { pane: "%1 claude".to_owned(), pid: 4242 }
    )]
    fn each_has_its_text(#[case] name: &str, #[case] context: ContextView) {
        insta::assert_snapshot!(name, draw(context, 80, 4).backend());
    }
}

mod scrolling {
    use super::*;

    #[test]
    fn the_wheel_moves_the_lists_and_keeps_the_header() {
        let context = ContextView::Session(ContextSession { scroll: 2, ..session() });
        let terminal = draw(context, 80, 12);
        let buffer = terminal.backend().buffer();
        assert!(line(buffer, 1).starts_with("%1 claude"));
        assert!(line(buffer, 6).contains("AGENTS.md"), "{}", line(buffer, 6));
    }

    #[test]
    fn stops_when_the_last_part_is_on_screen() {
        let context = ContextView::Session(session());
        assert_eq!(scroll_max(&context, 30), 0);
        assert_eq!(scroll_max(&context, 10), 5);
        assert_eq!(scroll_max(&ContextView::NoPane, 10), 0);
    }
}
