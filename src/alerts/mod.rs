#[cfg(test)]
mod fake;
mod message;
mod rules;
mod watch;

use std::io::Write;

use crate::config::Loaded;
use crate::cornercase::{Client, CommandRunner};
use crate::procs;

use rules::Thresholds;
use watch::{SystemClock, Watch};

pub(crate) fn serve(loaded: &Loaded) {
    let mut log = std::io::stderr();
    for warning in &loaded.warnings {
        let _ = writeln!(log, "cctop watch: {warning}");
    }
    let client = Client::new(CommandRunner::from_env());
    let thresholds = Thresholds::from(&loaded.config);
    let mut watch = Watch::new(client, procs::system(), log, thresholds);
    watch.run(&SystemClock, loaded.config.alert_interval, || true);
}
