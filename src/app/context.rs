use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;

#[cfg(test)]
pub(super) mod fake;

use crate::context::breakdown::{Breakdown, Group, Grouped, Part, PartKind};
use crate::context::{Claude, Sessions, session};
use crate::cornercase::Snapshot;
use crate::procs::{Pid, Process, Sample};
use crate::ui::{ContextDetail, ContextGroup, ContextPart, ContextPercent, ContextSession};
use crate::usage::{self, PaneNode, Tree};

const CLAUDE: &str = "claude";
const RUNTIMES: [&str; 2] = ["node", "bun"];
const WINDOWS: [u64; 2] = [200_000, 1_000_000];
const TOP: usize = 3;
const OPEN_AT_START: usize = 2;

pub(super) type Folds = HashMap<Group, bool>;

pub(crate) trait Contexts {
    fn read(&mut self, claudes: &[Claude]) -> HashMap<Pid, Breakdown>;
}

impl Contexts for Sessions {
    fn read(&mut self, claudes: &[Claude]) -> HashMap<Pid, Breakdown> {
        Sessions::read(self, claudes)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PaneContext {
    pub(super) pane: u64,
    pub(super) found: Found,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Found {
    NoClaude,
    NoSession(Pid),
    Session(Pid, Breakdown),
}

pub(super) struct Source<'a> {
    pub(super) snapshot: &'a Snapshot,
    pub(super) sample: &'a Sample,
    pub(super) home: Option<&'a OsString>,
}

pub(super) fn read(contexts: &mut impl Contexts, source: &Source<'_>, pane: Option<u64>) -> Option<PaneContext> {
    let Some(pane) = pane else {
        contexts.read(&[]);
        return None;
    };
    let Some(process) = find_claude(source.snapshot, source.sample, pane) else {
        contexts.read(&[]);
        return Some(PaneContext { pane, found: Found::NoClaude });
    };
    let pid = process.pid;
    let claudes: Vec<Claude> =
        config_dir(process, source.home).map(|config| Claude { pid, config }).into_iter().collect();
    let found = match contexts.read(&claudes).remove(&pid) {
        Some(breakdown) => Found::Session(pid, breakdown),
        None => Found::NoSession(pid),
    };
    Some(PaneContext { pane, found })
}

fn find_claude<'a>(snapshot: &Snapshot, sample: &'a Sample, pane: u64) -> Option<&'a Process> {
    usage::pane_processes(snapshot, sample, pane).into_iter().find(|process| is_claude(process))
}

fn is_claude(process: &Process) -> bool {
    let program = |index: usize| process.args.get(index).and_then(|arg| Path::new(arg).file_name());
    let named = |index: usize, names: &[&str]| program(index).is_some_and(|name| names.iter().any(|n| name == *n));
    process.name == CLAUDE || named(0, &[CLAUDE]) || (named(0, &RUNTIMES) && named(1, &[CLAUDE]))
}

fn config_dir(process: &Process, home: Option<&OsString>) -> Option<std::path::PathBuf> {
    session::config_dir(|key| match key {
        "HOME" => home.cloned(),
        session::CONFIG_ENV => process.claude_config_dir.clone().map(OsString::from),
        _ => None,
    })
}

pub(super) fn pane_node(tree: &Tree, id: u64) -> Option<&PaneNode> {
    let tabs = tree.projects.iter().flat_map(|project| &project.workspaces).flat_map(|workspace| &workspace.tabs);
    tabs.flat_map(|tab| &tab.panes).find(|pane| pane.id == id)
}

pub(super) fn view(
    pane: Option<(&PaneNode, &str)>,
    context: Option<&PaneContext>,
    scroll: usize,
    folds: &Folds,
) -> ContextDetail {
    let Some((pane, label)) = pane else { return ContextDetail::NoAgent };
    let label = label.to_owned();
    match context.filter(|context| context.pane == pane.id).map(|context| &context.found) {
        None => ContextDetail::Reading { pane: label },
        Some(Found::NoClaude) => ContextDetail::NoClaude { pane: label },
        Some(&Found::NoSession(pid)) => ContextDetail::NoSession { pane: label, pid },
        Some(Found::Session(_, breakdown)) => {
            let root = pane.path.as_deref();
            let grouped = breakdown.groups();
            let open = states(&grouped, folds);
            ContextDetail::Session(ContextSession {
                total: breakdown.total,
                percent: breakdown.total.zip(pane.context_percent).and_then(|(total, shown)| percent(total, shown)),
                loaded: loaded(&breakdown.fixed, root),
                groups: grouped.iter().zip(open).map(|(grouped, (_, open))| group(grouped, open, root)).collect(),
                scroll,
            })
        }
    }
}

