use std::collections::HashSet;

use super::view::Line;
use crate::config;
use crate::procs::Pid;
use crate::ui::{self, ViewTab};
use crate::usage::Sort;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RowId {
    Project(u64),
    Workspace(u64),
    Tab(u64),
    Pane(u64),
    Process { pane: u64, pid: Pid },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct State {
    pub(super) tab: ViewTab,
    pub(super) sort: Option<Sort>,
    pub(super) selected: Option<RowId>,
    selected_at: usize,
    pub(super) unfolded: HashSet<RowId>,
    pub(super) scroll: usize,
    pub(super) page: usize,
    pub(super) context_scroll: usize,
    pub(super) agent: Option<u64>,
}

impl State {
    pub(super) fn new(sort: Option<Sort>) -> Self {
        Self { sort, page: 1, ..Self::default() }
    }

    pub(super) fn index(&self, lines: &[Line]) -> Option<usize> {
        let id = self.selected?;
        lines.iter().position(|line| line.id == id)
    }

    pub(super) fn select(&mut self, lines: &[Line], index: usize) {
        let Some(line) = lines.get(index) else { return };
        self.selected = Some(line.id);
        self.selected_at = index;
        let page = self.page.max(1);
        if index < self.scroll {
            self.scroll = index;
        } else if index >= self.scroll + page {
            self.scroll = index + 1 - page;
        }
    }

    pub(super) fn step(&mut self, lines: &[Line], delta: isize) {
        let last = lines.len().saturating_sub(1);
        let index = match self.index(lines) {
            Some(at) => at.saturating_add_signed(delta).min(last),
            None if delta < 0 => last,
            None => 0,
        };
        self.select(lines, index);
    }

    pub(super) fn reconcile(&mut self, lines: &[Line]) {
        match self.index(lines) {
            Some(at) => self.selected_at = at,
            None if self.selected.is_some() && !lines.is_empty() => {
                self.select(lines, self.selected_at.min(lines.len() - 1));
            }
            None => self.selected = None,
        }
        self.scroll = self.scroll.min(lines.len().saturating_sub(self.page.max(1)));
    }

    pub(super) fn scroll_by(&mut self, lines: &[Line], delta: isize) {
        self.scroll = self.scroll.saturating_add_signed(delta);
        self.reconcile(lines);
    }

    pub(super) fn step_agent(&mut self, agents: &[u64], delta: isize) {
        let Some(last) = agents.len().checked_sub(1) else { return };
        let index = match agents.iter().position(|pane| Some(*pane) == self.agent) {
            Some(at) => at.saturating_add_signed(delta).min(last),
            None => 0,
        };
        self.agent = agents.get(index).copied();
    }

    pub(super) fn fold(&mut self, id: RowId, fold: bool) {
        if fold {
            self.unfolded.remove(&id);
        } else {
            self.unfolded.insert(id);
        }
    }

    pub(super) fn toggle(&mut self, id: RowId) {
        self.fold(id, self.unfolded.contains(&id));
    }

    pub(super) fn sort_by(&mut self, column: ui::Sort) {
        match column {
            ui::Sort::Name => self.sort = None,
            ui::Sort::Memory => self.sort = Some(Sort::Memory),
            ui::Sort::Cpu => self.sort = Some(Sort::Cpu),
            ui::Sort::Agent | ui::Sort::Status | ui::Sort::Context => {}
        }
    }

    pub(super) fn next_sort(&mut self) {
        self.sort = match self.sort {
            Some(Sort::Memory) => Some(Sort::Cpu),
            Some(Sort::Cpu) => None,
            None => Some(Sort::Memory),
        };
    }

    pub(super) fn next_tab(&mut self) {
        self.tab = match self.tab {
            ViewTab::Processes => ViewTab::Context,
            ViewTab::Context => ViewTab::Processes,
        };
    }
}

pub(super) fn column(sort: Option<Sort>) -> ui::Sort {
    match sort {
        Some(Sort::Memory) => ui::Sort::Memory,
        Some(Sort::Cpu) => ui::Sort::Cpu,
        None => ui::Sort::Name,
    }
}

pub(super) fn from_config(sort: config::Sort) -> Sort {
    match sort {
        config::Sort::Memory => Sort::Memory,
        config::Sort::Cpu => Sort::Cpu,
    }
}
