use std::collections::HashMap;

use crate::procs::{Pid, Process};

pub(super) struct Assignment<'a> {
    roots: HashMap<u64, Vec<&'a Process>>,
    children: HashMap<(u64, Pid), Vec<&'a Process>>,
}

impl<'a> Assignment<'a> {
    pub(super) fn new(boot: Option<&str>, processes: &'a [Process]) -> Self {
        let by_pid: HashMap<Pid, &Process> = processes.iter().map(|process| (process.pid, process)).collect();
        let mut owned: Vec<(&Process, u64)> =
            processes.iter().filter_map(|process| Some((process, pane_of(process, &by_pid, boot?)?))).collect();
        owned.sort_by_key(|(process, _)| process.pid);
        let parents = parents_in_pane(&owned);
        let mut assignment = Self { roots: HashMap::new(), children: HashMap::new() };
        for (process, pane) in owned {
            assignment.place(process, pane, parents.get(&process.pid).copied());
        }
        assignment
    }

    pub(super) fn roots(&self, pane: u64) -> Vec<&'a Process> {
        self.roots.get(&pane).cloned().unwrap_or_default()
    }

    pub(super) fn children(&self, pane: u64, pid: Pid) -> Vec<&'a Process> {
        self.children.get(&(pane, pid)).cloned().unwrap_or_default()
    }

    fn place(&mut self, process: &'a Process, pane: u64, parent: Option<Pid>) {
        match parent {
            Some(ppid) => self.children.entry((pane, ppid)).or_default().push(process),
            None => self.roots.entry(pane).or_default().push(process),
        }
    }
}

fn parents_in_pane(owned: &[(&Process, u64)]) -> HashMap<Pid, Pid> {
    let panes: HashMap<Pid, u64> = owned.iter().map(|(process, pane)| (process.pid, *pane)).collect();
    let mut parents: HashMap<Pid, Pid> = owned
        .iter()
        .filter_map(|(process, pane)| {
            process.ppid.filter(|ppid| *ppid != process.pid && panes.get(ppid) == Some(pane)).map(|p| (process.pid, p))
        })
        .collect();
    for (process, _) in owned {
        if closes_a_cycle(process.pid, &parents) {
            parents.remove(&process.pid);
        }
    }
    parents
}

fn closes_a_cycle(pid: Pid, parents: &HashMap<Pid, Pid>) -> bool {
    let mut current = pid;
    for _ in 0..parents.len() {
        match parents.get(&current) {
            Some(parent) if *parent == pid => return true,
            Some(parent) => current = *parent,
            None => return false,
        }
    }
    false
}

fn pane_of(process: &Process, by_pid: &HashMap<Pid, &Process>, boot: &str) -> Option<u64> {
    let marked = marked_ancestor(process, by_pid)?;
    marked.server.as_deref().filter(|server| *server == boot).and(marked.pane)
}

fn marked_ancestor<'a>(process: &'a Process, by_pid: &HashMap<Pid, &'a Process>) -> Option<&'a Process> {
    let mut current = process;
    for _ in 0..=by_pid.len() {
        if current.pane.is_some() || current.server.is_some() {
            return Some(current);
        }
        current = by_pid.get(&current.ppid?)?;
    }
    None
}
