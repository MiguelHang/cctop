use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::environ::parse_environ;
use super::{Identity, Pid, Process, ProcessSource, Sample};
use crate::error::Result;

const DEFAULT_TICKS_PER_SECOND: u64 = 100;
const NANOS_PER_SECOND: u64 = 1_000_000_000;

pub(crate) struct LinuxSource {
    root: PathBuf,
    uid: u32,
    ticks_per_second: u64,
}

impl LinuxSource {
    pub(crate) fn new() -> Self {
        Self::at(PathBuf::from("/proc"), current_uid(), ticks_per_second())
    }

    fn at(root: PathBuf, uid: u32, ticks_per_second: u64) -> Self {
        Self { root, uid, ticks_per_second: ticks_per_second.max(1) }
    }

    fn read(&self, pid: Pid) -> Option<Process> {
        let dir = self.root.join(pid.to_string());
        let status = parse_status(&String::from_utf8_lossy(&fs::read(dir.join("status")).ok()?));
        if status.uid != Some(self.uid) {
            return None;
        }
        let stat = parse_stat(&String::from_utf8_lossy(&fs::read(dir.join("stat")).ok()?))?;
        let args = fs::read(dir.join("cmdline")).map(|bytes| parse_cmdline(&bytes)).unwrap_or_default();
        let env = fs::read(dir.join("environ")).map(|bytes| parse_environ(&bytes)).unwrap_or_default();
        Some(Process {
            pid,
            ppid: status.ppid,
            pgid: stat.pgid,
            name: stat.name,
            args,
            memory: status.memory,
            cpu_time: self.ticks_to_duration(stat.cpu_ticks),
            started: stat.started,
            pane: env.pane,
            server: env.server,
            claude_config_dir: env.claude_config_dir,
        })
    }

    fn ticks_to_duration(&self, ticks: u64) -> Duration {
        let whole = ticks / self.ticks_per_second;
        let rest = (ticks % self.ticks_per_second) * NANOS_PER_SECOND / self.ticks_per_second;
        Duration::from_secs(whole) + Duration::from_nanos(rest)
    }
}

impl ProcessSource for LinuxSource {
    fn sample(&self) -> Result<Sample> {
        let taken = Instant::now();
        let processes = fs::read_dir(&self.root)?
            .filter_map(|entry| entry.ok()?.file_name().to_str()?.parse::<Pid>().ok())
            .filter_map(|pid| self.read(pid))
            .collect();
        Ok(Sample { taken, processes })
    }
}

pub(crate) fn identity(pid: Pid) -> Option<Identity> {
    let stat = parse_stat(&String::from_utf8_lossy(&fs::read(format!("/proc/{pid}/stat")).ok()?))?;
    Some(Identity { started: stat.started, pgid: stat.pgid })
}

fn current_uid() -> u32 {
    unsafe { libc::getuid() }
}

fn ticks_per_second() -> u64 {
    let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    u64::try_from(ticks).ok().filter(|&ticks| ticks > 0).unwrap_or(DEFAULT_TICKS_PER_SECOND)
}

#[derive(Debug, Default, PartialEq)]
struct Status {
    ppid: Option<Pid>,
    uid: Option<u32>,
    memory: Option<u64>,
}

fn parse_status(text: &str) -> Status {
    let mut rss_anon: Option<u64> = None;
    let mut swap = None;
    let mut status = Status::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let first = value.split_whitespace().next();
        match key {
            "PPid" => status.ppid = first.and_then(|value| value.parse().ok()),
            "Uid" => status.uid = first.and_then(|value| value.parse().ok()),
            "RssAnon" => rss_anon = first.and_then(|value| value.parse().ok()),
            "VmSwap" => swap = first.and_then(|value| value.parse().ok()),
            _ => {}
        }
    }
    status.memory = rss_anon.map(|kib| kib.saturating_add(swap.unwrap_or(0)).saturating_mul(1024));
    status
}

#[derive(Debug, PartialEq)]
struct Stat {
    name: String,
    pgid: Pid,
    cpu_ticks: u64,
    started: u64,
}

fn parse_stat(text: &str) -> Option<Stat> {
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    let name = text.get(open + 1..close)?.to_owned();
    let fields: Vec<&str> = text.get(close + 1..)?.split_whitespace().collect();
    let number = |index: usize| fields.get(index)?.parse::<u64>().ok();
    Some(Stat {
        name,
        pgid: fields.get(2)?.parse().ok()?,
        cpu_ticks: number(11)?.checked_add(number(12)?)?,
        started: number(19)?,
    })
}

fn parse_cmdline(bytes: &[u8]) -> Vec<String> {
    let bytes = bytes.strip_suffix(&[0]).unwrap_or(bytes);
    if bytes.is_empty() {
        return Vec::new();
    }
    bytes.split(|&byte| byte == 0).map(|arg| String::from_utf8_lossy(arg).into_owned()).collect()
}

#[cfg(test)]
mod tests;
