use std::cmp::Ordering;

use super::{ProcessNode, ProjectNode, Totals, Tree, WorkspaceNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Sort {
    #[default]
    Memory,
    Cpu,
}

impl Sort {
    pub(super) fn value(self, memory: Option<u64>, cpu: Option<f64>) -> Option<f64> {
        match self {
            #[expect(clippy::cast_precision_loss, reason = "only compared, never shown")]
            Self::Memory => memory.map(|memory| memory as f64),
            Self::Cpu => cpu,
        }
    }

    fn descending(self, a: Totals, b: Totals) -> Ordering {
        let of = |totals: Totals| self.value(totals.memory, totals.cpu).unwrap_or(0.0);
        of(b).total_cmp(&of(a))
    }
}

pub(super) fn sort(tree: &mut Tree, by: Option<Sort>) {
    tree.projects.iter_mut().for_each(|project| sort_project(project, by));
    arrange(&mut tree.projects, by, |project| project.totals);
}

fn sort_project(project: &mut ProjectNode, by: Option<Sort>) {
    project.workspaces.iter_mut().for_each(|workspace| sort_workspace(workspace, by));
    arrange(&mut project.workspaces, by, |workspace| workspace.totals);
}

fn sort_workspace(workspace: &mut WorkspaceNode, by: Option<Sort>) {
    for tab in &mut workspace.tabs {
        tab.panes.iter_mut().for_each(|pane| processes(&mut pane.processes, by));
        arrange(&mut tab.panes, by, |pane| pane.totals);
    }
    arrange(&mut workspace.tabs, by, |tab| tab.totals);
}

fn arrange<T>(items: &mut [T], by: Option<Sort>, totals: impl Fn(&T) -> Totals) {
    if let Some(by) = by {
        items.sort_by(|a, b| by.descending(totals(a), totals(b)));
    }
}

fn processes(nodes: &mut [ProcessNode], by: Option<Sort>) {
    match by {
        Some(by) => nodes.sort_by(|a, b| by.descending(a.totals, b.totals).then(a.pid.cmp(&b.pid))),
        None => nodes.sort_by_key(|node| node.pid),
    }
    for node in nodes.iter_mut() {
        processes(&mut node.children, by);
    }
}
