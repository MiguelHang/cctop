#![cfg_attr(not(test), expect(dead_code, reason = "used by the TUI and the watch service once they are wired"))]

mod values;

use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;
use toml::Value;

use values::{DURATION, Kind, PERCENT, SIZE, SORT};

const GIB: u64 = 1 << 30;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Config {
    pub(crate) interval: Duration,
    pub(crate) memory_threshold: u64,
    pub(crate) cpu_threshold: f64,
    pub(crate) cpu_sustain: Duration,
    pub(crate) alert_interval: Duration,
    pub(crate) sort: Sort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Sort {
    #[default]
    Memory,
    Cpu,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Loaded {
    pub(crate) config: Config,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Raw {
    interval: Option<Value>,
    memory_threshold: Option<Value>,
    cpu_threshold: Option<Value>,
    cpu_sustain: Option<Value>,
    alert_interval: Option<Value>,
    sort: Option<Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(2),
            memory_threshold: 4 * GIB,
            cpu_threshold: 300.0,
            cpu_sustain: Duration::from_secs(60),
            alert_interval: Duration::from_secs(5),
            sort: Sort::Memory,
        }
    }
}

impl Config {
    pub(crate) fn load(path: &Path) -> Loaded {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text, path),
            Err(error) if error.kind() == ErrorKind::NotFound => Loaded::defaults(Vec::new()),
            Err(error) => {
                Loaded::defaults(vec![format!("could not read {}: {error}; using the defaults", path.display())])
            }
        }
    }

    pub(crate) fn parse(text: &str, path: &Path) -> Loaded {
        match toml::from_str::<Raw>(text) {
            Ok(raw) => Self::from_raw(raw),
            Err(error) => Loaded::defaults(vec![not_toml(text, path, &error)]),
        }
    }

    fn from_raw(raw: Raw) -> Loaded {
        let default = Self::default();
        let mut warnings = Vec::new();
        let config = Self {
            interval: setting(raw.interval, "interval", &DURATION, default.interval, &mut warnings),
            memory_threshold: setting(
                raw.memory_threshold,
                "memory_threshold",
                &SIZE,
                default.memory_threshold,
                &mut warnings,
            ),
            cpu_threshold: setting(raw.cpu_threshold, "cpu_threshold", &PERCENT, default.cpu_threshold, &mut warnings),
            cpu_sustain: setting(raw.cpu_sustain, "cpu_sustain", &DURATION, default.cpu_sustain, &mut warnings),
            alert_interval: setting(
                raw.alert_interval,
                "alert_interval",
                &DURATION,
                default.alert_interval,
                &mut warnings,
            ),
            sort: setting(raw.sort, "sort", &SORT, default.sort, &mut warnings),
        };
        Loaded { config, warnings }
    }
}

impl Loaded {
    fn defaults(warnings: Vec<String>) -> Self {
        Self { config: Config::default(), warnings }
    }
}

fn not_toml(text: &str, path: &Path, error: &toml::de::Error) -> String {
    let line = error.span().and_then(|span| text.get(..span.start)).map(|before| before.matches('\n').count() + 1);
    let place = line.map_or_else(String::new, |line| format!(" at line {line}"));
    format!("{} is not valid TOML{place} ({}); using the defaults", path.display(), error.message())
}

fn setting<T>(value: Option<Value>, key: &str, kind: &Kind<T>, default: T, warnings: &mut Vec<String>) -> T {
    let Some(value) = value else { return default };
    (kind.parse)(&value).unwrap_or_else(|| {
        warnings.push(format!("{key} = {value} is not valid, expected {}; using the default", kind.expected));
        default
    })
}

#[cfg(test)]
mod tests;
