use std::collections::HashMap;
use std::time::Instant;

use rstest::rstest;
use serde::Deserialize;

use super::{PaneNode, ProcessNode, Sort, Totals, Tree, build, pane_processes};
use crate::cornercase::Snapshot;
use crate::procs::{Pid, Process, Sample};
use crate::test_util::fixture;

const STATUS: &str = fixture!("cornercase/status-0.12.173.json");
const SAMPLE: &str = fixture!("usage/sample.json");
const BOOT: &str = "4242-1700000000000000000";

#[derive(Deserialize)]
struct Fixture {
    processes: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    pid: Pid,
    ppid: Option<Pid>,
    name: String,
    memory: Option<u64>,
    cpu: Option<f64>,
    pane: Option<u64>,
    server: Option<String>,
}

fn fixture_input() -> (Sample, HashMap<Pid, f64>) {
    let entries = serde_json::from_str::<Fixture>(SAMPLE).expect("parse the sample fixture").processes;
    let cpu = entries.iter().filter_map(|entry| Some((entry.pid, entry.cpu?))).collect();
    let processes = entries
        .into_iter()
        .map(|e| Process {
            pid: e.pid,
            ppid: e.ppid,
            name: e.name,
            memory: e.memory,
            pane: e.pane,
            server: e.server,
            ..Process::default()
        })
        .collect();
    (Sample { taken: Instant::now(), processes }, cpu)
}

fn fixture_tree(by: Option<Sort>) -> Tree {
    let snapshot = Snapshot::parse(STATUS).expect("parse the status fixture");
    let (sample, cpu) = fixture_input();
    build(&snapshot, &sample, &cpu, by)
}

