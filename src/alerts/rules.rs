use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::usage::{PaneNode, Tree, WorkspaceNode};

const REARM_PERCENT: u32 = 80;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Thresholds {
    pub(crate) memory: u64,
    pub(crate) cpu: f64,
    pub(crate) cpu_sustain: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Memory,
    Cpu,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Alert<'t> {
    pub(crate) kind: Kind,
    pub(crate) workspace: &'t WorkspaceNode,
    pub(crate) pane: &'t PaneNode,
}

#[derive(Debug, Default)]
struct PaneState {
    memory_fired: bool,
    cpu_fired: bool,
    cpu_since: Option<Instant>,
}

pub(crate) struct Rules {
    thresholds: Thresholds,
    panes: HashMap<u64, PaneState>,
}

impl From<&Config> for Thresholds {
    fn from(config: &Config) -> Self {
        Self { memory: config.memory_threshold, cpu: config.cpu_threshold, cpu_sustain: config.cpu_sustain }
    }
}

impl Rules {
    pub(crate) fn new(thresholds: Thresholds) -> Self {
        Self { thresholds, panes: HashMap::new() }
    }

    pub(crate) fn check<'t>(&mut self, tree: &'t Tree, now: Instant) -> Vec<Alert<'t>> {
        let mut seen = HashMap::new();
        let mut alerts = Vec::new();
        for (workspace, pane) in panes(tree) {
            let mut state = self.panes.remove(&pane.id).unwrap_or_default();
            if self.thresholds.memory(&mut state, pane.totals.memory) {
                alerts.push(Alert { kind: Kind::Memory, workspace, pane });
            }
            if self.thresholds.cpu(&mut state, pane.totals.cpu, now) {
                alerts.push(Alert { kind: Kind::Cpu, workspace, pane });
            }
            seen.insert(pane.id, state);
        }
        self.panes = seen;
        alerts
    }
}

impl Thresholds {
    fn memory(self, state: &mut PaneState, memory: Option<u64>) -> bool {
        let Some(memory) = memory else { return false };
        if u128::from(memory) * 100 < u128::from(self.memory) * u128::from(REARM_PERCENT) {
            state.memory_fired = false;
        }
        let fires = memory > self.memory && !state.memory_fired;
        state.memory_fired |= fires;
        fires
    }

    fn cpu(self, state: &mut PaneState, cpu: Option<f64>, now: Instant) -> bool {
        if !cpu.is_some_and(|cpu| cpu > self.cpu) {
            state.cpu_since = None;
            state.cpu_fired &= cpu.is_none_or(|cpu| cpu * 100.0 >= self.cpu * f64::from(REARM_PERCENT));
            return false;
        }
        let since = *state.cpu_since.get_or_insert(now);
        let fires = !state.cpu_fired && now.saturating_duration_since(since) >= self.cpu_sustain;
        state.cpu_fired |= fires;
        fires
    }
}

fn panes(tree: &Tree) -> impl Iterator<Item = (&WorkspaceNode, &PaneNode)> {
    tree.projects
        .iter()
        .flat_map(|project| &project.workspaces)
        .flat_map(|workspace| workspace.tabs.iter().flat_map(|tab| &tab.panes).map(move |pane| (workspace, pane)))
}

#[cfg(test)]
mod tests;
