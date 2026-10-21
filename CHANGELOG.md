# Changelog

## 0.1.0

The first release: a `btop` for cornercase, as a plugin for Linux and macOS (x86_64 and aarch64).

- **Processes view**: every project, workspace (by name or branch, marked when it is a worktree), tab and pane of the cornercase server, with the process tree under each pane. Memory is the same figure cornercase uses (what a process holds on its own, not RSS) and CPU % is measured between refreshes; both are totalled at every level, so a folded row still shows what it hides. Panes also show their agent, status and context %. Processes that clean their environment still land in their pane through their parent, and panes of another cornercase server are left out.
- **The costliest path** of each pane is highlighted, from the pane's first process down to the one that uses the most memory (or CPU, when sorting by CPU).
- **Sorting** by memory, by CPU or in cornercase's own order, by clicking a column or with `s`.
- **Killing** a process or its whole group with `k` or a right click; it always asks first, naming the process, its pid, its group and its pane, and sends `SIGTERM`. cctop never offers to kill itself, and it does not signal a process that ended or whose pid was reused meanwhile.
- **Context view**: for the Claude session in the selected pane, the real total of tokens and its share of the context window next to cornercase's own %, a bar of fixed against dynamic context, and the parts behind each: the system prompt, every instruction file (`CLAUDE.md`, `AGENTS.md` and their imports), skills, tools, MCP and agents, then the files, commands, searches, prompts and replies since the last compaction, largest first. Per-part figures are estimates and marked `≈`. The transcript is read incrementally, so following a long session stays cheap. Sessions under other Claude accounts (`CLAUDE_CONFIG_DIR`) are found too.
- **Alerts**: the plugin's background service sends a cornercase notification when a pane's processes go over a memory threshold (4 GiB by default) or stay over a CPU threshold (300 % for 60 s by default), naming the pane, its total and its heaviest process. It warns once per crossing and again only after the pane drops well below the threshold.
- **The `cctop` action** in cornercase's menu, in every context, focuses a project named `cctop` with a tab running cctop, creating them when they are missing.
- **Mouse first**: click to select, fold and unfold, sort and switch views; the wheel scrolls. Arrows, `PgUp`/`PgDn`, `Enter`, `Tab`, `s`, `k` and `q` work too.
- **`config.toml`** for the refresh interval, the starting sort and the alert thresholds. A bad value keeps its default and shows a warning; cctop always starts.
- When cornercase stops answering, cctop keeps the last tree on screen, says how long it has been waiting and stays responsive.
