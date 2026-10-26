mod version;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub(crate) use version::Version;

use crate::error::{Error, Result};

const LATEST: &str = "https://api.github.com/repos/MiguelHang/cctop/releases/latest";
const CACHE_FILE: &str = "latest-release.json";
const CHECK_EVERY: Duration = Duration::from_hours(24);
const TIMEOUT_SECONDS: &str = "10";

pub(crate) trait Fetch {
    fn get(&self, url: &str) -> Result<String>;
}

pub(crate) struct Curl {
    program: &'static str,
}

impl Default for Curl {
    fn default() -> Self {
        Self { program: "curl" }
    }
}

impl Fetch for Curl {
    fn get(&self, url: &str) -> Result<String> {
        let argv = ["--fail", "--silent", "--show-error", "--location", "--max-time", TIMEOUT_SECONDS, url];
        let output = Command::new(self.program).args(argv).output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let argv = std::iter::once(self.program).chain(argv).map(str::to_owned).collect();
            return Err(Error::Command { argv, stderr });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Cache {
    checked: u64,
    tag: Option<String>,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

pub(crate) fn cache_file(state: &Path) -> PathBuf {
    state.join(CACHE_FILE)
}

pub(crate) fn own() -> Option<Version> {
    Version::parse(env!("CARGO_PKG_VERSION"))
}

pub(crate) fn spawn(cache: PathBuf) -> Receiver<Version> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let newer = own().and_then(|own| newer(&Curl::default(), &cache, SystemTime::now(), own));
        if let Some(newer) = newer {
            let _ = sender.send(newer);
        }
    });
    receiver
}

pub(crate) fn newer(fetch: &impl Fetch, cache: &Path, now: SystemTime, own: Version) -> Option<Version> {
    let latest = latest(fetch, cache, seconds(now));
    latest.as_deref().and_then(Version::parse).filter(|latest| *latest > own)
}

fn latest(fetch: &impl Fetch, cache: &Path, now: u64) -> Option<String> {
    let previous = read(cache);
    if let Some(previous) = previous.as_ref().filter(|previous| fresh(previous, now)) {
        return previous.tag.clone();
    }
    let tag = fetch.get(LATEST).ok().and_then(|body| serde_json::from_str::<Release>(&body).ok()).map(|r| r.tag_name);
    let tag = tag.or_else(|| previous.and_then(|previous| previous.tag));
    write(cache, &Cache { checked: now, tag: tag.clone() });
    tag
}

fn fresh(cache: &Cache, now: u64) -> bool {
    now.checked_sub(cache.checked).is_some_and(|age| age < CHECK_EVERY.as_secs())
}

fn seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

fn read(cache: &Path) -> Option<Cache> {
    serde_json::from_str(&std::fs::read_to_string(cache).ok()?).ok()
}

fn write(cache: &Path, value: &Cache) {
    let Ok(text) = serde_json::to_string(value) else { return };
    let Some(folder) = cache.parent() else { return };
    let temporary = folder.join(format!(".{CACHE_FILE}.{}", std::process::id()));
    let written = std::fs::create_dir_all(folder)
        .and_then(|()| std::fs::write(&temporary, text))
        .and_then(|()| std::fs::rename(&temporary, cache));
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
}

#[cfg(test)]
mod tests;
