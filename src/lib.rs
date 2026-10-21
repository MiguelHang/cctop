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
    Ok(())
}

pub fn run_watch() -> Result<()> {
    Ok(())
}

pub fn run_open() -> Result<()> {
    Ok(())
}
