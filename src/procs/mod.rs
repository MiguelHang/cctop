use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::error::Result;

#[cfg(test)]
pub(crate) mod fake;
#[cfg(target_os = "linux")]
pub(crate) mod linux;

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

#[derive(Debug, Clone)]
pub(crate) struct Sample {
    pub(crate) taken: Instant,
    pub(crate) processes: Vec<Process>,
}

pub(crate) trait ProcessSource {
    fn sample(&self) -> Result<Sample>;
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
