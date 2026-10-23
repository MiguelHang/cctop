use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use rstest::rstest;

use super::scroll_max;
use crate::test_util::find;
use crate::ui::tests::line;
use crate::ui::{
    AgentRow, ContextDetail, ContextGroup, ContextPart, ContextPercent, ContextSession, ContextView, Header, Hit, Hits,
    View, ViewTab, render,
};

const PANE: &str = "shop › checkout";
const TITLE: &str = "shop › checkout · claude Opus 5.5 · working";

fn part(label: &str, tokens: u64) -> ContextPart {
    ContextPart { label: label.to_owned(), tokens, path: false }
}

fn file(label: &str, tokens: u64) -> ContextPart {
    ContextPart { path: true, ..part(label, tokens) }
}

fn group(name: &str, noun: &str, tokens: u64, items: Vec<ContextPart>) -> ContextGroup {
    ContextGroup {
        name: name.to_owned(),
        noun: noun.to_owned(),
        tokens,
        open: !items.is_empty(),
        items,
        ..ContextGroup::default()
    }
}

fn commands() -> ContextGroup {
    let items =
        vec![part("grep -n -i window src", 18_000), part("cat docs/NOTES.md", 7_000), part("cargo test", 5_000)];
    ContextGroup { more: 37, more_tokens: 68_000, ..group("Commands", "command", 98_000, items) }
}

fn session() -> ContextSession {
    ContextSession {
        total: Some(196_000),
        percent: Some(ContextPercent { used: 19.6, window: 1_000_000 }),
        loaded: vec![
            part("Skills", 18_000),
            part("System prompt", 5_000),
            file("AGENTS.md", 5_000),
            part("Agents", 3_000),
        ],
        groups: vec![
            commands(),
            group("Claude's replies", "", 30_000, Vec::new()),
            group("Your messages", "", 20_000, Vec::new()),
            group("Files", "file", 17_000, vec![file("src/cart.rs", 9_000), file("src/search_bar.rs", 8_000)]),
        ],
        scroll: 0,
    }
}

fn agents() -> Vec<AgentRow> {
    vec![
        AgentRow { label: "shop › search-bar".to_owned(), percent: Some(85.0) },
        AgentRow { label: PANE.to_owned(), percent: Some(20.0) },
        AgentRow { label: "notes › main".to_owned(), percent: Some(60.0) },
        AgentRow { label: "notes › drafts".to_owned(), percent: None },
    ]
}

