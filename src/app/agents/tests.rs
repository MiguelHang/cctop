use std::collections::HashMap;
use std::time::Instant;

use rstest::rstest;

use super::{agents, candidate, under};
use crate::app::state::RowId;
use crate::app::tests::{STATUS, first_sample};
use crate::cornercase::Snapshot;
use crate::usage::{self, TabNode, Tree};

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
        let expected = [(1, "notes › main".to_owned(), Some(42)), (7, "notes › feature-x".to_owned(), Some(7))];
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

mod labels {
    use super::*;

    fn labels(tree: &Tree) -> Vec<String> {
        agents(tree).into_iter().map(|agent| agent.label).collect()
    }

    fn tabs(tree: &mut Tree) -> impl Iterator<Item = &mut TabNode> {
        tree.projects.iter_mut().flat_map(|project| &mut project.workspaces).flat_map(|w| &mut w.tabs)
    }

    #[test]
    fn a_tab_with_its_own_name_joins_the_place() {
        let mut tree = tree();
        tabs(&mut tree).filter(|tab| tab.id == 3).for_each(|tab| tab.name = Some("search-bar".to_owned()));
        assert_eq!(labels(&tree), ["notes › main", "notes › feature-x › search-bar"]);
    }

    #[rstest]
    #[case::the_agent("claude")]
    #[case::the_project("notes")]
    #[case::the_workspace("feature-x")]
    fn a_tab_named_like_its_agent_or_place_adds_nothing(#[case] name: &str) {
        let mut tree = tree();
        tabs(&mut tree).filter(|tab| tab.id == 3).for_each(|tab| tab.name = Some(name.to_owned()));
        assert_eq!(labels(&tree)[1], "notes › feature-x");
    }

    #[test]
    fn the_pane_id_only_breaks_ties() {
        let mut tree = tree();
        let workspaces = tree.projects.iter_mut().flat_map(|project| &mut project.workspaces);
        workspaces.for_each(|workspace| workspace.name = Some("checkout".to_owned()));
        assert_eq!(labels(&tree), ["notes › checkout %1", "notes › checkout %7"]);
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
