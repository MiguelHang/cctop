# Processes

Code: `procs/`

- **The memory metric is cornercase's** (`design/memory.md` there): memory a process holds on its own. Linux: `RssAnon + VmSwap` from `/proc/<pid>/status`. macOS: `ri_phys_footprint` from `proc_pid_rusage` (Activity Monitor's Memory). Neither counts file-backed pages shared by every process (the binary, its libraries), so the sum over a tree is close to what it costs. RSS was rejected: cornercase measured it at about twice the real cost for Claude Code (its 224 MB binary counted once per process). For the same reason `sysinfo` is not the source: it reports RSS.
- **CPU %** is the change in user + system time between two samples divided by the wall time between them, per process (100 % = one core, as `top`). The first sample shows no CPU.
- **Environment**: Linux reads `/proc/<pid>/environ` (only the user's own processes are readable, which are the ones that matter). macOS reads `sysctl` `KERN_PROCARGS2`, which holds the arguments then the environment. Only `CORNERCASE_PANE`, `CORNERCASE_SERVER` and `CLAUDE_CONFIG_DIR` are kept.
- **One sample** lists every process of the user once (pid, parent, name, arguments, memory, CPU time, the kept variables). Processes of other users are skipped. A process that ends while being read is dropped, never an error.
- **Tests** of the real sources run on the test process: its pid is listed, a variable it set in a child is read back, and memory written to an anonymous mapping of 64 MiB raises the figure while a mapped file does not (the check cornercase uses).