fn snapshot_with_panes(panes: &[u64]) -> Snapshot {
    let panes: Vec<String> = panes.iter().map(|id| format!(r#"{{"id": {id}}}"#)).collect();
    let json = format!(
        r#"{{"boot": "{BOOT}", "projects": [{{"id": 1, "workspaces": [{{"id": 1, "tabs": [{{"id": 1, "panes": [{}]}}]}}]}}]}}"#,
        panes.join(",")
    );
    Snapshot::parse(&json).expect("parse the snapshot")
}

fn process(pid: Pid, parent: Option<Pid>, pane: Option<u64>, server: Option<&str>) -> Process {
    Process { pid, ppid: parent, pane, server: server.map(str::to_owned), memory: Some(1), ..Process::default() }
}

fn panes(tree: &Tree) -> Vec<&PaneNode> {
    tree.projects.iter().flat_map(|p| &p.workspaces).flat_map(|w| &w.tabs).flat_map(|t| &t.panes).collect()
}

fn pane(tree: &Tree, id: u64) -> &PaneNode {
    panes(tree).into_iter().find(|pane| pane.id == id).expect("the pane is in the tree")
}

fn shape(nodes: &[ProcessNode]) -> String {
    let parts: Vec<String> = nodes
        .iter()
        .map(|node| match node.children.as_slice() {
            [] => node.pid.to_string(),
            children => format!("{}({})", node.pid, shape(children)),
        })
        .collect();
    parts.join(",")
}

fn hot_pids(nodes: &[ProcessNode]) -> Vec<Pid> {
    nodes.iter().filter(|node| node.hot).flat_map(|node| [vec![node.pid], hot_pids(&node.children)].concat()).collect()
}

fn all_pids(tree: &Tree) -> Vec<Pid> {
    fn walk(nodes: &[ProcessNode], out: &mut Vec<Pid>) {
        for node in nodes {
            out.push(node.pid);
            walk(&node.children, out);
        }
    }
    let mut out = Vec::new();
    for pane in panes(tree) {
        walk(&pane.processes, &mut out);
    }
    out
}

mod assignment {
    use super::*;

    fn shape_of(processes: Vec<Process>, boot: Option<&str>) -> String {
        let mut snapshot = snapshot_with_panes(&[1]);
        snapshot.boot = boot.map(str::to_owned);
        let sample = Sample { taken: Instant::now(), processes };
        shape(&pane(&build(&snapshot, &sample, &HashMap::new(), None), 1).processes)
    }

    #[rstest]
    #[case::its_own_variables(vec![process(5, Some(1), Some(1), Some(BOOT))], "5")]
    #[case::a_grandchild_with_a_clean_environment(
        vec![process(5, Some(1), Some(1), Some(BOOT)), process(6, Some(5), None, None), process(7, Some(6), None, None)],
        "5(6(7))"
    )]
    #[case::the_nearest_ancestor_wins(
        vec![process(5, Some(1), Some(2), Some(BOOT)), process(6, Some(5), Some(1), Some(BOOT)), process(7, Some(6), None, None)],
        "6(7)"
    )]
    #[case::not_another_server(
        vec![process(5, Some(1), Some(1), Some("1-2")), process(6, Some(5), None, None)],
        ""
    )]
    #[case::not_a_pane_without_server(vec![process(5, Some(1), Some(1), None)], "")]
    #[case::not_a_process_without_marked_ancestors(vec![process(5, Some(1), None, None)], "")]
    #[case::roots_whose_parent_is_elsewhere(
        vec![process(5, Some(1), Some(1), Some(BOOT)), process(9, Some(8), Some(1), Some(BOOT)), process(8, Some(1), Some(2), Some(BOOT))],
        "5,9"
    )]
    #[case::a_parent_cycle_without_marks(vec![process(5, Some(6), None, None), process(6, Some(5), None, None)], "")]
    #[case::a_parent_cycle_inside_a_pane_from_its_lowest_pid(
        vec![process(6, Some(5), Some(1), Some(BOOT)), process(5, Some(6), Some(1), Some(BOOT)), process(7, Some(6), None, None)],
        "5(6(7))"
    )]
    #[case::a_branch_hanging_from_a_parent_cycle(
        vec![process(5, Some(6), Some(1), Some(BOOT)), process(6, Some(5), Some(1), Some(BOOT)), process(4, Some(6), Some(1), Some(BOOT))],
        "5(6(4))"
    )]
    fn maps_a_process_by(#[case] processes: Vec<Process>, #[case] expected: &str) {
        assert_eq!(shape_of(processes, Some(BOOT)), expected);
    }

    #[test]
    fn maps_nothing_without_a_boot_token() {
        assert_eq!(shape_of(vec![process(5, Some(1), Some(1), Some(BOOT))], None), "");
    }

    #[test]
    fn nests_the_fixture_by_parent_pid() {
        let tree = fixture_tree(None);
        let shapes: Vec<(u64, String)> = panes(&tree).iter().map(|pane| (pane.id, shape(&pane.processes))).collect();
        let expected = [(1, "100(101(102(103(104)),105))"), (2, "200(201(202))"), (7, "300(301)"), (5, "")];
        assert_eq!(shapes, expected.map(|(id, shape)| (id, shape.to_owned())));
    }

    #[test]
    fn leaves_out_other_servers_and_unknown_panes() {
        let pids = all_pids(&fixture_tree(None));
        for outsider in [1, 10, 400, 401, 500] {
            assert!(!pids.contains(&outsider), "{outsider} is in {pids:?}");
        }
    }

    #[test]
    fn copies_the_pane_data_from_the_snapshot() {
        let tree = fixture_tree(None);
        let claude = pane(&tree, 7);
        let fields = (claude.agent.as_deref(), claude.status.as_deref(), claude.model.as_deref());
        assert_eq!(fields, (Some("claude"), Some("idle"), Some("sonnet")));
        assert_eq!((claude.context_percent, claude.subagents, claude.parent), (Some(7), Some(0), Some(1)));
        assert_eq!(claude.command.as_deref(), Some("claude"));
    }

    #[test]
    fn keeps_a_pane_without_processes_empty() {
        let tree = fixture_tree(Some(Sort::Memory));
        let empty = pane(&tree, 5);
        assert_eq!(empty.processes, []);
        assert_eq!(empty.totals, Totals::default());
    }
}

mod pane_processes {
    use super::*;

