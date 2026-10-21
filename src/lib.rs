pub mod error;
#[cfg_attr(not(test), expect(dead_code, reason = "only ui uses it until the app draws ui in task 010"))]
mod format;
mod paths;

mod alerts;
mod config;
mod context;
mod cornercase;
#[cfg_attr(not(test), expect(dead_code, reason = "usage and alerts read the samples once they land"))]
mod procs;
mod usage;

mod app;
#[cfg_attr(not(test), expect(dead_code, unused_imports, reason = "the app draws it from task 010"))]
mod ui;

#[cfg(test)]
mod test_util;

pub use error::{Error, Result};

pub fn run_tui() -> Result<()> {
    Ok(())
}

pub fn run_watch() -> Result<()> {
    Ok(())
}

pub fn run_open() -> Result<()> {
    let state = cornercase::open::state_folder(std::env::var_os("CORNERCASE_PLUGIN_STATE"))?;
    let client = cornercase::Client::new(cornercase::CommandRunner::from_env());
    cornercase::open::open_cctop(&client, &state, &std::env::current_exe()?)
}
