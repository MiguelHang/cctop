# Design decisions

Why cctop works the way it does, area by area. The rules are in [AGENTS.md](AGENTS.md). Each area's file says what was decided and why, what was rejected, and limits that are not obvious; never how a change was built.

## Areas

- [Cornercase client](design/cornercase-client.md) (`cornercase/`): which commands, mapping processes to panes by `boot`, what the core could expose later.
- [Processes](design/processes.md) (`procs/`): the memory metric, CPU %, reading the environment on Linux and macOS.
- [Usage tree](design/usage-tree.md) (`usage/`): the hierarchy, totals, processes without a pane, sorting.
- [Context](design/context.md) (`context/`): finding the session, the transcript's records, fixed vs dynamic, estimating and scaling tokens.
- [Config](design/config.md) (`config/`, `paths.rs`): the keys and their defaults, units, tolerating bad files, the config and state folders.
- [Alerts](design/alerts.md) (`alerts/`): thresholds, hysteresis, the service.
- [Release check](design/release.md) (`release/`): asking GitHub for a newer version once a day, the notice on the footer.
- [TUI](design/tui.md) (`ui/`, `app/`): views, mouse and keys, refresh, killing.
- [Plugin](design/plugin.md) (`cornercase-plugin.toml`, `cctop open`): the action, the "cctop" project, distribution.
