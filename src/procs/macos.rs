use std::ffi::{CStr, c_char, c_int, c_void};
use std::io;
use std::mem::{MaybeUninit, size_of};
use std::ptr;
use std::time::{Duration, Instant};

use super::procargs::{ProcArgs, parse_procargs};
use super::{Identity, Pid, Process, ProcessSource, Sample};
use crate::error::Result;

const SPARE_PIDS: usize = 64;
const DEFAULT_ARG_MAX: usize = 1024 * 1024;
const MICROS_PER_SECOND: u64 = 1_000_000;

pub(crate) struct MacosSource {
    uid: u32,
    timebase: Timebase,
    arg_max: usize,
}

impl MacosSource {
    pub(crate) fn new() -> Self {
        Self { uid: current_uid(), timebase: Timebase::current(), arg_max: arg_max() }
    }

    fn read(&self, pid: c_int, buffer: &mut Vec<u8>) -> Option<Process> {
        let bsd = pid_info::<libc::proc_bsdinfo>(pid, libc::PROC_PIDTBSDINFO)?;
        if bsd.pbi_ruid != self.uid {
            return None;
        }
        let cpu_ticks = pid_info::<libc::proc_taskinfo>(pid, libc::PROC_PIDTASKINFO)
            .map_or(0, |task| task.pti_total_user.saturating_add(task.pti_total_system));
        let ProcArgs { args, env } = self.procargs(pid, buffer).map(parse_procargs).unwrap_or_default();
        Some(Process {
            pid: bsd.pbi_pid,
            ppid: Some(bsd.pbi_ppid),
            pgid: bsd.pbi_pgid,
            name: name(&bsd),
            args,
            memory: footprint(pid),
            cpu_time: self.timebase.to_duration(cpu_ticks),
            started: started(&bsd),
            pane: env.pane,
            server: env.server,
            claude_config_dir: env.claude_config_dir,
        })
    }

    fn procargs<'a>(&self, pid: c_int, buffer: &'a mut Vec<u8>) -> Option<&'a [u8]> {
        let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
        buffer.resize(self.arg_max, 0);
        let mut length = buffer.len();
        let status = unsafe {
            libc::sysctl(mib.as_mut_ptr(), 3, buffer.as_mut_ptr().cast(), &raw mut length, ptr::null_mut(), 0)
        };
        if status != 0 {
            return None;
        }
        buffer.get(..length)
    }
}

impl ProcessSource for MacosSource {
    fn sample(&self) -> Result<Sample> {
        let taken = Instant::now();
        let mut buffer = Vec::new();
        let processes = list_pids()?.into_iter().filter_map(|pid| self.read(pid, &mut buffer)).collect();
        Ok(Sample { taken, processes })
    }
}

fn list_pids() -> Result<Vec<c_int>> {
    let count = unsafe { libc::proc_listallpids(ptr::null_mut(), 0) };
    let count = usize::try_from(count).map_err(|_| io::Error::last_os_error())?;
    let mut pids: Vec<c_int> = vec![0; count + SPARE_PIDS];
    let bytes =
        c_int::try_from(pids.len() * size_of::<c_int>()).map_err(|_| io::Error::from(io::ErrorKind::OutOfMemory))?;
    let filled = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), bytes) };
    let filled = usize::try_from(filled).map_err(|_| io::Error::last_os_error())?;
    pids.truncate(filled.min(pids.len()));
    pids.retain(|&pid| pid > 0);
    Ok(pids)
}

#[expect(
    clippy::missing_safety_doc,
    reason = "the repo has no doc comments; the safety contract is in design/processes.md"
)]
unsafe trait PlainData: Copy {}

unsafe impl PlainData for libc::proc_bsdinfo {}
unsafe impl PlainData for libc::proc_taskinfo {}

pub(crate) fn identity(pid: Pid) -> Option<Identity> {
    let bsd = pid_info::<libc::proc_bsdinfo>(c_int::try_from(pid).ok()?, libc::PROC_PIDTBSDINFO)?;
    Some(Identity { started: started(&bsd), pgid: bsd.pbi_pgid })
}

fn started(bsd: &libc::proc_bsdinfo) -> u64 {
    bsd.pbi_start_tvsec.saturating_mul(MICROS_PER_SECOND).saturating_add(bsd.pbi_start_tvusec)
}

fn pid_info<T: PlainData>(pid: c_int, flavor: c_int) -> Option<T> {
    let mut info = MaybeUninit::<T>::zeroed();
    let size = c_int::try_from(size_of::<T>()).ok()?;
    let written = unsafe { libc::proc_pidinfo(pid, flavor, 0, info.as_mut_ptr().cast::<c_void>(), size) };
    (written == size).then(|| unsafe { info.assume_init() })
}

fn footprint(pid: c_int) -> Option<u64> {
    let mut info = MaybeUninit::<libc::rusage_info_v4>::zeroed();
    let status = unsafe { libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V4, info.as_mut_ptr().cast()) };
    (status == 0).then(|| unsafe { info.assume_init() }.ri_phys_footprint)
}

fn name(bsd: &libc::proc_bsdinfo) -> String {
    let long = c_string(&bsd.pbi_name);
    if long.is_empty() { c_string(&bsd.pbi_comm) } else { long }
}

fn c_string(chars: &[c_char]) -> String {
    let bytes: Vec<u8> = chars.iter().map(|&byte| byte.to_ne_bytes()[0]).collect();
    CStr::from_bytes_until_nul(&bytes)
        .map_or_else(|_| String::from_utf8_lossy(&bytes).into_owned(), |text| text.to_string_lossy().into_owned())
}

fn current_uid() -> u32 {
    unsafe { libc::getuid() }
}

pub(crate) fn machine_memory() -> Option<u64> {
    let mut value: u64 = 0;
    let mut length = size_of::<u64>();
    let status = unsafe {
        libc::sysctlbyname(c"hw.memsize".as_ptr(), (&raw mut value).cast(), &raw mut length, ptr::null_mut(), 0)
    };
    (status == 0 && value > 0).then_some(value)
}

fn arg_max() -> usize {
    let mut mib = [libc::CTL_KERN, libc::KERN_ARGMAX];
    let mut value: c_int = 0;
    let mut length = size_of::<c_int>();
    let status =
        unsafe { libc::sysctl(mib.as_mut_ptr(), 2, (&raw mut value).cast(), &raw mut length, ptr::null_mut(), 0) };
    usize::try_from(value).ok().filter(|&value| status == 0 && value > 0).unwrap_or(DEFAULT_ARG_MAX)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Timebase {
    numer: u32,
    denom: u32,
}

impl Timebase {
    #[expect(deprecated, reason = "libc points to the mach2 crate, which would be a second crate for one call")]
    fn current() -> Self {
        let mut info = libc::mach_timebase_info { numer: 0, denom: 0 };
        let status = unsafe { libc::mach_timebase_info(&raw mut info) };
        Self::new(status, info.numer, info.denom)
    }

    fn new(status: c_int, numer: u32, denom: u32) -> Self {
        if status == 0 && numer > 0 && denom > 0 { Self { numer, denom } } else { Self { numer: 1, denom: 1 } }
    }

    fn to_duration(self, ticks: u64) -> Duration {
        let nanos = u128::from(ticks) * u128::from(self.numer) / u128::from(self.denom);
        Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
    }
}

#[cfg(test)]
mod tests;
