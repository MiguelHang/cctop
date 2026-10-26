pub mod error;
mod format;
mod paths;

mod alerts;
mod config;
mod context;
mod cornercase;
mod procs;
mod release;
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
    alerts::serve(&config::load_system());
    Ok(())
}

pub fn run_open() -> Result<()> {
    let clicked = cornercase::open::clicked(paths::system_env)?;
    let client = cornercase::Client::new(cornercase::CommandRunner::from_env());
    cornercase::open::open_cctop(&client, clicked, &std::env::current_exe()?)
}
