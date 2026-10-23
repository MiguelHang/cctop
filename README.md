# cctop

A `btop` for [cornercase](https://usecornercase.dev), shipped as a cornercase plugin.

- **Processes**: one view of every project, workspace (and its worktree), tab and pane of the cornercase server, with the process tree under each pane and the memory and CPU it uses, totalled at every level, under graphs of the last hour and with a memory bar and a minute of CPU per row. Panes show their agent, status and context %, and the costliest path of each pane is highlighted down to the process that weighs most. Sort by memory or CPU, fold what you do not need, and kill a runaway process or its group (it always asks first).
- **Context**: every Claude session of the server, fullest first, and for the one you pick, how full its context is (the real total Claude reports, its share of the window, a bar in green, yellow or red), what is loaded on every message (`CLAUDE.md` and its imports, skills, tools, the system prompt) and what fills this conversation since the last compaction, grouped into commands, files, searches, Claude's replies, your messages and the rest, each with its three largest parts. It also says how much `/compact` would free and names a single part that takes over a tenth of the conversation. The split between parts is estimated from the transcript; the total is Claude's own.
- **Alerts**: a background service that warns with a cornercase notification when a pane's processes cross a memory threshold, or stay over a CPU threshold for a while.

It runs on Linux and macOS and uses only cornercase's public commands.

## Install

```sh
cornercase plugin install MiguelHang/cctop
cornercase plugin enable cctop
```

Enabling it also starts the alerts service.

cornercase shows what the plugin runs and asks before installing. When the installed version has a release archive for your machine (Linux or macOS, x86_64 or aarch64) it downloads it and checks its sha256; otherwise it builds it with `cargo build --release --locked`, which needs a Rust toolchain.

## Use

Click the **cctop** button (`◔` in the icon bar), at the foot of the projects column, or pick **cctop** from cornercase's menu in any project, workspace, tab or pane. It opens cctop in a popup over the window, already pointing at the pane on screen (or at what you clicked): its path is unfolded and it is selected, so `Tab` shows its Claude's context. `q` or `Esc` closes it. It needs cornercase 0.12.187 or newer.

To keep cctop open beside your work, pick **cctop in a split** from a pane's menu instead. It opens cctop in a split to the right of that pane and focuses it; when that tab already shows cctop, it focuses that one instead. Quitting cctop (`q`) closes its pane.

cctop leaves itself out of the tree, so it never adds to the pane it was opened from. You can also run `cctop` in any terminal while cornercase runs; it asks the server through `$CORNERCASE_BIN`, or `cornercase` on the `PATH`.

The mouse comes first: click a row to select it, its arrow to fold or unfold it, a column's header to sort by it (`Name` goes back to cornercase's order) and a view's name at the top to switch to it; right-click a process to kill it; the wheel scrolls. Keys:

| Key | Does |
|---|---|
| `↑` `↓`, `PgUp` `PgDn` | move the selection (in Context, choose the agent) |
| `Enter`, `←` `→` | fold or unfold (in Context, `Enter` opens or folds every group) |
| `Tab` | switch between the processes and context views |
| `s` | change the sort: memory, CPU, cornercase's order |
| `k` | kill the selected process or its group (asks first) |
| `q`, `Esc` | quit (closes the popup) |

In the kill dialog, `←` `→` or `Tab` pick `Cancel`, `Process` or `Group`, `Enter` confirms and `Esc` cancels. cctop sends `SIGTERM`, and never offers to kill itself.

## Configure

cctop reads `config.toml` from the plugin's config folder (`$XDG_CONFIG_HOME/cctop/config.toml`, by default `~/.config/cctop/config.toml`, when run outside cornercase). Every key is optional:

```toml
interval = "2s"            # how often the view refreshes
sort = "memory"            # starting sort: "memory" or "cpu"
memory_threshold = "4GiB"  # memory per pane that alerts
cpu_threshold = 300        # CPU % per pane that alerts once sustained
cpu_sustain = "60s"        # how long the CPU must stay over its threshold
alert_interval = "5s"      # how often the alerts service samples
```

Sizes take a unit (`MB`, `GiB`…) and durations `ms`, `s`, `m` or `h`. A bad value keeps its default and shows a warning; cctop always starts. The alerts service reads the file when it starts, so a change reaches it the next time cornercase restarts it.

## Build from source

```sh
cargo build --release --locked
cornercase plugin link .
cornercase plugin enable cctop
```

## License

MIT or Apache-2.0, at your option: [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE).
