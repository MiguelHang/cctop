use std::ffi::OsString;
use std::path::Path;

use super::snapshot::{Pane, Project, Tab, Workspace};
use super::{Client, Runner, Snapshot, Target};
use crate::error::{Error, Result};

const CCTOP: &str = "cctop";
const CONTEXT: &str = "CORNERCASE_CONTEXT";

type Place = fn(u64) -> Target;

const CLICKED: [(&str, Place); 4] = [
    ("CORNERCASE_PANE", Target::Pane),
    ("CORNERCASE_TAB", Target::Tab),
    ("CORNERCASE_WORKSPACE", Target::Workspace),
    ("CORNERCASE_PROJECT", Target::Project),
];

pub(crate) fn clicked(env: impl Fn(&str) -> Option<OsString>) -> Result<Target> {
    CLICKED
        .into_iter()
        .find_map(|(key, target)| env(key).and_then(|id| id.to_str()?.trim().parse().ok()).map(target))
        .ok_or(Error::NotInCornercase)
}

pub(crate) fn launched(env: impl Fn(&str) -> Option<OsString>) -> Option<Target> {
    env(CONTEXT).filter(|context| !context.is_empty())?;
    clicked(env).ok()
}

pub(crate) fn open_cctop<R: Runner>(client: &Client<R>, clicked: Target, exe: &Path) -> Result<()> {
    let snapshot = client.status()?;
    let tab = clicked_tab(&snapshot, clicked).ok_or_else(|| Error::NoPane(describe(clicked)))?;
    if let Some(pane) = tab.panes.iter().find(|pane| runs_cctop(pane)) {
        return client.focus(Target::Pane(pane.id));
    }
    let pane = match clicked {
        Target::Pane(id) => id,
        _ => active_or_first(&tab.panes, |pane| pane.active).ok_or_else(|| Error::NoPane(describe(clicked)))?.id,
    };
    client.split_focused(pane, &["exec".to_owned(), shell_quote(&exe.to_string_lossy())]).map(drop)
}

fn clicked_tab(snapshot: &Snapshot, clicked: Target) -> Option<&Tab> {
    let workspaces = || snapshot.projects.iter().flat_map(|project| &project.workspaces);
    let tabs = || workspaces().flat_map(|workspace| &workspace.tabs);
    match clicked {
        Target::Pane(id) => tabs().find(|tab| tab.panes.iter().any(|pane| pane.id == id)),
        Target::Tab(id) => tabs().find(|tab| tab.id == id),
        Target::Workspace(id) => workspaces().find(|workspace| workspace.id == id).and_then(shown_tab),
        Target::Project(id) => snapshot.projects.iter().find(|project| project.id == id).and_then(shown_workspace),
    }
}

fn shown_workspace(project: &Project) -> Option<&Tab> {
    active_or_first(&project.workspaces, |workspace| workspace.active).and_then(shown_tab)
}

fn shown_tab(workspace: &Workspace) -> Option<&Tab> {
    active_or_first(&workspace.tabs, |tab| tab.active)
}

fn active_or_first<T>(items: &[T], active: impl Fn(&T) -> bool) -> Option<&T> {
    items.iter().find(|item| active(item)).or_else(|| items.first())
}

fn runs_cctop(pane: &Pane) -> bool {
    pane.command.as_deref().is_some_and(|program| Path::new(program).file_name().is_some_and(|name| name == CCTOP))
}

fn describe(target: Target) -> String {
    match target {
        Target::Pane(id) => format!("pane {id}"),
        Target::Tab(id) => format!("tab {id}"),
        Target::Workspace(id) => format!("workspace {id}"),
        Target::Project(id) => format!("project {id}"),
    }
}

fn shell_quote(word: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "/._+-=:,@%".contains(c);
    if !word.is_empty() && word.chars().all(plain) {
        return word.to_owned();
    }
    format!("'{}'", word.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests;
