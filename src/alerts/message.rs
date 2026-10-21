use super::rules::{Alert, Kind};
use crate::format;
use crate::usage::ProcessNode;

const MAX_COMMAND: usize = 40;

pub(crate) fn message(alert: &Alert<'_>) -> String {
    let pane = alert.pane;
    let partial = alert.kind == Kind::Memory && pane.totals.memory_partial;
    let total = format::estimated(figure(alert.kind, pane.totals.memory, pane.totals.cpu), partial);
    let heaviest = heaviest(&pane.processes, alert.kind)
        .map(|process| format!(" — {} {}", command(process), figure(alert.kind, process.memory, process.cpu)));
    format!("cctop: {} › {} {total}{}", workspace(alert), pane_name(alert), heaviest.unwrap_or_default())
}

fn workspace(alert: &Alert<'_>) -> String {
    let workspace = alert.workspace;
    workspace.name.clone().or_else(|| workspace.branch.clone()).unwrap_or_else(|| format!("workspace {}", workspace.id))
}

fn pane_name(alert: &Alert<'_>) -> String {
    let pane = alert.pane;
    pane.agent.clone().or_else(|| pane.command.clone()).unwrap_or_else(|| format!("pane {}", pane.id))
}

fn figure(kind: Kind, memory: Option<u64>, cpu: Option<f64>) -> String {
    match kind {
        Kind::Memory => format::or_missing(memory, format::bytes),
        Kind::Cpu => format::or_missing(cpu, format::percent),
    }
}

fn heaviest(processes: &[ProcessNode], kind: Kind) -> Option<&ProcessNode> {
    let own = |process: &ProcessNode| match kind {
        #[expect(clippy::cast_precision_loss, reason = "only compared, never shown")]
        Kind::Memory => process.memory.map(|memory| memory as f64),
        Kind::Cpu => process.cpu,
    };
    flatten(processes)
        .filter(|process| own(process).is_some_and(|value| value > 0.0))
        .reduce(|best, process| if own(process) > own(best) { process } else { best })
}

fn flatten(processes: &[ProcessNode]) -> Box<dyn Iterator<Item = &ProcessNode> + '_> {
    Box::new(processes.iter().flat_map(|process| std::iter::once(process).chain(flatten(&process.children))))
}

fn command(process: &ProcessNode) -> String {
    let words = std::iter::once(process.name.as_str()).chain(process.args.iter().skip(1).map(String::as_str));
    let line = words.collect::<Vec<_>>().join(" ");
    if line.chars().count() <= MAX_COMMAND {
        return line;
    }
    let cut: String = line.chars().take(MAX_COMMAND - 1).collect();
    format!("{}…", cut.trim_end())
}

#[cfg(test)]
mod tests;
