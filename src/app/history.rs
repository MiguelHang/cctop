use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use super::state::RowId;
use crate::usage::{PaneNode, ProcessNode, Tree};

const ROW_SPAN: Duration = Duration::from_secs(60);
const TOTAL_SPAN: Duration = Duration::from_secs(3600);
const MAX_POINTS: usize = 3600;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct History {
    row_points: usize,
    total_points: usize,
    rows: HashMap<RowId, VecDeque<Option<f64>>>,
    cpu: VecDeque<f64>,
    memory: VecDeque<u64>,
}

impl History {
    pub(super) fn new(interval: Duration) -> Self {
        Self { row_points: points(ROW_SPAN, interval), total_points: points(TOTAL_SPAN, interval), ..Self::default() }
    }

    pub(super) fn record(&mut self, tree: &Tree) {
        let mut rows = HashMap::with_capacity(self.rows.len());
        for (id, cpu) in rows_of(tree) {
            let mut ring = self.rows.remove(&id).unwrap_or_default();
            push(&mut ring, cpu, self.row_points);
            rows.insert(id, ring);
        }
        self.rows = rows;
        if let Some(cpu) = tree.totals.cpu {
            push(&mut self.cpu, cpu, self.total_points);
        }
        if let Some(memory) = tree.totals.memory {
            push(&mut self.memory, memory, self.total_points);
        }
    }

    pub(super) fn row(&self, id: RowId) -> Vec<Option<f64>> {
        self.rows.get(&id).map(|ring| ring.iter().copied().collect()).unwrap_or_default()
    }

    pub(super) fn cpu_share(&self, cpus: f64) -> Vec<f64> {
        self.cpu.iter().map(|cpu| cpu / cpus).collect()
    }

    pub(super) fn memory(&self) -> Vec<u64> {
        self.memory.iter().copied().collect()
    }

    pub(super) fn row_points(&self) -> usize {
        self.row_points
    }

    pub(super) fn total_points(&self) -> usize {
        self.total_points
    }

    #[cfg(test)]
    pub(super) fn rows(&self) -> usize {
        self.rows.len()
    }
}

fn points(span: Duration, interval: Duration) -> usize {
    let points = span.as_millis().div_ceil(interval.as_millis().max(1));
    usize::try_from(points).unwrap_or(MAX_POINTS).clamp(1, MAX_POINTS)
}

fn push<T>(ring: &mut VecDeque<T>, value: T, capacity: usize) {
    while ring.len() >= capacity {
        ring.pop_front();
    }
    ring.push_back(value);
}

fn rows_of(tree: &Tree) -> Vec<(RowId, Option<f64>)> {
    let mut rows = Vec::new();
    for project in &tree.projects {
        rows.push((RowId::Project(project.id), project.totals.cpu));
        for workspace in &project.workspaces {
            rows.push((RowId::Workspace(workspace.id), workspace.totals.cpu));
            for tab in &workspace.tabs {
                rows.push((RowId::Tab(tab.id), tab.totals.cpu));
                tab.panes.iter().for_each(|pane| pane_rows(pane, &mut rows));
            }
        }
    }
    rows
}

fn pane_rows(pane: &PaneNode, rows: &mut Vec<(RowId, Option<f64>)>) {
    rows.push((RowId::Pane(pane.id), pane.totals.cpu));
    let mut pending: Vec<&ProcessNode> = pane.processes.iter().collect();
    while let Some(process) = pending.pop() {
        rows.push((RowId::Process { pane: pane.id, pid: process.pid }, process.totals.cpu));
        pending.extend(&process.children);
    }
}

#[cfg(test)]
mod tests;
