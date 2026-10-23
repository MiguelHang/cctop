use std::collections::HashMap;
use std::io::Write;
use std::time::{Duration, Instant};

use super::message::message;
use super::rules::{Rules, Thresholds};
use crate::cornercase::{Client, Runner};
use crate::error::{Error, Result};
use crate::procs::{ProcessSource, Sample, cpu_percent};
use crate::usage;

const PLUGIN: &str = "cctop";

pub(crate) trait Clock {
    fn now(&self) -> Instant;
    fn sleep(&self, duration: Duration);
}

pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

pub(crate) struct Watch<R, S, W> {
    client: Client<R>,
    source: S,
    log: W,
    rules: Rules,
    previous: Option<Sample>,
}

impl<R: Runner, S: ProcessSource, W: Write> Watch<R, S, W> {
    pub(crate) fn new(client: Client<R>, source: S, log: W, thresholds: Thresholds) -> Self {
        Self { client, source, log, rules: Rules::new(thresholds), previous: None }
    }

    pub(crate) fn run(&mut self, clock: &impl Clock, interval: Duration, mut more: impl FnMut() -> bool) {
        let mut next = clock.now();
        while more() {
            if let Err(error) = self.tick() {
                report(&mut self.log, &error);
            }
            clock.sleep(wait(&mut next, clock.now(), interval));
        }
    }

    fn tick(&mut self) -> Result<()> {
        let mut sample = self.source.sample()?;
        sample.processes.retain(|process| process.action_of.as_deref() != Some(PLUGIN));
        let cpu = self.previous.as_ref().map_or_else(HashMap::new, |previous| cpu_percent(previous, &sample));
        let status = self.client.status();
        let sample = self.previous.insert(sample);
        let tree = usage::build(&status?, sample, &cpu, None);
        for alert in self.rules.check(&tree, sample.taken) {
            if let Err(error) = self.client.notify(&message(&alert)) {
                report(&mut self.log, &error);
            }
        }
        Ok(())
    }
}

fn wait(next: &mut Instant, now: Instant, interval: Duration) -> Duration {
    let Some(due) = next.checked_add(interval) else {
        *next = now;
        return interval;
    };
    *next = due.max(now);
    *next - now
}

fn report(log: &mut impl Write, error: &Error) {
    let _ = writeln!(log, "cctop watch: {error}");
}

#[cfg(test)]
mod tests;