    #[test]
    fn lists_a_pane_tree_parents_first_in_pid_order() {
        let processes = vec![
            process(10, Some(1), Some(1), Some(BOOT)),
            process(13, Some(10), None, None),
            process(11, Some(10), None, None),
            process(12, Some(11), None, None),
            process(20, Some(1), Some(2), Some(BOOT)),
        ];
        let sample = Sample { taken: Instant::now(), processes };
        let pids: Vec<Pid> =
            pane_processes(&snapshot_with_panes(&[1, 2]), &sample, 1).iter().map(|process| process.pid).collect();
        assert_eq!(pids, [10, 11, 12, 13]);
    }
}

mod totals {
    use super::*;

    fn assert_sum_of_children(parent: Totals, children: impl IntoIterator<Item = Totals>) {
        assert_eq!(parent, Totals::sum(children));
    }

    #[test]
    fn are_the_sum_of_the_children_at_every_level() {
        let tree = fixture_tree(Some(Sort::Cpu));
        assert_sum_of_children(tree.totals, tree.projects.iter().map(|p| p.totals));
        for project in &tree.projects {
            assert_sum_of_children(project.totals, project.workspaces.iter().map(|w| w.totals));
            for workspace in &project.workspaces {
                assert_sum_of_children(workspace.totals, workspace.tabs.iter().map(|t| t.totals));
            }
        }
        for tab in tree.projects.iter().flat_map(|p| &p.workspaces).flat_map(|w| &w.tabs) {
            assert_sum_of_children(tab.totals, tab.panes.iter().map(|p| p.totals));
        }
        for pane in panes(&tree) {
            assert_sum_of_children(pane.totals, pane.processes.iter().map(|p| p.totals));
        }
    }

    #[test]
    fn add_a_process_and_its_whole_tree() {
        let tree = fixture_tree(None);
        let claude = &pane(&tree, 1).processes[0].children[0];
        let memory = 400_000_000 + 3_000_000 + 90_000_000 + 4_000_000_000 + 60_000_000;
        assert_eq!(claude.totals, Totals { memory: Some(memory), memory_partial: false, cpu: Some(133.0) });
    }

    #[test]
    fn leave_unknown_memory_out_and_mark_it_up_to_the_root() {
        let tree = fixture_tree(None);
        let cargo = pane(&tree, 2);
        assert_eq!(cargo.totals.memory, Some(85_000_000));
        assert!(cargo.totals.memory_partial);
        assert!(tree.projects.iter().find(|p| p.id == 1).expect("notes").totals.memory_partial);
        assert!(!tree.projects.iter().find(|p| p.id == 2).expect("shop").totals.memory_partial);
    }

    #[rstest]
    #[case::both_known(Totals::of(Some(2), Some(1.0)), Totals::of(Some(3), Some(2.0)), Totals::of(Some(5), Some(3.0)))]
    #[case::one_unknown(Totals::of(None, None), Totals::of(Some(3), Some(2.0)), Totals { memory: Some(3), memory_partial: true, cpu: Some(2.0) })]
    #[case::nothing(Totals::default(), Totals::default(), Totals::default())]
    fn add(#[case] a: Totals, #[case] b: Totals, #[case] expected: Totals) {
        assert_eq!(a.add(b), expected);
    }
}

mod sorting {
    use super::*;

    fn order(tree: &Tree) -> (Vec<u64>, Vec<u64>, Vec<u64>) {
        let projects = tree.projects.iter().map(|p| p.id).collect();
        let workspaces = tree.projects.iter().flat_map(|p| &p.workspaces).map(|w| w.id).collect();
        let panes = panes(tree).iter().map(|p| p.id).collect();
        (projects, workspaces, panes)
    }

    #[rstest]
    #[case::cornercase_order_without_a_sort(None, (vec![1, 2], vec![1, 2, 3], vec![1, 2, 7, 5]), "100(101(102(103(104)),105))")]
    #[case::memory(Some(Sort::Memory), (vec![1, 2], vec![1, 2, 3], vec![1, 2, 7, 5]), "100(101(102(103(104)),105))")]
    #[case::cpu(Some(Sort::Cpu), (vec![1, 2], vec![1, 2, 3], vec![2, 1, 7, 5]), "100(101(105,102(103(104))))")]
    fn orders_every_level(
        #[case] by: Option<Sort>,
        #[case] levels: (Vec<u64>, Vec<u64>, Vec<u64>),
        #[case] claude: &str,
    ) {
        let tree = fixture_tree(by);
        assert_eq!(order(&tree), levels);
        assert_eq!(shape(&pane(&tree, 1).processes), claude);
    }

