use std::collections::HashSet;

use super::state::{RowId, State, column};
use crate::ui::{ContextView, Header, Row, RowKind, View};
use crate::usage::{PaneNode, ProcessNode, ProjectNode, TabNode, Totals, Tree, WorkspaceNode};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Line {
    pub(super) id: RowId,
    pub(super) row: Row,
}

pub(super) fn lines(tree: &Tree, folded: &HashSet<RowId>) -> Vec<Line> {
    let mut flat = Flat { folded, lines: Vec::new() };
    tree.projects.iter().for_each(|project| flat.project(project));
    flat.lines
}

pub(super) fn view(state: &State, lines: &[Line], header: Header, context: ContextView) -> View {
    View {
        tab: state.tab,
        rows: lines.iter().map(|line| line.row.clone()).collect(),
        scroll: state.scroll,
        selected: state.index(lines),
        sort: column(state.sort),
        header,
        context,
    }
}

struct Flat<'a> {
    folded: &'a HashSet<RowId>,
    lines: Vec<Line>,
}

impl Flat<'_> {
    fn push(&mut self, id: RowId, row: Row, children: bool) -> bool {
        let folded = self.folded.contains(&id);
        self.lines.push(Line { id, row: Row { folded: children.then_some(folded), ..row } });
        children && !folded
    }

    fn project(&mut self, project: &ProjectNode) {
        let row = node(0, RowKind::Project, project_label(project), project.totals);
        if self.push(RowId::Project(project.id), row, !project.workspaces.is_empty()) {
            project.workspaces.iter().for_each(|workspace| self.workspace(workspace));
        }
    }

    fn workspace(&mut self, workspace: &WorkspaceNode) {
        let row = node(1, RowKind::Workspace, workspace_label(workspace), workspace.totals);
        if self.push(RowId::Workspace(workspace.id), row, !workspace.tabs.is_empty()) {
            workspace.tabs.iter().for_each(|tab| self.tab(tab));
        }
    }

    fn tab(&mut self, tab: &TabNode) {
        let label = tab.name.clone().unwrap_or_else(|| format!("tab {}", tab.id));
        if self.push(RowId::Tab(tab.id), node(2, RowKind::Tab, label, tab.totals), !tab.panes.is_empty()) {
            tab.panes.iter().for_each(|pane| self.pane(pane));
        }
    }

    fn pane(&mut self, pane: &PaneNode) {
        let row = Row {
            agent: pane.agent.clone(),
            status: pane.status.clone(),
            context: pane.context_percent.map(f64::from),
            ..node(3, RowKind::Pane, pane_label(pane), pane.totals)
        };
        if self.push(RowId::Pane(pane.id), row, !pane.processes.is_empty()) {
            pane.processes.iter().for_each(|process| self.process(pane.id, 4, process));
        }
    }

    fn process(&mut self, pane: u64, depth: u16, process: &ProcessNode) {
        let row = Row { hot: process.hot, ..node(depth, RowKind::Process, process_label(process), process.totals) };
        if self.push(RowId::Process { pane, pid: process.pid }, row, !process.children.is_empty()) {
            process.children.iter().for_each(|child| self.process(pane, depth.saturating_add(1), child));
        }
    }
}

fn node(depth: u16, kind: RowKind, label: String, totals: Totals) -> Row {
    Row {
        depth,
        kind,
        label,
        memory: totals.memory,
        cpu: totals.cpu,
        estimated: totals.memory_partial,
        ..Row::default()
    }
}

fn project_label(project: &ProjectNode) -> String {
    let folder =
        project.path.as_ref().and_then(|path| path.file_name()).map(|name| name.to_string_lossy().into_owned());
    project.name.clone().or(folder).unwrap_or_else(|| format!("project {}", project.id))
}

fn workspace_label(workspace: &WorkspaceNode) -> String {
    let name = workspace.name.clone().or_else(|| workspace.branch.clone());
    let name = name.unwrap_or_else(|| format!("workspace {}", workspace.id));
    if workspace.worktree { format!("{name} (worktree)") } else { name }
}

pub(super) fn pane_label(pane: &PaneNode) -> String {
    match pane.command.as_ref().or(pane.agent.as_ref()) {
        Some(program) => format!("%{} {program}", pane.id),
        None => format!("%{}", pane.id),
    }
}

fn process_label(process: &ProcessNode) -> String {
    let mut words = vec![process.name.as_str()];
    words.extend(process.args.iter().skip(1).map(String::as_str));
    words.join(" ")
}
