# TUI

Code: `ui/`, `app/`

- **Two views**, switched with tabs at the top (click or `Tab`): **Processes** (the usage tree as a table: name, memory, CPU %, and for panes agent/status/context) and **Context** (the breakdown of the selected pane's Claude session: total and %, fixed vs dynamic bar, then the top files and commands). Selecting a pane in Processes and switching keeps it selected.
- **Mouse first**, like cornercase: click selects a row and folds or unfolds it on its arrow, a header click sorts by that column, the wheel scrolls. Keys exist too, since the pane is cctop's own: arrows, `Enter` (fold), `Tab`, `s`, `k`, `q`.
- **Refresh** every `interval` ([config](config.md)) on a background thread that samples and asks cornercase; the UI loop never blocks on I/O. Context is only read for the selected pane.
- **Killing** (`k`, or the row's menu) asks first with the process, its pid and its pane; it sends `SIGTERM` to the process (or its group, chosen in the dialog). Never without confirming.
- `ui` renders a `View` (plain data built by `app`) and returns hit areas for the mouse, so it is tested with snapshots and no I/O.
