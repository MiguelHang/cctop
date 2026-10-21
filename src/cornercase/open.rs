use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::snapshot::{Project, Tab};
use super::{Client, Runner, Snapshot, Target};
use crate::error::{Error, Result};

const CCTOP: &str = "cctop";

pub(crate) fn state_folder(var: Option<OsString>) -> Result<PathBuf> {
    let folder = var.filter(|var| !var.is_empty()).map(PathBuf::from).ok_or(Error::NotInCornercase)?;
    std::fs::create_dir_all(&folder)?;
    Ok(folder.canonicalize()?)
}

pub(crate) fn open_cctop<R: Runner>(client: &Client<R>, state: &Path, exe: &Path) -> Result<()> {
    let id = client.open(state, true)?;
    let snapshot = client.status()?;
    let project = find_project(&snapshot, id, state);
    if let Some(tab) = project.and_then(cctop_tab) {
        return client.focus(Target::Tab(tab.id));
    }
    let workspace =
        project.and_then(|project| project.workspaces.first()).ok_or_else(|| Error::NoWorkspace(state.to_owned()))?;
    let pane = client.new_tab(workspace.id, CCTOP, &[exe.to_string_lossy().into_owned()])?;
    client.focus(Target::Pane(pane))
}

fn find_project<'a>(snapshot: &'a Snapshot, id: u64, state: &Path) -> Option<&'a Project> {
    let by_id = snapshot.projects.iter().find(|project| project.id == id);
    by_id.or_else(|| {
        snapshot.projects.iter().find(|project| project.path.as_deref().is_some_and(|path| same_path(path, state)))
    })
}

fn same_path(a: &Path, b: &Path) -> bool {
    let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_owned());
    canonical(a) == canonical(b)
}

fn cctop_tab(project: &Project) -> Option<&Tab> {
    project.workspaces.iter().flat_map(|workspace| &workspace.tabs).find(|tab| runs_cctop(tab))
}

fn runs_cctop(tab: &Tab) -> bool {
    let programs: Vec<&str> = tab.panes.iter().filter_map(|pane| pane.command.as_deref()).collect();
    if programs.is_empty() {
        return tab.name.as_deref() == Some(CCTOP);
    }
    programs.into_iter().any(|program| Path::new(program).file_name().is_some_and(|name| name == CCTOP))
}

#[cfg(test)]
mod tests;
