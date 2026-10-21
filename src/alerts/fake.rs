use crate::usage::{PaneNode, ProcessNode, ProjectNode, TabNode, Totals, Tree, WorkspaceNode};

pub(crate) const GB: u64 = 1 << 30;

pub(crate) fn process(name: &str, args: &[&str], memory: Option<u64>, cpu: Option<f64>) -> ProcessNode {
    ProcessNode {
        pid: 1,
        name: name.to_owned(),
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        memory,
        cpu,
        children: Vec::new(),
        totals: Totals { memory, memory_partial: false, cpu },
        hot: false,
    }
}

pub(crate) fn pane(id: u64, agent: Option<&str>, processes: Vec<ProcessNode>) -> PaneNode {
    let totals = processes.iter().fold(Totals::default(), |sum, process| Totals {
        memory: sum.memory.into_iter().chain(process.totals.memory).reduce(|a, b| a + b),
        memory_partial: false,
        cpu: sum.cpu.into_iter().chain(process.totals.cpu).reduce(|a, b| a + b),
    });
    PaneNode {
        id,
        agent: agent.map(str::to_owned),
        status: None,
        model: None,
        context_percent: None,
        subagents: None,
        parent: None,
        command: None,
        path: None,
        processes,
        totals,
    }
}

pub(crate) fn workspace(name: Option<&str>, branch: Option<&str>, panes: Vec<PaneNode>) -> WorkspaceNode {
    let tab = TabNode { id: 1, name: None, panes, totals: Totals::default() };
    WorkspaceNode {
        id: 7,
        name: name.map(str::to_owned),
        branch: branch.map(str::to_owned),
        path: None,
        worktree: false,
        tabs: vec![tab],
        totals: Totals::default(),
    }
}

pub(crate) fn tree(panes: Vec<PaneNode>) -> Tree {
    let workspace = workspace(Some("feat/x"), None, panes);
    let project = ProjectNode { id: 1, name: None, path: None, workspaces: vec![workspace], totals: Totals::default() };
    Tree { projects: vec![project], totals: Totals::default() }
}
