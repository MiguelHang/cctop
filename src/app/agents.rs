use std::cmp::Reverse;

use super::state::RowId;
use super::view::{project_label, workspace_name};
use crate::usage::{PaneNode, Tree};

const CLAUDE: &str = "claude";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Agent {
    pub(super) pane: u64,
    pub(super) under: [RowId; 3],
    pub(super) label: String,
    pub(super) percent: Option<u16>,
}

pub(super) fn agents(tree: &Tree) -> Vec<Agent> {
    let mut agents = Vec::new();
    for project in &tree.projects {
        let project_name = project_label(project);
        for workspace in &project.workspaces {
            let path = format!("{project_name} › {}", workspace_name(workspace));
            let under = |tab: u64| [RowId::Project(project.id), RowId::Workspace(workspace.id), RowId::Tab(tab)];
            let panes = workspace.tabs.iter().flat_map(|tab| tab.panes.iter().map(move |pane| (tab.id, pane)));
            agents.extend(panes.filter(|(_, pane)| is_claude(pane)).map(|(tab, pane)| Agent {
                pane: pane.id,
                under: under(tab),
                label: format!("{path} › %{}", pane.id),
                percent: pane.context_percent,
            }));
        }
    }
    agents.sort_by_key(|agent| Reverse(agent.percent));
    agents
}

fn is_claude(pane: &PaneNode) -> bool {
    pane.agent.as_deref() == Some(CLAUDE)
}

pub(super) fn candidate(agents: &[Agent], selected: Option<RowId>) -> Option<u64> {
    selected.and_then(|id| under(agents, id)).or_else(|| agents.first().map(|agent| agent.pane))
}

pub(super) fn under(agents: &[Agent], id: RowId) -> Option<u64> {
    let below = |agent: &&Agent| match id {
        RowId::Pane(pane) | RowId::Process { pane, .. } => agent.pane == pane,
        _ => agent.under.contains(&id),
    };
    agents.iter().find(below).map(|agent| agent.pane)
}

#[cfg(test)]
mod tests;
