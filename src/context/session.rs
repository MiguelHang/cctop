use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::paths::absolute;

const CONFIG_ENV: &str = "CLAUDE_CONFIG_DIR";
const DIR_NAME_MAX: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Session {
    pub(crate) id: String,
    cwd: PathBuf,
}

#[cfg_attr(not(test), expect(dead_code, reason = "the app reads each claude's environment, task 012"))]
pub(crate) fn config_dir(env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    absolute(&env, CONFIG_ENV).or_else(|| absolute(&env, "HOME").map(|home| home.join(".claude")))
}

pub(crate) fn session(config: &Path, pid: u32) -> Option<Session> {
    let text = fs::read_to_string(config.join("sessions").join(format!("{pid}.json"))).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    let id = value["sessionId"].as_str().filter(|id| safe(id))?.to_owned();
    let cwd = PathBuf::from(value["cwd"].as_str()?);
    Some(Session { id, cwd })
}

pub(crate) fn transcript(config: &Path, session: &Session) -> Option<PathBuf> {
    let projects = config.join("projects");
    let file = format!("{}.jsonl", session.id);
    let direct = projects.join(project_dir(&session.cwd)).join(&file);
    if direct.is_file() {
        return Some(direct);
    }
    fs::read_dir(&projects).ok()?.flatten().map(|entry| entry.path().join(&file)).find(|path| path.is_file())
}

fn safe(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn project_dir(cwd: &Path) -> String {
    let cwd = cwd.to_string_lossy();
    let name: String = cwd
        .encode_utf16()
        .map(|unit| u8::try_from(unit).ok().filter(u8::is_ascii_alphanumeric).map_or('-', char::from))
        .collect();
    if name.len() <= DIR_NAME_MAX {
        return name;
    }
    format!("{}-{}", &name[..DIR_NAME_MAX], base36(string_hash(&cwd)))
}

fn string_hash(text: &str) -> u32 {
    text.encode_utf16()
        .fold(0_i32, |hash, unit| hash.wrapping_shl(5).wrapping_sub(hash).wrapping_add(i32::from(unit)))
        .unsigned_abs()
}

fn base36(mut n: u32) -> String {
    let mut digits = Vec::new();
    loop {
        digits.push(char::from_digit(n % 36, 36).unwrap_or('0'));
        n /= 36;
        if n == 0 {
            return digits.iter().rev().collect();
        }
    }
}

#[cfg(test)]
mod tests;
