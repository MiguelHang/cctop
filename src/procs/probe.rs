use std::fs;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use super::{Pid, Process, ProcessSource, cpu_percent};
use crate::test_util::{TempDir, wait_until};

const MIB: u64 = 1024 * 1024;
const MEMORY_PROBE_VARIABLE: &str = "CCTOP_MEMORY_PROBE";
const SPIN: Duration = Duration::from_millis(200);

pub(crate) fn own_process(source: &impl ProcessSource, pid: Pid) -> Option<Process> {
    source.sample().expect("sample the processes").processes.into_iter().find(|process| process.pid == pid)
}

pub(crate) fn lists_the_test_process(source: &impl ProcessSource) {
    let process = own_process(source, std::process::id()).expect("own process listed");
    assert_eq!(process.ppid, Some(std::os::unix::process::parent_id()));
    assert!(process.name.starts_with("cctop"), "{}", process.name);
    assert_eq!(process.args.first(), std::env::args().next().as_ref());
    assert!(process.memory.is_some());
}

struct Reaped(Child);

impl Drop for Reaped {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(crate) fn reads_the_variables_of_a_child(source: &impl ProcessSource) {
    let child = Command::new("/bin/sleep")
        .arg("30")
        .env("CORNERCASE_PANE", "7")
        .env("CORNERCASE_SERVER", "x-1")
        .spawn()
        .expect("spawn sleep");
    let child = Reaped(child);
    let mut found = None;
    wait_until("the child runs sleep", || {
        found = own_process(source, child.0.id()).filter(|process| process.name == "sleep");
        found.is_some()
    });
    let process = found.expect("child listed");
    assert_eq!((process.pane, process.server.as_deref()), (Some(7), Some("x-1")));
    assert_eq!(process.ppid, Some(std::process::id()));
}

pub(crate) fn shows_cpu_after_spinning(source: &impl ProcessSource) {
    let pid = std::process::id();
    let before = source.sample().expect("first sample");
    let start = Instant::now();
    let mut spins = 0_u64;
    while start.elapsed() < SPIN {
        spins = std::hint::black_box(spins.wrapping_add(1));
    }
    let after = source.sample().expect("second sample");
    let percent = cpu_percent(&before, &after).get(&pid).copied().expect("cpu of the test process");
    assert!(percent > 0.0, "{percent}");
}

pub(crate) fn run_the_memory_probe_alone(probe: &str) {
    let output = Command::new(std::env::current_exe().expect("test binary"))
        .args([probe, "--exact", "--ignored", "--test-threads=1"])
        .env(MEMORY_PROBE_VARIABLE, "1")
        .output()
        .expect("run the memory probe");
    let report = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{report}");
    assert!(report.contains("1 passed"), "{report}");
}

pub(crate) fn counts_anonymous_memory_but_not_a_mapped_file(source: &impl ProcessSource) {
    if std::env::var_os(MEMORY_PROBE_VARIABLE).is_none() {
        return;
    }
    let memory = || own_process(source, std::process::id()).and_then(|p| p.memory).expect("own memory");
    let before = memory();
    let anonymous = Mapping::anonymous(64 * MIB);
    anonymous.write_every_page();
    assert!(memory() >= before + 60 * MIB, "anonymous memory not counted");
    drop(anonymous);
    let dir = TempDir::new();
    let path = dir.path().join("big");
    fs::write(&path, vec![1_u8; usize::try_from(64 * MIB).expect("fits")]).expect("write file");
    let before = memory();
    let file = Mapping::file(&path, 64 * MIB);
    assert!(file.read_every_page() > 0);
    assert!(memory() <= before + 8 * MIB, "mapped file counted");
}

struct Mapping {
    address: *mut libc::c_void,
    length: usize,
}

impl Mapping {
    fn anonymous(length: u64) -> Self {
        Self::map(length, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_PRIVATE | libc::MAP_ANON, -1)
    }

    fn file(path: &Path, length: u64) -> Self {
        let file = fs::File::open(path).expect("open mapped file");
        Self::map(length, libc::PROT_READ, libc::MAP_PRIVATE, std::os::fd::AsRawFd::as_raw_fd(&file))
    }

    fn map(length: u64, protection: i32, flags: i32, fd: i32) -> Self {
        let length = usize::try_from(length).expect("length fits");
        let address = unsafe { libc::mmap(std::ptr::null_mut(), length, protection, flags, fd, 0) };
        assert_ne!(address, libc::MAP_FAILED, "mmap failed");
        Self { address, length }
    }

    fn pages(&self) -> impl Iterator<Item = *mut u8> {
        let base = self.address.cast::<u8>();
        (0..self.length).step_by(4096).map(move |offset| base.wrapping_add(offset))
    }

    fn write_every_page(&self) {
        for page in self.pages() {
            unsafe { page.write_volatile(1) };
        }
    }

    fn read_every_page(&self) -> u64 {
        self.pages().map(|page| u64::from(unsafe { page.read_volatile() })).sum()
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.address, self.length) };
    }
}
