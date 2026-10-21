# cctop

A `btop` for [cornercase](https://usecornercase.dev), shipped as a cornercase plugin.

- **Processes**: one view of every project, workspace (and its worktree), tab and pane of the cornercase server, with the process tree under each pane and the memory and CPU it uses, totalled at every level. Panes show their agent, status and context %, and the costliest path of each pane is highlighted down to the process that weighs most. Sort by memory or CPU, fold what you do not need, and kill a runaway process or its group (it always asks first).
- **Context**: for the Claude session in the selected pane, what fills its context: the fixed part (`CLAUDE.md` and its imports, skills, tools, the system prompt) and the files and commands read since the last compaction, against the real total Claude reports and the % cornercase shows. Estimated numbers are marked `≈`.
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

Pick **cctop** from cornercase's menu, in any project, workspace, tab or pane. It focuses a project named `cctop` with a tab running cctop, creating them when they are missing, so there is always one cctop and always in the same place.

You can also run `cctop` in any terminal while cornercase runs; it asks the server through `$CORNERCASE_BIN`, or `cornercase` on the `PATH`.

The mouse comes first: click a row to select it, its arrow to fold or unfold it, a column's header to sort by it (`Name` goes back to cornercase's order) and a view's name at the top to switch to it; right-click a process to kill it; the wheel scrolls. Keys:

| Key | Does |
|---|---|
| `↑` `↓`, `PgUp` `PgDn` | move the selection |
| `Enter`, `←` `→` | fold or unfold |
| `Tab` | switch between the processes and context views |
| `s` | change the sort: memory, CPU, cornercase's order |
| `k` | kill the selected process or its group (asks first) |
| `q` | quit |

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
