use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};

use super::record::{Delta, Entry, Listing, Record, Reply, Tool, UserBlock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Breakdown {
    pub(crate) total: Option<u64>,
    pub(crate) fixed: Vec<Part>,
    pub(crate) dynamic: Vec<Part>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Part {
    pub(crate) kind: PartKind,
    pub(crate) tokens: Estimate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Estimate(pub(crate) u64);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum PartKind {
    SystemPrompt,
    Instructions(String),
    Skills,
    Tools,
    Mcp,
    Agents,
    File(String),
    Command(String),
    Search(String),
    Prompts,
    Replies,
    Subagents,
    OtherTools,
}

impl From<Tool> for PartKind {
    fn from(tool: Tool) -> Self {
        match tool {
            Tool::File(path) => Self::File(path),
            Tool::Command(command) => Self::Command(command),
            Tool::Search(pattern) => Self::Search(pattern),
            Tool::Subagent => Self::Subagents,
            Tool::Other => Self::OtherTools,
        }
    }
}

impl From<Listing> for PartKind {
    fn from(listing: Listing) -> Self {
        match listing {
            Listing::Tools => Self::Tools,
            Listing::Mcp => Self::Mcp,
            Listing::Agents => Self::Agents,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Accumulator {
    system_prompt: Option<usize>,
    instructions: Vec<Entry>,
    skills: Option<usize>,
    listings: BTreeMap<Listing, BTreeMap<String, usize>>,
    uses: HashMap<String, PartKind>,
    dynamic: HashMap<PartKind, usize>,
    total: Option<u64>,
}

impl Accumulator {
    pub(crate) fn push(&mut self, record: Record) {
        match record {
            Record::Instructions(files) => self.instructions = files,
            Record::Skills(chars) => self.skills = Some(chars),
            Record::SystemPrompt(chars) => self.system_prompt = Some(chars),
            Record::Delta(listing, delta) => self.apply(listing, delta),
            Record::CompactBoundary => self.compact(),
            Record::User(blocks) => blocks.into_iter().for_each(|block| self.user(block)),
            Record::Assistant(reply) => self.reply(reply),
            Record::Other => {}
        }
    }

    pub(crate) fn breakdown(&self) -> Breakdown {
        let fixed = self.fixed();
        let mut dynamic: Vec<(PartKind, usize)> =
            self.dynamic.iter().map(|(kind, &chars)| (kind.clone(), chars)).collect();
        dynamic.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let estimates: Vec<u64> = fixed.iter().chain(&dynamic).map(|&(_, chars)| estimate(chars)).collect();
        let mut tokens = scale(&estimates, self.total).into_iter();
        let mut parts = |sizes: Vec<(PartKind, usize)>| -> Vec<Part> {
            sizes
                .into_iter()
                .zip(&mut tokens)
                .map(|((kind, _), tokens)| Part { kind, tokens: Estimate(tokens) })
                .collect()
        };
        Breakdown { total: self.total, fixed: parts(fixed), dynamic: parts(dynamic) }
    }

    fn fixed(&self) -> Vec<(PartKind, usize)> {
        let mut parts: Vec<(PartKind, usize)> = Vec::new();
        parts.extend(self.system_prompt.map(|chars| (PartKind::SystemPrompt, chars)));
        parts.extend(self.instructions.iter().map(|file| (PartKind::Instructions(file.name.clone()), file.chars)));
        parts.extend(self.skills.map(|chars| (PartKind::Skills, chars)));
        let listings = self.listings.iter().filter(|(_, entries)| !entries.is_empty());
        parts.extend(listings.map(|(&listing, entries)| (listing.into(), entries.values().sum())));
        parts
    }

    fn apply(&mut self, listing: Listing, delta: Delta) {
        let entries = self.listings.entry(listing).or_default();
        for name in delta.removed {
            entries.remove(&name);
        }
        entries.extend(delta.added.into_iter().map(|entry| (entry.name, entry.chars)));
    }

    fn compact(&mut self) {
        self.uses.clear();
        self.dynamic.clear();
        self.total = None;
    }

    fn user(&mut self, block: UserBlock) {
        match block {
            UserBlock::Prompt(chars) => self.add(PartKind::Prompts, chars),
            UserBlock::ToolResult { id, chars } => {
                let kind = self.uses.remove(&id).unwrap_or(PartKind::OtherTools);
                self.add(kind, chars);
            }
        }
    }

    fn reply(&mut self, reply: Reply) {
        self.add(PartKind::Replies, reply.text);
        for tool_use in reply.tools {
            let kind = PartKind::from(tool_use.tool);
            self.add(kind.clone(), tool_use.chars);
            self.uses.insert(tool_use.id, kind);
        }
        if let Some(total) = reply.usage {
            self.total = Some(total);
        }
    }

    fn add(&mut self, kind: PartKind, chars: usize) {
        if chars > 0 {
            *self.dynamic.entry(kind).or_default() += chars;
        }
    }
}

fn estimate(chars: usize) -> u64 {
    u64::try_from(chars).unwrap_or(u64::MAX).div_ceil(4)
}

fn scale(estimates: &[u64], total: Option<u64>) -> Vec<u64> {
    let sum: u128 = estimates.iter().map(|&estimate| u128::from(estimate)).sum();
    let Some(total) = total.filter(|_| sum > 0) else {
        return if total.is_some() { vec![0; estimates.len()] } else { estimates.to_vec() };
    };
    let exact: Vec<u128> = estimates.iter().map(|&estimate| u128::from(estimate) * u128::from(total)).collect();
    let mut tokens: Vec<u64> = exact.iter().map(|share| u64::try_from(share / sum).unwrap_or(u64::MAX)).collect();
    let missing = total.saturating_sub(tokens.iter().sum());
    let mut order: Vec<usize> = (0..tokens.len()).collect();
    order.sort_by_key(|&index| Reverse(exact[index] % sum));
    for &index in order.iter().take(usize::try_from(missing).unwrap_or(usize::MAX)) {
        tokens[index] += 1;
    }
    tokens
}

#[cfg(test)]
mod tests;
