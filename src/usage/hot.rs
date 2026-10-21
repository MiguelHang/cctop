use super::{ProcessNode, Sort, Tree};

pub(super) fn mark(tree: &mut Tree, by: Sort) {
    let panes = tree.projects.iter_mut().flat_map(|p| &mut p.workspaces).flat_map(|w| &mut w.tabs);
    for pane in panes.flat_map(|tab| &mut tab.panes) {
        mark_processes(&mut pane.processes, by);
    }
}

fn mark_processes(roots: &mut [ProcessNode], by: Sort) {
    let mut path = Vec::new();
    if let Some((_, found)) = hottest(roots, by, &mut path) {
        set(roots, &found);
    }
}

type Found = (f64, Vec<usize>);

fn hottest(nodes: &[ProcessNode], by: Sort, path: &mut Vec<usize>) -> Option<Found> {
    let mut best = None;
    for (at, node) in nodes.iter().enumerate() {
        path.push(at);
        if let Some(value) = by.value(node.memory, node.cpu).filter(|value| *value > 0.0) {
            best = Some(improve(best, value, path));
        }
        best = [best, hottest(&node.children, by, path)].into_iter().flatten().reduce(keep_first_max);
        path.pop();
    }
    best
}

fn improve(best: Option<Found>, value: f64, path: &[usize]) -> Found {
    match best {
        Some(found) if found.0 >= value => found,
        _ => (value, path.to_vec()),
    }
}

fn keep_first_max(best: Found, candidate: Found) -> Found {
    if candidate.0 > best.0 { candidate } else { best }
}

fn set(nodes: &mut [ProcessNode], path: &[usize]) {
    let Some((first, rest)) = path.split_first() else { return };
    if let Some(node) = nodes.get_mut(*first) {
        node.hot = true;
        set(&mut node.children, rest);
    }
}