fn view(detail: ContextDetail) -> View {
    View {
        tab: ViewTab::Context,
        header: Header { memory: Some(1 << 30), ..Header::default() },
        context: ContextView { agents: agents(), marked: Some(1), title: TITLE.to_owned(), detail },
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

fn screen(buffer: &Buffer) -> String {
    (0..buffer.area.height).map(|y| line(buffer, y)).collect::<Vec<_>>().join("\n")
}

fn with(change: impl FnOnce(&mut ContextSession)) -> ContextDetail {
    let mut session = session();
    change(&mut session);
    ContextDetail::Session(session)
}

mod layout {
    use super::*;

    #[test]
    fn draws_the_band_the_agents_and_the_conversation_at_120_by_40() {
        insta::assert_snapshot!(draw(ContextDetail::Session(session()), 120, 40).backend());
    }

    #[test]
    fn draws_the_band_the_agents_and_the_conversation_at_80_by_24() {
        insta::assert_snapshot!(draw(ContextDetail::Session(session()), 80, 24).backend());
    }

    #[test]
    fn stacks_the_agents_over_the_conversation_at_40_columns() {
        insta::assert_snapshot!(draw(ContextDetail::Session(session()), 40, 24).backend());
    }

    #[rstest]
    #[case::wide(120, 40)]
    #[case::standard(80, 24)]
    #[case::narrow(40, 24)]
    fn uses_the_new_words_only(#[case] width: u16, #[case] height: u16) {
        let shown = screen(draw(ContextDetail::Session(session()), width, height).backend().buffer());
        for old in ["fixed", "dynamic", "cornercase", "≈"] {
            assert!(!shown.contains(old), "{old} in\n{shown}");
        }
    }

    #[rstest]
    #[case::wide(120, 40)]
    #[case::standard(80, 24)]
    #[case::short(100, 12)]
    #[case::narrow(40, 24)]
    fn switching_tabs_keeps_every_box_border_in_place(#[case] width: u16, #[case] height: u16) {
        let context = view(ContextDetail::Session(session()));
        let processes = View { tab: ViewTab::Processes, ..context.clone() };
        let corners = |view: &View| {
            let (terminal, _) = draw_view(view, width, height);
            let buffer = terminal.backend().buffer().clone();
            (0..height).filter(|&y| ["╭", "╰"].contains(&buffer[(0, y)].symbol())).collect::<Vec<u16>>()
        };
        let shown = corners(&context);
        assert!(corners(&processes).iter().all(|y| shown.contains(y)), "{shown:?} against {:?}", corners(&processes));
        assert_eq!(shown.first(), corners(&processes).first());
    }

    #[rstest]
    #[case::empty(0, 0)]
    #[case::one_line(40, 1)]
    #[case::tiny(10, 3)]
    #[case::narrow(20, 12)]
    #[case::short_and_wide(120, 4)]
    #[case::band_and_little_else(60, 16)]
    fn small_terminals_draw_without_panicking(#[case] width: u16, #[case] height: u16) {
        draw(with(|session| session.scroll = 100), width, height);
    }
}

mod band {
    use super::*;

    #[test]
    fn names_the_agent_and_how_full_its_context_is() {
        let terminal = draw(ContextDetail::Session(session()), 120, 40);
        let buffer = terminal.backend().buffer();
        assert!(line(buffer, 0).contains(&format!(" {TITLE} ")), "{}", line(buffer, 0));
        assert!(line(buffer, 1).contains("Context used 196k of 1M tokens"), "{}", line(buffer, 1));
        assert!(line(buffer, 1).trim_end_matches('│').trim_end().ends_with("20%"), "{}", line(buffer, 1));
    }

    #[test]
    fn the_legend_says_what_each_part_of_the_bar_is_in_words() {
        let terminal = draw(ContextDetail::Session(session()), 120, 40);
        let bottom = line(terminal.backend().buffer(), 3);
        assert!(bottom.contains("▓ loaded on every message 31.0k"), "{bottom}");
        assert!(bottom.contains("▒ this conversation 165k"), "{bottom}");
        assert!(bottom.contains("░ free 804k"), "{bottom}");
    }

    #[rstest]
    #[case::green(19.6, Color::Green)]
    #[case::yellow(50.0, Color::Yellow)]
    #[case::red(80.0, Color::Red)]
    fn the_bar_and_percent_take_the_level_colour(#[case] used: f64, #[case] color: Color) {
        let terminal =
            draw(with(|session| session.percent = Some(ContextPercent { used, window: 1_000_000 })), 120, 40);
        let buffer = terminal.backend().buffer();
        let (x, y) = find(buffer, "%│");
        assert_eq!(buffer[(x - 1, y)].fg, color);
        assert_eq!((buffer[(1, 2)].symbol(), buffer[(1, 2)].fg), ("▓", color));
        assert_eq!((buffer[(118, 2)].symbol(), buffer[(118, 2)].fg), ("░", Color::DarkGray));
    }

    #[test]
    fn a_short_band_puts_the_bar_beside_the_words() {
        let terminal = draw(ContextDetail::Session(session()), 100, 20);
        let used = line(terminal.backend().buffer(), 1);
        assert!(used.contains("Context used 196k of 1M tokens  ▓"), "{used}");
        assert!(used.contains("░   20%│"), "{used}");
    }

    #[rstest]
    #[case::bar_on_its_own_line(120, 40, 3)]
    #[case::bar_beside_the_words(100, 20, 2)]
    fn without_the_window_no_bar_is_drawn(#[case] width: u16, #[case] height: u16, #[case] bottom: u16) {
        let terminal = draw(with(|session| session.percent = None), width, height);
        let buffer = terminal.backend().buffer();
        let band = (0..=bottom).map(|y| line(buffer, y)).collect::<String>();
        assert!(band.contains("Context used 196k tokens"), "{band}");
        assert!(band.contains("loaded on every message 31.0k"), "{band}");
        assert!(!band.contains('▓') && !band.contains('▒') && !band.contains('░'), "{band}");
    }

    #[test]
    fn without_a_session_the_use_is_missing() {
        let terminal = draw(ContextDetail::Reading { pane: PANE.to_owned() }, 120, 20);
        assert!(line(terminal.backend().buffer(), 1).contains("Context used —"));
    }

    #[test]
    fn a_hidden_band_moves_the_use_to_the_conversation_title() {
        let top = line(draw(ContextDetail::Session(session()), 120, 12).backend().buffer(), 0);
        assert!(top.contains(" 196k of 1M · 20% "), "{top}");
    }
}

mod conversation {
    use super::*;

    #[test]
    fn groups_show_their_total_share_and_three_largest_parts() {
        let terminal = draw(ContextDetail::Session(session()), 120, 40);
        let buffer = terminal.backend().buffer();
        let (_, y) = find(buffer, "▾ Commands");
        assert!(line(buffer, y).contains("98.0k  59%"), "{}", line(buffer, y));
        assert!(line(buffer, y + 1).contains("    grep -n -i window src"), "{}", line(buffer, y + 1));
        assert!(line(buffer, y + 3).contains("    cargo test"), "{}", line(buffer, y + 3));
        assert!(line(buffer, y + 4).contains("    … 37 more"), "{}", line(buffer, y + 4));
        assert!(line(buffer, y + 4).contains("68.0k"), "{}", line(buffer, y + 4));
        assert!(line(buffer, y + 5).contains("  Claude's replies"), "{}", line(buffer, y + 5));
    }

    #[test]
    fn a_folded_group_hides_its_parts() {
        let terminal = draw(with(|session| session.groups[0].open = false), 120, 40);
        let buffer = terminal.backend().buffer();
        let (_, y) = find(buffer, "▸ Commands");
        assert!(line(buffer, y + 1).contains("Claude's replies"), "{}", line(buffer, y + 1));
    }

    #[test]
    fn only_groups_with_parts_are_clickable() {
        let (terminal, hits) = draw_view(&view(ContextDetail::Session(session())), 120, 40);
        let buffer = terminal.backend().buffer();
        let (x, y) = find(buffer, "Commands");
        assert_eq!(hits.at(x, y), Some(Hit::Group(0)));
        let (x, y) = find(buffer, "Claude's replies");
        assert_eq!(hits.at(x, y), None);
        let (x, y) = find(buffer, "▾ Files");
        assert_eq!(hits.at(x, y), Some(Hit::Group(3)));
    }

    #[test]
    fn the_title_has_the_conversation_total() {
        let top = line(draw(ContextDetail::Session(session()), 120, 40).backend().buffer(), 4);
        assert!(top.contains(" what fills this conversation (165k) "), "{top}");
    }

    #[test]
    fn an_empty_conversation_shows_a_dash() {
        let terminal = draw(with(|session| session.groups.clear()), 120, 40);
        assert!(line(terminal.backend().buffer(), 5).contains("││—"), "{}", line(terminal.backend().buffer(), 5));
    }

    #[test]
    fn long_paths_keep_their_file_name() {
        let long = format!("{}/cart.rs", "deep/".repeat(30));
        let terminal = draw(with(|session| session.groups[3].items[0] = file(&long, 9_000)), 120, 40);
        let (_, y) = find(terminal.backend().buffer(), "/cart.rs");
        assert!(line(terminal.backend().buffer(), y).contains("    …"), "{}", line(terminal.backend().buffer(), y));
    }
}

mod loaded {
    use super::*;

    #[test]
    fn the_loaded_parts_share_one_line_under_their_heading() {
        let terminal = draw(ContextDetail::Session(session()), 120, 40);
        let buffer = terminal.backend().buffer();
        let (_, y) = find(buffer, "├─ loaded on every message (31.0k) ");
        assert!(line(buffer, y + 1).contains("Skills 18.0k · System prompt 5.0k · AGENTS.md 5.0k · Agents 3.0k"));
    }

    #[test]
    fn the_line_wraps_when_it_does_not_fit() {
        let terminal = draw(ContextDetail::Session(session()), 40, 30);
        let buffer = terminal.backend().buffer();
        let (_, y) = find(buffer, "├─ loaded on every message");
        assert!(line(buffer, y + 1).contains("Skills 18.0k · System prompt 5.0k"), "{}", line(buffer, y + 1));
        assert!(line(buffer, y + 2).contains("AGENTS.md 5.0k · Agents 3.0k"), "{}", line(buffer, y + 2));
    }

    #[test]
    fn compact_frees_the_conversation_and_keeps_the_loaded_part() {
        let shown = screen(draw(ContextDetail::Session(session()), 120, 40).backend().buffer());
        assert!(shown.contains("/compact would free about 165k; the 31.0k stays."), "{shown}");
        assert!(shown.contains("The split is estimated; the total is Claude's own."), "{shown}");
    }

    #[rstest]
    #[case::over_a_tenth(18_000, true)]
    #[case::a_tenth_or_less(16_500, false)]
    fn names_one_part_only_over_a_tenth_of_the_conversation(#[case] tokens: u64, #[case] named: bool) {
        let detail = with(|session| session.groups[0].items[0].tokens = tokens);
        let shown = screen(draw(detail, 120, 40).backend().buffer());
        assert_eq!(shown.contains("One command, `grep -n -i window src`, took"), named, "{shown}");
    }

    #[test]
    fn without_a_total_says_the_parts_are_estimates() {
        let detail = with(|session| {
            session.total = None;
            session.percent = None;
        });
        let shown = screen(draw(detail, 120, 40).backend().buffer());
        assert!(shown.contains("No reply since the last compaction: estimates only."), "{shown}");
        assert!(shown.contains("Context used: unknown until Claude's next reply"), "{shown}");
    }
}

mod agents_list {
    use super::*;

    #[test]
    fn the_marked_agent_is_reversed_and_each_percent_has_its_level_colour() {
        let terminal = draw(ContextDetail::Session(session()), 120, 40);
        let buffer = terminal.backend().buffer();
        let (_, y) = find(buffer, "▶ shop › checkout");
        assert!(buffer[(5, y)].modifier.contains(Modifier::REVERSED));
        assert!(!buffer[(5, y - 1)].modifier.contains(Modifier::REVERSED));
        let (x, y) = find(buffer, "85%");
        assert_eq!(buffer[(x, y)].fg, Color::Red);
        let (x, y) = find(buffer, "60%");
        assert_eq!(buffer[(x, y)].fg, Color::Yellow);
        assert!(line(buffer, y + 1).contains(" —│"), "{}", line(buffer, y + 1));
    }

    #[test]
    fn says_how_the_list_is_sorted() {
        let terminal = draw(ContextDetail::Session(session()), 120, 40);
        let (x, _) = find(terminal.backend().buffer(), "sorted by context used");
        assert_eq!(x, 1);
    }

    #[test]
    fn a_box_too_narrow_for_both_tabs_keeps_the_active_one() {
        let terminal = draw(ContextDetail::Session(session()), 20, 12);
        let top = line(terminal.backend().buffer(), 0);
        assert!(top.contains(" Context ") && !top.contains("Processes"), "{top}");
    }

    #[test]
    fn a_click_on_an_agent_names_it() {
        let (terminal, hits) = draw_view(&view(ContextDetail::Session(session())), 120, 40);
        let (x, y) = find(terminal.backend().buffer(), "notes › main");
        assert_eq!(hits.at(x, y), Some(Hit::Agent(2)));
    }

    #[test]
    fn the_list_scrolls_to_keep_the_mark_on_screen() {
        let many: Vec<AgentRow> =
            (0..30).map(|n| AgentRow { label: format!("shop › tab{n}"), percent: Some(1.0) }).collect();
        let mut shown = view(ContextDetail::NoAgent);
        shown.context = ContextView { agents: many, marked: Some(25), ..ContextView::default() };
        let (terminal, hits) = draw_view(&shown, 120, 12);
        let (x, y) = find(terminal.backend().buffer(), "tab25");
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
        insta::assert_snapshot!(name, draw(detail, 100, 8).backend());
    }

    #[test]
    fn the_footer_offers_enter_only_with_a_session() {
        let empty = line(draw(ContextDetail::NoAgent, 120, 40).backend().buffer(), 39);
        let session = line(draw(ContextDetail::Session(session()), 120, 40).backend().buffer(), 39);
        assert!(!empty.contains('⏎') && empty.contains("Tab view"), "{empty}");
        assert!(session.contains("⏎ open/fold all"), "{session}");
    }

    #[test]
    fn an_error_takes_the_right_of_the_conversation_title() {
        let mut shown = view(ContextDetail::Session(session()));
        shown.header.error = Some("cornercase: server not running".to_owned());
        let (terminal, _) = draw_view(&shown, 120, 40);
        let (x, y) = find(terminal.backend().buffer(), "cornercase: server not running");
        assert_eq!((y, terminal.backend().buffer()[(x, y)].fg), (4, Color::Red));
    }
}

mod scrolling {
    use super::*;

    fn area(width: u16, height: u16) -> ratatui::layout::Rect {
        ratatui::layout::Rect::new(0, 0, width, height)
    }

    #[test]
    fn the_wheel_moves_the_groups_and_keeps_the_loaded_part() {
        let terminal = draw(with(|session| session.scroll = 2), 120, 14);
        let buffer = terminal.backend().buffer();
        assert!(line(buffer, 1).contains("cat docs/NOTES.md"), "{}", line(buffer, 1));
        let shown = screen(buffer);
        assert!(shown.contains("loaded on every message"), "{shown}");
    }

    #[test]
    fn stops_when_the_last_group_is_on_screen() {
        let shown = view(ContextDetail::Session(session()));
        assert_eq!(scroll_max(&shown, area(120, 40)), 0);
        assert_eq!(scroll_max(&shown, area(120, 14)), 10 - 7);
        assert_eq!(scroll_max(&view(ContextDetail::NoAgent), area(120, 7)), 0);
    }
}
