use std::collections::HashSet;

use super::history::History;
use super::state::{RowId, State, column};
use crate::ui::{ContextView, Header, Row, RowKind, View};
use crate::usage::{PaneNode, ProcessNode, ProjectNode, TabNode, Totals, Tree, WorkspaceNode};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Line {
    pub(super) id: RowId,
    pub(super) row: Row,
}

pub(super) fn lines(tree: &Tree, unfolded: &HashSet<RowId>) -> Vec<Line> {
    let mut flat = Flat { unfolded, lines: Vec::new() };
    tree.projects.iter().for_each(|project| flat.project(project));
    flat.lines
}

pub(super) fn view(state: &State, lines: &[Line], history: &History, header: Header, context: ContextView) -> View {
    View {
        tab: state.tab,
        rows: rows(state, lines, history),
        scroll: state.scroll,
        selected: state.index(lines),
        sort: column(state.sort),
        header,
        context,
        ..View::default()
    }
}

fn rows(state: &State, lines: &[Line], history: &History) -> Vec<Row> {
    let visible = state.scroll..state.scroll.saturating_add(state.page.max(1));
    lines
        .iter()
        .enumerate()
        .map(|(at, line)| {
            let cpu_history = if visible.contains(&at) { history.row(line.id) } else { Vec::new() };
            Row { cpu_history, ..line.row.clone() }
        })
        .collect()
}

struct Flat<'a> {
    unfolded: &'a HashSet<RowId>,
    lines: Vec<Line>,
}

impl Flat<'_> {
    fn push(&mut self, id: RowId, row: Row, children: bool) -> bool {
        let folded = !self.unfolded.contains(&id);
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

pub(super) fn path(tree: &Tree, id: RowId) -> Vec<RowId> {
    tree.projects.iter().find_map(|project| project_path(project, id)).unwrap_or_default()
}

fn within(own: RowId, id: RowId, inner: impl FnOnce() -> Option<Vec<RowId>>) -> Option<Vec<RowId>> {
    if own == id {
        return Some(vec![own]);
    }
    let mut rest = inner()?;
    rest.insert(0, own);
    Some(rest)
}

fn project_path(project: &ProjectNode, id: RowId) -> Option<Vec<RowId>> {
    within(RowId::Project(project.id), id, || project.workspaces.iter().find_map(|w| workspace_path(w, id)))
}

fn workspace_path(workspace: &WorkspaceNode, id: RowId) -> Option<Vec<RowId>> {
    within(RowId::Workspace(workspace.id), id, || workspace.tabs.iter().find_map(|tab| tab_path(tab, id)))
}

fn tab_path(tab: &TabNode, id: RowId) -> Option<Vec<RowId>> {
    within(RowId::Tab(tab.id), id, || tab.panes.iter().find_map(|pane| pane_path(pane, id)))
}

fn pane_path(pane: &PaneNode, id: RowId) -> Option<Vec<RowId>> {
    within(RowId::Pane(pane.id), id, || pane.processes.iter().find_map(|process| process_path(pane.id, process, id)))
}

fn process_path(pane: u64, process: &ProcessNode, id: RowId) -> Option<Vec<RowId>> {
    let own = RowId::Process { pane, pid: process.pid };
    within(own, id, || process.children.iter().find_map(|child| process_path(pane, child, id)))
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

pub(super) fn project_label(project: &ProjectNode) -> String {
    let folder =
        project.path.as_ref().and_then(|path| path.file_name()).map(|name| name.to_string_lossy().into_owned());
    project.name.clone().or(folder).unwrap_or_else(|| format!("project {}", project.id))
}

fn workspace_label(workspace: &WorkspaceNode) -> String {
    let name = workspace_name(workspace);
    if workspace.worktree { format!("{name} (worktree)") } else { name }
}

pub(super) fn workspace_name(workspace: &WorkspaceNode) -> String {
    let name = workspace.name.clone().or_else(|| workspace.branch.clone());
    name.unwrap_or_else(|| format!("workspace {}", workspace.id))
}

fn pane_label(pane: &PaneNode) -> String {
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
