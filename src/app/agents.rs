use std::cmp::Reverse;
use std::collections::HashMap;

use super::state::RowId;
use super::view::{project_label, workspace_name};
use crate::usage::{PaneNode, TabNode, Tree};

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
            let names = [project_name.as_str(), &workspace_name(workspace)].map(str::to_owned);
            let under = |tab: u64| [RowId::Project(project.id), RowId::Workspace(workspace.id), RowId::Tab(tab)];
            let panes = workspace.tabs.iter().flat_map(|tab| tab.panes.iter().map(move |pane| (tab, pane)));
            agents.extend(panes.filter(|(_, pane)| is_claude(pane)).map(|(tab, pane)| Agent {
                pane: pane.id,
                under: under(tab.id),
                label: place(&names, tab, pane),
                percent: pane.context_percent,
            }));
        }
    }
    break_ties(&mut agents);
    agents.sort_by_key(|agent| Reverse(agent.percent));
    agents
}

fn place(names: &[String; 2], tab: &TabNode, pane: &PaneNode) -> String {
    let [project, workspace] = names;
    let generic = |name: &str| {
        name == project || name == workspace || [pane.agent.as_deref(), pane.command.as_deref()].contains(&Some(name))
    };
    match tab.name.as_deref().filter(|name| !name.is_empty() && !generic(name)) {
        Some(name) => format!("{project} › {workspace} › {name}"),
        None => format!("{project} › {workspace}"),
    }
}

fn break_ties(agents: &mut [Agent]) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for agent in agents.iter() {
        *seen.entry(agent.label.clone()).or_default() += 1;
    }
    for agent in agents.iter_mut() {
        if seen.get(&agent.label).is_some_and(|count| *count > 1) {
            agent.label = format!("{} %{}", agent.label, agent.pane);
        }
    }
}

pub(super) fn title(agent: &Agent, pane: &PaneNode) -> String {
    let who = [pane.agent.as_deref(), pane.model.as_deref()].into_iter().flatten().collect::<Vec<_>>().join(" ");
    [Some(agent.label.as_str()), Some(who.as_str()).filter(|who| !who.is_empty()), pane.status.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
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
