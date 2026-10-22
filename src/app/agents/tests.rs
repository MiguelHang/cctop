use std::collections::HashMap;
use std::time::Instant;

use rstest::rstest;

use super::{agents, candidate, under};
use crate::app::state::RowId;
use crate::app::tests::{STATUS, first_sample};
use crate::cornercase::Snapshot;
use crate::usage::{self, Tree};

fn tree() -> Tree {
    let snapshot = Snapshot::parse(STATUS).expect("parse the status fixture");
    usage::build(&snapshot, &first_sample(Instant::now()), &HashMap::new(), None)
}

fn set_percent(tree: &mut Tree, pane: u64, percent: Option<u16>) {
    let tabs = tree.projects.iter_mut().flat_map(|project| &mut project.workspaces).flat_map(|w| &mut w.tabs);
    for found in tabs.flat_map(|tab| &mut tab.panes).filter(|found| found.id == pane) {
        found.context_percent = percent;
    }
}

mod listing {
    use super::*;

    #[test]
    fn lists_the_claude_panes_by_context_with_their_place() {
        let listed: Vec<_> =
            agents(&tree()).into_iter().map(|agent| (agent.pane, agent.label, agent.percent)).collect();
        let expected =
            [(1, "notes › main › %1".to_owned(), Some(42)), (7, "notes › feature-x › %7".to_owned(), Some(7))];
        assert_eq!(listed, expected);
    }

    #[test]
    fn a_pane_without_a_percent_goes_last() {
        let mut tree = tree();
        set_percent(&mut tree, 1, None);
        let order: Vec<u64> = agents(&tree).iter().map(|agent| agent.pane).collect();
        assert_eq!(order, [7, 1]);
    }

    #[test]
    fn ties_keep_the_order_of_the_tree() {
        let mut tree = tree();
        set_percent(&mut tree, 7, Some(42));
        let order: Vec<u64> = agents(&tree).iter().map(|agent| agent.pane).collect();
        assert_eq!(order, [1, 7]);
    }
}

mod choosing {
    use super::*;

    #[rstest]
    #[case::nothing_selected_takes_the_most_context(None, Some(1))]
    #[case::its_pane(Some(RowId::Pane(7)), Some(7))]
    #[case::a_process_of_its_pane(Some(RowId::Process { pane: 7, pid: 300 }), Some(7))]
    #[case::its_tab(Some(RowId::Tab(3)), Some(7))]
    #[case::its_workspace(Some(RowId::Workspace(2)), Some(7))]
    #[case::a_project_with_two_takes_the_most_context(Some(RowId::Project(1)), Some(1))]
    #[case::a_pane_without_claude(Some(RowId::Pane(2)), Some(1))]
    #[case::another_project(Some(RowId::Project(2)), Some(1))]
    fn the_selection_in_processes_picks_the_agent(#[case] selected: Option<RowId>, #[case] expected: Option<u64>) {
        assert_eq!(candidate(&agents(&tree()), selected), expected);
    }

    #[test]
    fn a_row_without_claude_has_none_under_it() {
        assert_eq!(under(&agents(&tree()), RowId::Pane(2)), None);
        assert_eq!(under(&agents(&tree()), RowId::Tab(3)), Some(7));
    }

    #[test]
    fn no_agents_pick_nothing() {
        assert_eq!(candidate(&[], Some(RowId::Pane(1))), None);
    }
}
