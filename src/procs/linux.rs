use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{self, DirEntry, File};
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::environ::{Env, parse_environ};
use super::{Identity, Pid, Process, ProcessSource, Sample};
use crate::error::Result;

const DEFAULT_TICKS_PER_SECOND: u64 = 100;
const DEFAULT_PAGE_SIZE: u64 = 4096;
const NANOS_PER_SECOND: u64 = 1_000_000_000;
const READ_CAPACITY: usize = 4096;

pub(crate) struct LinuxSource {
    root: PathBuf,
    uid: u32,
    ticks_per_second: u64,
    page_size: u64,
    launches: RefCell<HashMap<Pid, Launch>>,
}

struct Launch {
    started: u64,
    name: String,
    args: Vec<String>,
    env: Env,
    settled: bool,
}

impl LinuxSource {
    pub(crate) fn new() -> Self {
        Self::at(PathBuf::from("/proc"), current_uid(), ticks_per_second())
    }

    fn at(root: PathBuf, uid: u32, ticks_per_second: u64) -> Self {
        let ticks_per_second = ticks_per_second.max(1);
        Self { root, uid, ticks_per_second, page_size: page_size(), launches: RefCell::default() }
    }

    fn owned_pid(&self, entry: &DirEntry) -> Option<Pid> {
        let pid = entry.file_name().to_str()?.parse::<Pid>().ok()?;
        (entry.metadata().ok()?.uid() == self.uid).then_some(pid)
    }

    fn swapped(&self) -> bool {
        read_file(&self.root.join("meminfo")).is_none_or(|bytes| swapped(&String::from_utf8_lossy(&bytes)))
    }

    fn read(&self, pid: Pid, known: &mut HashMap<Pid, Launch>, swapped: bool) -> Option<(Process, Launch)> {
        let dir = self.root.join(pid.to_string());
        let stat = parse_stat(&String::from_utf8_lossy(&read_file(&dir.join("stat"))?))?;
        let memory = if swapped {
            parse_status(&String::from_utf8_lossy(&read_file(&dir.join("status"))?))
        } else {
            parse_statm(&read_file(&dir.join("statm"))?, self.page_size)
        };
        let launch = known_launch(known.remove(&pid), &dir, &stat);
        let process = Process {
            pid,
            ppid: Some(stat.ppid),
            pgid: stat.pgid,
            name: stat.name,
            args: launch.args.clone(),
            memory,
            cpu_time: self.ticks_to_duration(stat.cpu_ticks),
            started: stat.started,
            pane: launch.env.pane,
            server: launch.env.server.clone(),
            claude_config_dir: launch.env.claude_config_dir.clone(),
            action_of: launch.env.action_of.clone(),
        };
        Some((process, launch))
    }

    fn ticks_to_duration(&self, ticks: u64) -> Duration {
        let whole = ticks / self.ticks_per_second;
        let rest = (ticks % self.ticks_per_second) * NANOS_PER_SECOND / self.ticks_per_second;
        Duration::from_secs(whole) + Duration::from_nanos(rest)
    }
}

fn known_launch(known: Option<Launch>, dir: &Path, stat: &Stat) -> Launch {
    match known.filter(|launch| launch.started == stat.started && launch.name == stat.name) {
        Some(launch) if launch.settled => launch,
        Some(_) => Launch { settled: true, ..launch(dir, stat) },
        None => launch(dir, stat),
    }
}

fn launch(dir: &Path, stat: &Stat) -> Launch {
    Launch {
        started: stat.started,
        name: stat.name.clone(),
        args: read_file(&dir.join("cmdline")).map(|bytes| parse_cmdline(&bytes)).unwrap_or_default(),
        env: read_file(&dir.join("environ")).map(|bytes| parse_environ(&bytes)).unwrap_or_default(),
        settled: false,
    }
}

impl ProcessSource for LinuxSource {
    fn sample(&self) -> Result<Sample> {
        let taken = Instant::now();
        let swapped = self.swapped();
        let mut known = self.launches.take();
        let mut launches = HashMap::with_capacity(known.len());
        let processes = fs::read_dir(&self.root)?
            .filter_map(|entry| self.owned_pid(&entry.ok()?))
            .filter_map(|pid| self.read(pid, &mut known, swapped))
            .map(|(process, launch)| {
                launches.insert(process.pid, launch);
                process
            })
            .collect();
        self.launches.replace(launches);
        Ok(Sample { taken, processes })
    }
}

fn read_file(path: &Path) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(READ_CAPACITY);
    File::open(path).ok()?.read_to_end(&mut bytes).ok()?;
    Some(bytes)
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

fn page_size() -> u64 {
    let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    u64::try_from(size).ok().filter(|&size| size > 0).unwrap_or(DEFAULT_PAGE_SIZE)
}

pub(crate) fn machine_memory() -> Option<u64> {
    mem_total(&String::from_utf8_lossy(&read_file(Path::new("/proc/meminfo"))?))
}

fn meminfo_kib(meminfo: &str, key: &str) -> Option<u64> {
    let line = meminfo.lines().find_map(|line| line.strip_prefix(key)?.strip_prefix(':'))?;
    line.split_whitespace().next()?.parse::<u64>().ok()
}

fn mem_total(meminfo: &str) -> Option<u64> {
    meminfo_kib(meminfo, "MemTotal").filter(|&kib| kib > 0).map(|kib| kib.saturating_mul(1024))
}

fn swapped(meminfo: &str) -> bool {
    match (meminfo_kib(meminfo, "SwapTotal"), meminfo_kib(meminfo, "SwapFree")) {
        (Some(total), Some(free)) => total > free,
        _ => true,
    }
}

fn parse_status(text: &str) -> Option<u64> {
    let mut rss_anon: Option<u64> = None;
    let mut swap = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let first = value.split_whitespace().next().and_then(|value| value.parse().ok());
        match key {
            "RssAnon" => rss_anon = first,
            "VmSwap" => swap = first,
            _ => {}
        }
    }
    rss_anon.map(|kib| kib.saturating_add(swap.unwrap_or(0)).saturating_mul(1024))
}

fn parse_statm(bytes: &[u8], page_size: u64) -> Option<u64> {
    let mut pages = std::str::from_utf8(bytes).ok()?.split_whitespace().map(|field| field.parse::<u64>().ok());
    let (size, resident, shared) = (pages.next()??, pages.next()??, pages.next()??);
    (size > 0).then(|| resident.saturating_sub(shared).saturating_mul(page_size))
}

#[derive(Debug, PartialEq)]
struct Stat {
    name: String,
    ppid: Pid,
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
        ppid: fields.get(1)?.parse().ok()?,
        pgid: fields.get(2)?.parse().ok()?,
        cpu_ticks: number(11)?.checked_add(number(12)?)?,
        started: number(19)?,
    })
}

fn parse_cmdline(bytes: &[u8]) -> Vec<String> {
    let end = bytes.iter().rposition(|&byte| byte != 0).map_or(0, |last| last + 1);
    let bytes = &bytes[..end];
    if bytes.is_empty() {
        return Vec::new();
    }
    bytes.split(|&byte| byte == 0).map(|arg| String::from_utf8_lossy(arg).into_owned()).collect()
}

#[cfg(test)]
mod tests;
