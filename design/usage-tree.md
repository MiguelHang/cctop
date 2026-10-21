# Usage tree

Code: `usage/`

- **Hierarchy**: project → workspace (branch, worktree or not) → tab → pane → process tree, in cornercase's sidebar order. Every level carries the sum of memory and CPU below it, so a collapsed project still shows its cost.
- **Pane roots**: a pane's processes are those mapped to it (see Cornercase client); the roots are the ones whose parent is not mapped to the same pane, normally the shell. Children are nested by parent pid.
- **Hot path**: in each pane the most expensive process (memory, or CPU when sorting by CPU) and the chain of ancestors to it are marked, so `claude → bash → cargo test → test binary` reads at a glance.
- **Sorting** by memory (default) or CPU applies inside each level; projects and workspaces keep cornercase's order unless sorting is on, then they sort by their totals too.
- **Agents**: a pane shows cornercase's agent, status, model, context % and subagents count from `status`; nothing is guessed from process names that cornercase already knows.
- Pure: built from a `Snapshot` and a `Sample`, tested with fixtures.
