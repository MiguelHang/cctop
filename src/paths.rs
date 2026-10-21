#![cfg_attr(not(test), expect(dead_code, reason = "used by the TUI and the watch service once they are wired"))]

use std::ffi::OsString;
use std::path::PathBuf;

const APP: &str = "cctop";
const CONFIG_FILE: &str = "config.toml";

pub(crate) fn config_dir(env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    absolute(&env, "CORNERCASE_PLUGIN_CONFIG")
        .or_else(|| absolute(&env, "XDG_CONFIG_HOME").map(|dir| dir.join(APP)))
        .or_else(|| home(&env).map(|home| home.join(".config").join(APP)))
}

pub(crate) fn config_file(env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    config_dir(env).map(|dir| dir.join(CONFIG_FILE))
}

pub(crate) fn state_dir(env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    absolute(&env, "CORNERCASE_PLUGIN_STATE")
        .or_else(|| absolute(&env, "XDG_STATE_HOME").map(|dir| dir.join(APP)))
        .or_else(|| home(&env).map(|home| home.join(".local").join("state").join(APP)))
}

pub(crate) fn system_env(key: &str) -> Option<OsString> {
    std::env::var_os(key)
}

fn home(env: &impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    absolute(env, "HOME")
}

fn absolute(env: &impl Fn(&str) -> Option<OsString>, key: &str) -> Option<PathBuf> {
    env(key).map(PathBuf::from).filter(|path| path.is_absolute())
}

#[cfg(test)]
mod tests;
