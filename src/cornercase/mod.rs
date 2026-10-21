use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

pub(crate) mod open;
pub(crate) mod snapshot;

#[cfg(test)]
pub(crate) mod fake;

pub(crate) use snapshot::Snapshot;

pub(crate) trait Runner {
    fn run(&self, argv: &[String]) -> Result<String>;
}

pub(crate) struct CommandRunner {
    program: OsString,
}

impl CommandRunner {
    pub(crate) fn from_env() -> Self {
        Self::new(std::env::var_os("CORNERCASE_BIN"))
    }

    fn new(bin: Option<OsString>) -> Self {
        let program = bin.filter(|bin| !bin.is_empty()).unwrap_or_else(|| "cornercase".into());
        Self { program }
    }
}

impl Runner for CommandRunner {
    fn run(&self, argv: &[String]) -> Result<String> {
        let output = Command::new(&self.program).args(argv).output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(Error::Command { argv: argv.to_vec(), stderr });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "no caller focuses a workspace or project yet; the client covers every focus flag")
)]
pub(crate) enum Target {
    Pane(u64),
    Tab(u64),
    Workspace(u64),
    Project(u64),
}

impl Target {
    fn flag(self) -> [String; 2] {
        let (flag, id) = match self {
            Self::Pane(id) => ("--pane", id),
            Self::Tab(id) => ("--tab", id),
            Self::Workspace(id) => ("--workspace", id),
            Self::Project(id) => ("--project", id),
        };
        [flag.to_owned(), id.to_string()]
    }
}

pub(crate) struct Client<R> {
    runner: R,
}

impl<R: Runner> Client<R> {
    pub(crate) fn new(runner: R) -> Self {
        Self { runner }
    }

    pub(crate) fn status(&self) -> Result<Snapshot> {
        Snapshot::parse(&self.runner.run(&argv(["status", "--json"]))?)
    }

    pub(crate) fn open(&self, path: &Path, focus: bool) -> Result<u64> {
        let mut argv = argv(["open", &path.to_string_lossy()]);
        if focus {
            argv.push("--focus".to_owned());
        }
        self.id(&argv)
    }

    pub(crate) fn new_tab(&self, workspace: u64, name: &str, command: &[String]) -> Result<u64> {
        let mut argv = argv(["new-tab", "--workspace", &workspace.to_string(), "--name", name, "--"]);
        argv.extend_from_slice(command);
        self.id(&argv)
    }

    pub(crate) fn focus(&self, target: Target) -> Result<()> {
        let mut argv = argv(["focus"]);
        argv.extend(target.flag());
        self.runner.run(&argv).map(drop)
    }

    pub(crate) fn notify(&self, text: &str) -> Result<()> {
        self.runner.run(&argv(["notify", "--", text])).map(drop)
    }

    fn id(&self, argv: &[String]) -> Result<u64> {
        let output = self.runner.run(argv)?;
        output.trim().parse().map_err(|_| Error::Answer { argv: argv.to_vec(), output })
    }
}

fn argv<const N: usize>(words: [&str; N]) -> Vec<String> {
    words.into_iter().map(str::to_owned).collect()
}

#[cfg(test)]
mod tests;
