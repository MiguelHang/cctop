use std::path::PathBuf;

use serde::Deserialize;

use crate::error::Result;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) boot: Option<String>,
    pub(crate) projects: Vec<Project>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Project {
    pub(crate) id: u64,
    pub(crate) name: Option<String>,
    pub(crate) path: Option<PathBuf>,
    group: Option<u64>,
    #[serde(default)]
    pub(crate) workspaces: Vec<Workspace>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Workspace {
    pub(crate) id: u64,
    pub(crate) name: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) path: Option<PathBuf>,
    #[serde(default)]
    pub(crate) worktree: bool,
    #[serde(default)]
    pub(crate) tabs: Vec<Tab>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Tab {
    pub(crate) id: u64,
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) panes: Vec<Pane>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Pane {
    pub(crate) id: u64,
    pub(crate) agent: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) model: Option<String>,
    #[serde(rename = "context")]
    pub(crate) context_percent: Option<u16>,
    pub(crate) subagents: Option<u32>,
    pub(crate) parent: Option<u64>,
    #[serde(rename = "program")]
    pub(crate) command: Option<String>,
    pub(crate) path: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Report {
    boot: Option<String>,
    #[serde(default)]
    groups: Vec<Group>,
    #[serde(default)]
    projects: Vec<Project>,
}

#[derive(Deserialize)]
struct Group {
    id: u64,
}

impl Snapshot {
    pub(crate) fn parse(json: &str) -> Result<Self> {
        let Report { boot, groups, mut projects } = serde_json::from_str(json)?;
        let place = |project: &Project| {
            project.group.and_then(|group| groups.iter().position(|g| g.id == group)).map_or(0, |at| at + 1)
        };
        projects.sort_by_key(place);
        Ok(Self { boot, projects })
    }
}
