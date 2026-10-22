use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use rstest::rstest;

use super::{Flow, Lists, handle};
use crate::app::state::{RowId, State};
use crate::app::view::Line;
use crate::ui::{Hits, Row, ViewTab};
use crate::usage::Sort;

const AGENTS: [u64; 3] = [7, 1, 2];

fn line(id: RowId, folded: Option<bool>) -> Line {
    Line { id, row: Row { folded, ..Row::default() } }
}

fn lines() -> Vec<Line> {
    vec![
        line(RowId::Project(1), Some(false)),
        line(RowId::Pane(1), Some(false)),
        line(RowId::Process { pane: 1, pid: 10 }, None),
        line(RowId::Pane(2), None),
    ]
}

fn press(state: &mut State, code: KeyCode) -> Flow {
    handle(state, &Lists { lines: &lines(), agents: &AGENTS }, &Hits::default(), &Event::Key(KeyEvent::from(code)))
}

fn selected_at(at: usize) -> State {
    let mut state = State::new(None);
    state.select(&lines(), at);
    state
}

mod keys {
    use super::*;

    #[rstest]
    #[case::down_from_nothing_takes_the_first(None, KeyCode::Down, 0)]
    #[case::up_from_nothing_takes_the_last(None, KeyCode::Up, 3)]
    #[case::down_moves_one(Some(1), KeyCode::Down, 2)]
    #[case::down_stops_at_the_end(Some(3), KeyCode::Down, 3)]
    #[case::up_stops_at_the_top(Some(0), KeyCode::Up, 0)]
    #[case::page_down_stops_at_the_end(Some(0), KeyCode::PageDown, 3)]
    fn arrows_move_the_selection(#[case] from: Option<usize>, #[case] code: KeyCode, #[case] to: usize) {
        let mut state = from.map_or_else(|| State::new(None), selected_at);
        state.page = 10;
        press(&mut state, code);
        assert_eq!(state.index(&lines()), Some(to));
    }

    #[test]
    fn moving_below_the_page_scrolls_to_keep_the_selection_visible() {
        let mut state = selected_at(1);
        state.page = 2;
        press(&mut state, KeyCode::Down);
        assert_eq!(state.scroll, 1);
    }

    #[test]
    fn enter_toggles_the_fold_and_left_right_set_it() {
        let mut state = selected_at(1);
        press(&mut state, KeyCode::Enter);
        assert!(state.folded.contains(&RowId::Pane(1)));
        press(&mut state, KeyCode::Right);
        assert!(!state.folded.contains(&RowId::Pane(1)));
        press(&mut state, KeyCode::Left);
        assert!(state.folded.contains(&RowId::Pane(1)));
    }

    #[test]
    fn enter_on_a_row_without_children_folds_nothing() {
        let mut state = selected_at(2);
        press(&mut state, KeyCode::Enter);
        assert!(state.folded.is_empty());
    }

    #[test]
    fn tab_switches_the_view_both_ways() {
        let mut state = State::new(None);
        press(&mut state, KeyCode::Tab);
        assert_eq!(state.tab, ViewTab::Context);
        press(&mut state, KeyCode::BackTab);
        assert_eq!(state.tab, ViewTab::Processes);
    }

    #[test]
    fn s_cycles_the_sort() {
        let mut state = State::new(Some(Sort::Memory));
        press(&mut state, KeyCode::Char('s'));
        assert_eq!(state.sort, Some(Sort::Cpu));
        press(&mut state, KeyCode::Char('s'));
        assert_eq!(state.sort, None);
    }

    #[test]
    fn q_and_esc_quit_and_other_keys_continue() {
        assert_eq!(press(&mut State::new(None), KeyCode::Char('q')), Flow::Quit);
        assert_eq!(press(&mut State::new(None), KeyCode::Esc), Flow::Quit);
        assert_eq!(press(&mut State::new(None), KeyCode::Char('x')), Flow::Continue);
    }

    #[test]
    fn a_key_release_does_nothing() {
        let mut state = State::new(None);
        let release = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        };
        let lists = Lists { lines: &lines(), agents: &AGENTS };
        assert_eq!(handle(&mut state, &lists, &Hits::default(), &Event::Key(release)), Flow::Continue);
    }
}

mod choosing_an_agent {
    use super::*;

    fn in_context(agent: Option<u64>) -> State {
        let mut state = State::new(None);
        state.tab = ViewTab::Context;
        state.agent = agent;
        state
    }

    #[rstest]
    #[case::down_moves_one(Some(7), KeyCode::Down, Some(1))]
    #[case::up_moves_one(Some(1), KeyCode::Up, Some(7))]
    #[case::down_stops_at_the_end(Some(2), KeyCode::Down, Some(2))]
    #[case::up_stops_at_the_top(Some(7), KeyCode::Up, Some(7))]
    #[case::from_nothing_takes_the_first(None, KeyCode::Down, Some(7))]
    #[case::from_one_that_left_takes_the_first(Some(9), KeyCode::Up, Some(7))]
    fn arrows_move_the_mark(#[case] from: Option<u64>, #[case] code: KeyCode, #[case] to: Option<u64>) {
        let mut state = in_context(from);
        press(&mut state, code);
        assert_eq!(state.agent, to);
    }

    #[test]
    fn arrows_leave_the_process_list_alone() {
        let mut state = in_context(Some(7));
        press(&mut state, KeyCode::Down);
        assert_eq!(state.selected, None);
    }

    #[test]
    fn without_agents_nothing_is_marked() {
        let mut state = in_context(None);
        handle(
            &mut state,
            &Lists { lines: &lines(), agents: &[] },
            &Hits::default(),
            &Event::Key(KeyEvent::from(KeyCode::Down)),
        );
        assert_eq!(state.agent, None);
    }
}

mod sorting_by_column {
    use super::*;
    use crate::ui;

    #[rstest]
    #[case::name_restores_cornercase_order(ui::Sort::Name, None)]
    #[case::memory(ui::Sort::Memory, Some(Sort::Memory))]
    #[case::cpu(ui::Sort::Cpu, Some(Sort::Cpu))]
    #[case::agent_is_not_sortable(ui::Sort::Agent, Some(Sort::Memory))]
    #[case::context_is_not_sortable(ui::Sort::Context, Some(Sort::Memory))]
    fn a_column_sets_the_tree_sort(#[case] column: ui::Sort, #[case] expected: Option<Sort>) {
        let mut state = State::new(Some(Sort::Memory));
        state.sort_by(column);
        assert_eq!(state.sort, expected);
    }
}
