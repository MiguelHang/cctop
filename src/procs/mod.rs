use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::error::Result;

mod environ;
#[cfg(test)]
pub(crate) mod fake;
mod kill;
#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(test)]
mod probe;
#[cfg(any(target_os = "macos", test))]
mod procargs;

#[cfg(target_os = "linux")]
use linux::identity;
#[cfg(target_os = "macos")]
use macos::identity;

pub(crate) use kill::{Killer, Signal, Target, own_group};

pub(crate) type Pid = u32;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Process {
    pub(crate) pid: Pid,
    pub(crate) ppid: Option<Pid>,
    pub(crate) pgid: Pid,
    pub(crate) name: String,
    pub(crate) args: Vec<String>,
    pub(crate) memory: Option<u64>,
    pub(crate) cpu_time: Duration,
    pub(crate) started: u64,
    pub(crate) pane: Option<u64>,
    pub(crate) server: Option<String>,
    pub(crate) claude_config_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Identity {
    pub(crate) started: u64,
    pub(crate) pgid: Pid,
}

#[derive(Debug, Clone)]
pub(crate) struct Sample {
    pub(crate) taken: Instant,
    pub(crate) processes: Vec<Process>,
}

pub(crate) trait ProcessSource {
    fn sample(&self) -> Result<Sample>;
}

#[cfg(target_os = "linux")]
pub(crate) fn system() -> impl ProcessSource {
    linux::LinuxSource::new()
}

#[cfg(target_os = "macos")]
pub(crate) fn system() -> impl ProcessSource {
    macos::MacosSource::new()
}

#[cfg(target_os = "linux")]
pub(crate) fn machine_memory() -> Option<u64> {
    linux::machine_memory()
}

#[cfg(target_os = "macos")]
pub(crate) fn machine_memory() -> Option<u64> {
    macos::machine_memory()
}

pub(crate) fn cpus() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}

pub(crate) fn cpu_percent(prev: &Sample, next: &Sample) -> HashMap<Pid, f64> {
    let wall = next.taken.saturating_duration_since(prev.taken).as_secs_f64();
    if wall <= 0.0 {
        return HashMap::new();
    }
    let before: HashMap<Pid, &Process> = prev.processes.iter().map(|process| (process.pid, process)).collect();
    next.processes
        .iter()
        .filter_map(|process| {
            let old = before.get(&process.pid).filter(|old| old.started == process.started)?;
            let used = process.cpu_time.checked_sub(old.cpu_time)?;
            Some((process.pid, used.as_secs_f64() / wall * 100.0))
        })
        .collect()
}

#[cfg(test)]
mod tests;
