# Plugin

Code: `cornercase-plugin.toml`, `cctop open`

- **One global place**: plugins cannot draw inside cornercase's UI and every tab lives in a workspace, so the global view is a project of its own, **cctop**, whose folder is the plugin's state folder (`CORNERCASE_PLUGIN_STATE`). The action `cctop` (in every context: project, workspace, tab, pane) runs `cctop open`: `cornercase open <state folder> --focus` finds or opens that project, and a tab running `cctop` is added only when none of its tabs runs it already. Closing the project is fine: the action makes it again. A panel inside cornercase was left for the core.
- **Services**: `[[service]] watch` runs `cctop watch` (alerts). The TUI is not a `[[window]]` process: those have no terminal.
- **Distribution**: `[download.<target>]` archives for `x86_64`/`aarch64` Linux and macOS with their sha256, built by a release workflow; `[[build]] cargo build --release` as the fallback. Published with the `cornercase-plugin` topic.
- **Permissions** declared: `read` (status), `send` is not needed; `notify` and `open` are listed so the preview is honest.
