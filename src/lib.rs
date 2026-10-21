pub mod error;
mod format;
mod paths;

mod alerts;
mod config;
mod context;
mod cornercase;
mod procs;
mod usage;

mod app;
mod ui;

#[cfg(test)]
mod test_util;

pub use error::{Error, Result};

pub fn run_tui() -> Result<()> {
    app::run_tui()
}

pub fn run_watch() -> Result<()> {
    let loaded = paths::config_file(paths::system_env).map(|path| config::Config::load(&path)).unwrap_or_default();
    alerts::serve(&loaded);
    Ok(())
}

pub fn run_open() -> Result<()> {
    let state = cornercase::open::state_folder(std::env::var_os("CORNERCASE_PLUGIN_STATE"))?;
    let client = cornercase::Client::new(cornercase::CommandRunner::from_env());
    cornercase::open::open_cctop(&client, &state, &std::env::current_exe()?)
}
