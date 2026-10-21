use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use crate::cornercase::{Client, Runner, Snapshot};
use crate::procs::{Pid, ProcessSource, Sample, cpu_percent};

#[derive(Debug, Clone)]
pub(crate) struct Reading {
    pub(super) snapshot: Snapshot,
    pub(super) sample: Sample,
    pub(super) cpu: HashMap<Pid, f64>,
}

pub(crate) type Update = Result<Reading, String>;

pub(crate) struct Sampler<R, S> {
    client: Client<R>,
    source: S,
    previous: Option<Sample>,
}

impl<R: Runner, S: ProcessSource> Sampler<R, S> {
    pub(super) fn new(client: Client<R>, source: S) -> Self {
        Self { client, source, previous: None }
    }

    pub(super) fn tick(&mut self) -> Update {
        let sample = self.source.sample().map_err(|error| format!("cannot read the processes: {error}"))?;
        let cpu = self.previous.as_ref().map(|previous| cpu_percent(previous, &sample)).unwrap_or_default();
        self.previous = Some(sample.clone());
        let snapshot = self.client.status().map_err(|error| format!("cornercase: {error}"))?;
        Ok(Reading { snapshot, sample, cpu })
    }
}

pub(crate) struct Running {
    pub(super) updates: Receiver<Update>,
    _stop: Sender<()>,
}

pub(super) fn spawn<R, S>(mut sampler: Sampler<R, S>, interval: Duration) -> Running
where
    R: Runner + Send + 'static,
    S: ProcessSource + Send + 'static,
{
    let (updates, receiver) = mpsc::channel();
    let (stop, stopped) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        loop {
            if updates.send(sampler.tick()).is_err() {
                return;
            }
            if stopped.recv_timeout(interval) != Err(RecvTimeoutError::Timeout) {
                return;
            }
        }
    });
    Running { updates: receiver, _stop: stop }
}

#[cfg(test)]
mod tests;
