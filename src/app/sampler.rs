use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use super::context::{self, Contexts, PaneContext, Source};
use crate::context::Sessions;
use crate::cornercase::{Client, Runner, Snapshot};
use crate::procs::{Pid, Process, ProcessSource, Sample, cpu_percent};

const STATUS_EVERY: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub(crate) struct Reading {
    pub(super) snapshot: Snapshot,
    pub(super) sample: Sample,
    pub(super) cpu: HashMap<Pid, f64>,
    pub(super) context: Option<PaneContext>,
}

pub(crate) type Tick = Result<Reading, String>;

#[derive(Debug)]
pub(crate) enum Update {
    Reading(Tick),
    Context(Option<PaneContext>),
}

pub(crate) struct Sampler<R, S, C = Sessions> {
    client: Client<R>,
    source: S,
    previous: Option<Sample>,
    contexts: C,
    home: Option<OsString>,
    pane: Option<u64>,
    last: Option<Reading>,
    asked: Option<Asked>,
    hidden: Option<Pid>,
}

struct Asked {
    snapshot: Snapshot,
    taken: Instant,
    panes: HashSet<u64>,
}

impl Asked {
    fn new(snapshot: Snapshot, sample: &Sample) -> Self {
        let mut panes = sample_panes(sample, snapshot.boot.as_deref());
        let projects = snapshot.projects.iter();
        let tabs = projects.flat_map(|project| &project.workspaces).flat_map(|workspace| &workspace.tabs);
        panes.extend(tabs.flat_map(|tab| &tab.panes).map(|pane| pane.id));
        Self { snapshot, taken: sample.taken, panes }
    }

    fn covers(&self, sample: &Sample) -> bool {
        sample.taken.saturating_duration_since(self.taken) < STATUS_EVERY
            && sample_panes(sample, self.snapshot.boot.as_deref()).is_subset(&self.panes)
    }
}

fn sample_panes(sample: &Sample, boot: Option<&str>) -> HashSet<u64> {
    let ours = |process: &&Process| boot.is_some() && process.server.as_deref() == boot;
    sample.processes.iter().filter(ours).filter_map(|process| process.pane).collect()
}

impl<R, S> Sampler<R, S> {
    pub(super) fn new(client: Client<R>, source: S) -> Self {
        let contexts = Sessions::default();
        Self { client, source, previous: None, contexts, home: None, pane: None, last: None, asked: None, hidden: None }
    }
}

impl<R, S, C> Sampler<R, S, C> {
    pub(super) fn with_contexts<D>(self, contexts: D, home: Option<OsString>) -> Sampler<R, S, D> {
        let Self { client, source, previous, pane, last, asked, hidden, .. } = self;
        Sampler { client, source, previous, contexts, home, pane, last, asked, hidden }
    }

    pub(super) fn hiding_group(self, group: Option<Pid>) -> Self {
        Self { hidden: group, ..self }
    }
}

impl<R: Runner, S: ProcessSource, C: Contexts> Sampler<R, S, C> {
    pub(super) fn tick(&mut self) -> Tick {
        let mut sample = self.source.sample().map_err(|error| format!("cannot read the processes: {error}"))?;
        if let Some(group) = self.hidden {
            sample.processes.retain(|process| process.pgid != group);
        }
        let cpu = self.previous.as_ref().map(|previous| cpu_percent(previous, &sample)).unwrap_or_default();
        self.previous = Some(sample.clone());
        let snapshot = self.status(&sample)?;
        let mut reading = Reading { snapshot, sample, cpu, context: None };
        reading.context = self.context(&reading);
        self.last = Some(reading.clone());
        Ok(reading)
    }

    fn status(&mut self, sample: &Sample) -> Result<Snapshot, String> {
        if let Some(asked) = self.asked.as_ref().filter(|asked| asked.covers(sample)) {
            return Ok(asked.snapshot.clone());
        }
        self.asked = None;
        let snapshot = self.client.status().map_err(|error| format!("cornercase: {error}"))?;
        self.asked = Some(Asked::new(snapshot.clone(), sample));
        Ok(snapshot)
    }

    pub(super) fn select(&mut self, pane: Option<u64>) -> Option<Update> {
        self.pane = pane;
        let reading = self.last.take()?;
        let context = self.context(&reading);
        self.last = Some(reading);
        Some(Update::Context(context))
    }

    fn context(&mut self, reading: &Reading) -> Option<PaneContext> {
        let source = Source { snapshot: &reading.snapshot, sample: &reading.sample, home: self.home.as_ref() };
        context::read(&mut self.contexts, &source, self.pane)
    }
}

pub(crate) struct Running {
    pub(super) updates: Receiver<Update>,
    pub(super) panes: Sender<Option<u64>>,
}

pub(super) fn spawn<R, S, C>(mut sampler: Sampler<R, S, C>, interval: Duration) -> Running
where
    R: Runner + Send + 'static,
    S: ProcessSource + Send + 'static,
    C: Contexts + Send + 'static,
{
    let (updates, receiver) = mpsc::channel();
    let (panes, selected) = mpsc::channel();
    std::thread::spawn(move || {
        while updates.send(Update::Reading(sampler.tick())).is_ok() && wait(&mut sampler, &selected, &updates, interval)
        {
        }
    });
    Running { updates: receiver, panes }
}

fn wait<R, S, C>(
    sampler: &mut Sampler<R, S, C>,
    selected: &Receiver<Option<u64>>,
    updates: &Sender<Update>,
    interval: Duration,
) -> bool
where
    R: Runner,
    S: ProcessSource,
    C: Contexts,
{
    let deadline = Instant::now() + interval;
    loop {
        let pane = match selected.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(pane) => selected.try_iter().last().unwrap_or(pane),
            Err(RecvTimeoutError::Timeout) => return true,
            Err(RecvTimeoutError::Disconnected) => return false,
        };
        if let Some(update) = sampler.select(pane)
            && updates.send(update).is_err()
        {
            return false;
        }
    }
}

#[cfg(test)]
mod tests;