    #[test]
    fn puts_the_costlier_project_first_only_when_sorting() {
        let mut snapshot = Snapshot::parse(STATUS).expect("parse the status fixture");
        snapshot.projects.reverse();
        let (sample, cpu) = fixture_input();
        let ids = |by| build(&snapshot, &sample, &cpu, by).projects.iter().map(|p| p.id).collect::<Vec<_>>();
        assert_eq!([ids(None), ids(Some(Sort::Memory))], [vec![2, 1], vec![1, 2]]);
    }

    #[test]
    fn puts_the_costlier_workspace_and_tab_first_only_when_sorting() {
        let mut snapshot = Snapshot::parse(STATUS).expect("parse the status fixture");
        let notes = snapshot.projects.iter_mut().find(|p| p.id == 1).expect("notes");
        notes.workspaces.reverse();
        notes.workspaces.iter_mut().for_each(|workspace| workspace.tabs.reverse());
        let (sample, cpu) = fixture_input();
        let ids = |by| {
            let tree = build(&snapshot, &sample, &cpu, by);
            let notes = tree.projects.into_iter().find(|p| p.id == 1).expect("notes");
            let tabs = notes.workspaces.iter().find(|w| w.id == 1).expect("main").tabs.iter().map(|t| t.id).collect();
            (notes.workspaces.iter().map(|w| w.id).collect::<Vec<_>>(), tabs)
        };
        assert_eq!(ids(None), (vec![2, 1], vec![2, 1]));
        assert_eq!(ids(Some(Sort::Memory)), (vec![1, 2], vec![1, 2]));
    }

    #[test]
    fn breaks_ties_by_cornercase_order_and_then_by_pid() {
        let snapshot = snapshot_with_panes(&[3, 1, 2]);
        let processes = vec![
            process(9, Some(1), Some(1), Some(BOOT)),
            process(8, Some(1), Some(1), Some(BOOT)),
            process(7, Some(1), Some(3), Some(BOOT)),
        ];
        let sample = Sample { taken: Instant::now(), processes };
        let tree = build(&snapshot, &sample, &HashMap::new(), Some(Sort::Memory));
        let ids: Vec<u64> = panes(&tree).iter().map(|p| p.id).collect();
        assert_eq!((ids, shape(&pane(&tree, 1).processes)), (vec![1, 3, 2], "8,9".to_owned()));
    }
}

mod hot_path {
    use super::*;

    #[rstest]
    #[case::memory_to_the_test_binary(Some(Sort::Memory), vec![100, 101, 102, 103, 104])]
    #[case::memory_by_default(None, vec![100, 101, 102, 103, 104])]
    #[case::cpu_to_the_busiest_process(Some(Sort::Cpu), vec![100, 101, 105])]
    fn marks_the_costliest_process_and_its_ancestors(#[case] by: Option<Sort>, #[case] expected: Vec<Pid>) {
        assert_eq!(hot_pids(&pane(&fixture_tree(by), 1).processes), expected);
    }

    #[test]
    fn marks_one_path_per_pane() {
        let tree = fixture_tree(Some(Sort::Memory));
        assert_eq!(hot_pids(&pane(&tree, 2).processes), [200, 201]);
        assert_eq!(hot_pids(&pane(&tree, 7).processes), [300, 301]);
    }

    #[test]
    fn marks_nothing_when_no_process_has_a_figure() {
        let snapshot = snapshot_with_panes(&[1]);
        let processes = vec![process(5, Some(1), Some(1), Some(BOOT))];
        let sample = Sample { taken: Instant::now(), processes };
        let tree = build(&snapshot, &sample, &HashMap::new(), Some(Sort::Cpu));
        assert_eq!(hot_pids(&pane(&tree, 1).processes), Vec::<Pid>::new());
    }
}