pub(super) fn states(groups: &[Grouped], folds: &Folds) -> Vec<(Group, Option<bool>)> {
    let mut rank = 0;
    let mut state = |grouped: &Grouped| {
        let open = folds.get(&grouped.group).copied().unwrap_or(rank < OPEN_AT_START);
        rank += 1;
        open
    };
    groups.iter().map(|grouped| (grouped.group, itemized(grouped.group).then(|| state(grouped)))).collect()
}

fn itemized(group: Group) -> bool {
    matches!(group, Group::Commands | Group::Files | Group::Searches)
}

fn group(grouped: &Grouped, open: Option<bool>, root: Option<&Path>) -> ContextGroup {
    let (name, noun) = match grouped.group {
        Group::Commands => ("Commands", "command"),
        Group::Files => ("Files", "file"),
        Group::Searches => ("Searches", "search"),
        Group::Replies => ("Claude's replies", ""),
        Group::Prompts => ("Your messages", ""),
        Group::Subagents => ("Subagent results", ""),
        Group::OtherTools => ("Other tools", ""),
    };
    let (top, rest) =
        if open.is_some() { grouped.parts.split_at(grouped.parts.len().min(TOP)) } else { (&[][..], &[][..]) };
    ContextGroup {
        name: name.to_owned(),
        noun: noun.to_owned(),
        tokens: grouped.tokens,
        items: parts(top, root),
        more: rest.len(),
        more_tokens: rest.iter().map(|part| part.tokens.0).sum(),
        open: open.unwrap_or(false),
    }
}

fn loaded(fixed: &[Part], root: Option<&Path>) -> Vec<ContextPart> {
    let mut loaded = parts(fixed, root);
    loaded.sort_by_key(|part| std::cmp::Reverse(part.tokens));
    loaded
}

#[expect(clippy::cast_precision_loss, reason = "a percentage is shown with one decimal at most")]
fn percent(total: u64, cornercase: u16) -> Option<ContextPercent> {
    let shown = f64::from(cornercase);
    let used = |window: u64| total as f64 / window as f64 * 100.0;
    let distance = |window: u64| (used(window) - shown).abs();
    let window =
        WINDOWS.into_iter().filter(|window| *window >= total).min_by(|a, b| distance(*a).total_cmp(&distance(*b)))?;
    Some(ContextPercent { used: used(window), window })
}

fn parts(parts: &[Part], root: Option<&Path>) -> Vec<ContextPart> {
    parts
        .iter()
        .map(|part| {
            let (label, path) = label(&part.kind, root);
            ContextPart { label, tokens: part.tokens.0, path }
        })
        .collect()
}

fn label(kind: &PartKind, root: Option<&Path>) -> (String, bool) {
    let text = match kind {
        PartKind::Instructions(path) | PartKind::File(path) => return (relative(path, root), true),
        PartKind::Command(text) | PartKind::Search(text) => return (one_line(text), false),
        PartKind::SystemPrompt => "System prompt",
        PartKind::Skills => "Skills",
        PartKind::Tools => "Deferred tools",
        PartKind::Mcp => "MCP instructions",
        PartKind::Agents => "Agents",
        PartKind::Prompts => "User prompts",
        PartKind::Replies => "Assistant replies",
        PartKind::Subagents => "Subagent results",
        PartKind::OtherTools => "Other tools",
    };
    (text.to_owned(), false)
}

fn relative(path: &str, root: Option<&Path>) -> String {
    root.and_then(|root| Path::new(path).strip_prefix(root).ok())
        .filter(|inside| !inside.as_os_str().is_empty())
        .map_or_else(|| path.to_owned(), |inside| inside.to_string_lossy().into_owned())
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests;
