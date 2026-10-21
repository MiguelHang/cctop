use std::collections::HashMap;
use std::path::PathBuf;

use crate::cornercase::Snapshot;
use crate::cornercase::snapshot::{Pane, Project, Tab, Workspace};
use crate::procs::{Pid, Process, Sample};

mod assign;
mod hot;
mod sort;
mod totals;

use assign::Assignment;
pub(crate) use sort::Sort;
pub(crate) use totals::Totals;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Tree {
    pub(crate) projects: Vec<ProjectNode>,
    pub(crate) totals: Totals,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProjectNode {
    pub(crate) id: u64,
    pub(crate) name: Option<String>,
    pub(crate) path: Option<PathBuf>,
    pub(crate) workspaces: Vec<WorkspaceNode>,
    pub(crate) totals: Totals,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkspaceNode {
    pub(crate) id: u64,
    pub(crate) name: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) path: Option<PathBuf>,
    pub(crate) worktree: bool,
    pub(crate) tabs: Vec<TabNode>,
    pub(crate) totals: Totals,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TabNode {
    pub(crate) id: u64,
    pub(crate) name: Option<String>,
    pub(crate) panes: Vec<PaneNode>,
    pub(crate) totals: Totals,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PaneNode {
    pub(crate) id: u64,
    pub(crate) agent: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) context_percent: Option<u16>,
    pub(crate) subagents: Option<u32>,
    pub(crate) parent: Option<u64>,
    pub(crate) command: Option<String>,
    pub(crate) path: Option<PathBuf>,
    pub(crate) processes: Vec<ProcessNode>,
    pub(crate) totals: Totals,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProcessNode {
    pub(crate) pid: Pid,
    pub(crate) name: String,
    pub(crate) args: Vec<String>,
    pub(crate) memory: Option<u64>,
    pub(crate) cpu: Option<f64>,
    pub(crate) children: Vec<ProcessNode>,
    pub(crate) totals: Totals,
    pub(crate) hot: bool,
}

pub(crate) fn build(snapshot: &Snapshot, sample: &Sample, cpu: &HashMap<Pid, f64>, by: Option<Sort>) -> Tree {
    let assignment = Assignment::new(snapshot.boot.as_deref(), &sample.processes);
    let builder = Builder { assignment: &assignment, cpu };
    let projects: Vec<ProjectNode> = snapshot.projects.iter().map(|project| builder.project(project)).collect();
    let totals = Totals::sum(projects.iter().map(|project| project.totals));
    let mut tree = Tree { projects, totals };
    sort::sort(&mut tree, by);
    hot::mark(&mut tree, by.unwrap_or_default());
    tree
}

pub(crate) fn pane_processes<'a>(snapshot: &Snapshot, sample: &'a Sample, pane: u64) -> Vec<&'a Process> {
    let assignment = Assignment::new(snapshot.boot.as_deref(), &sample.processes);
    let mut found = Vec::new();
    let mut pending: Vec<&Process> = assignment.roots(pane).into_iter().rev().collect();
    while let Some(process) = pending.pop() {
        found.push(process);
        pending.extend(assignment.children(pane, process.pid).into_iter().rev());
    }
    found
}

struct Builder<'a> {
    assignment: &'a Assignment<'a>,
    cpu: &'a HashMap<Pid, f64>,
}

impl Builder<'_> {
    fn project(&self, project: &Project) -> ProjectNode {
        let workspaces: Vec<WorkspaceNode> = project.workspaces.iter().map(|w| self.workspace(w)).collect();
        ProjectNode {
            id: project.id,
            name: project.name.clone(),
            path: project.path.clone(),
            totals: Totals::sum(workspaces.iter().map(|workspace| workspace.totals)),
            workspaces,
        }
    }

    fn workspace(&self, workspace: &Workspace) -> WorkspaceNode {
        let tabs: Vec<TabNode> = workspace.tabs.iter().map(|tab| self.tab(tab)).collect();
        WorkspaceNode {
            id: workspace.id,
            name: workspace.name.clone(),
            branch: workspace.branch.clone(),
            path: workspace.path.clone(),
            worktree: workspace.worktree,
            totals: Totals::sum(tabs.iter().map(|tab| tab.totals)),
            tabs,
        }
    }

    fn tab(&self, tab: &Tab) -> TabNode {
        let panes: Vec<PaneNode> = tab.panes.iter().map(|pane| self.pane(pane)).collect();
        TabNode { id: tab.id, name: tab.name.clone(), totals: Totals::sum(panes.iter().map(|pane| pane.totals)), panes }
    }

    fn pane(&self, pane: &Pane) -> PaneNode {
        let processes: Vec<ProcessNode> =
            self.assignment.roots(pane.id).into_iter().map(|root| self.process(pane.id, root)).collect();
        PaneNode {
            id: pane.id,
            agent: pane.agent.clone(),
            status: pane.status.clone(),
            model: pane.model.clone(),
            context_percent: pane.context_percent,
            subagents: pane.subagents,
            parent: pane.parent,
            command: pane.command.clone(),
            path: pane.path.clone(),
            totals: Totals::sum(processes.iter().map(|process| process.totals)),
            processes,
        }
    }

    fn process(&self, pane: u64, process: &Process) -> ProcessNode {
        let cpu = self.cpu.get(&process.pid).copied();
        let children: Vec<ProcessNode> =
            self.assignment.children(pane, process.pid).into_iter().map(|child| self.process(pane, child)).collect();
        let own = Totals::of(process.memory, cpu);
        ProcessNode {
            pid: process.pid,
            name: process.name.clone(),
            args: process.args.clone(),
            memory: process.memory,
            cpu,
            totals: own.add(Totals::sum(children.iter().map(|child| child.totals))),
            children,
            hot: false,
        }
    }
}

#[cfg(test)]
mod tests;
