# Cornercase client

Code: `cornercase/`

- **Only public commands**, run through `$CORNERCASE_BIN` (set for every plugin command; `cornercase` on the `PATH` when run by hand): `status --json` for the hierarchy, `open <path> --focus` and `new-tab --workspace ID --name cctop -- cctop` for the action, `focus`, `notify`. No socket protocol of our own: the commands are cornercase's stable contract (`CONTRACT` 1, additive), and a newer cornercase keeps answering them.
- **`status --json` rather than `status --panes`**: the TSV is flat and lacks the tree's order of tabs and the project paths the action needs; the JSON gives both, plus `boot`. Parsing is tolerant: unknown fields ignored, every optional field `Option`, so a newer cornercase never breaks it.
- **Mapping processes to panes**: every shell cornercase starts gets `CORNERCASE_PANE=<id>` and `CORNERCASE_SERVER=<token>`, and every descendant inherits them. The token is `<server pid>-<start ns>`, the same string as `boot` in `status --json`. A process belongs to pane N of this server when both match; a pane id alone is not enough, since a second server (a debug build of cornercase) numbers its panes from 1 too. A process that cleared its environment is attributed through its nearest ancestor that has them.
- **Runner**: the client runs commands through a `Runner` trait (argv in, stdout or error out), so tests answer with fixtures and never start cornercase.
- **For a later pull request to the core**: `pid` (the pane's shell) and `memory` (the figure the tab row already computes) as optional fields of each pane in `status`. With them cctop would not need to read other processes' environment (the hard part on macOS), and other clients would get the memory too.
