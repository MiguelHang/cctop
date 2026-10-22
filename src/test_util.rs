macro_rules! fixture {
    ($path:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/", $path))
    };
}

pub(crate) use fixture;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;

const TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(20);

pub(crate) fn wait_until(what: &str, cond: impl FnMut() -> bool) {
    wait_until_within(what, TIMEOUT, cond);
}

pub(crate) fn wait_until_within(what: &str, timeout: Duration, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while !cond() {
        assert!(Instant::now() < deadline, "timed out waiting for: {what}");
        std::thread::sleep(POLL);
    }
}

pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub(crate) fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("cctop-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir.canonicalize().expect("canonicalize temp dir"))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn screen_line(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect()
}

pub(crate) fn find(buffer: &Buffer, text: &str) -> (u16, u16) {
    (0..buffer.area.height)
        .find_map(|y| {
            let line = screen_line(buffer, y);
            let at = line.find(text)?;
            Some((u16::try_from(line[..at].chars().count()).expect("narrow line"), y))
        })
        .unwrap_or_else(|| panic!("{text:?} is not on screen"))
}

#[cfg(test)]
mod tests;
