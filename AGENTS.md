# cctop

A `btop` for [cornercase](https://usecornercase.dev), shipped as a cornercase plugin. One global view of every project, workspace (and its worktree), tab and pane of the cornercase server, with the process tree under each pane and what it costs in memory and CPU; a second view breaks down what fills the context of the Claude session in the selected pane (fixed instructions such as `CLAUDE.md` and its imports, and the files and commands read since the last compaction). A background service warns with `cornercase notify` when a pane's tree crosses a threshold.

The product decisions are in [docs/IDEA.md](docs/IDEA.md), the plan in [docs/ROADMAP.md](docs/ROADMAP.md) and `docs/tasks/`. The design decisions and their reasons are in `design/`, one file per area, listed in [DESIGN.md](DESIGN.md): read the files for the areas you touch before changing them, and update them in the same change when a decision changes.

## Commands

```sh
cargo run                                   # the TUI (needs a running cornercase server)
cargo run -- watch                          # the alerts service, as the plugin runs it
cargo run -- open                           # the menu action: focus or create the "cctop" project
cargo test --locked                         # unit + snapshot + integration tests
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo fmt
cargo-machete                               # unused dependencies
npx -y jscpd@4.3.0                          # copy-paste detector, reads .jscpd.json
.github/versions.sh                         # Cargo.toml and the manifest agree on the version
cornercase plugin link .                    # try it in the real cornercase, then `cornercase plugin enable cctop`
```

- **Before calling a task done, run fmt, clippy, tests, `cargo-machete` and jscpd, and fix what fails.** If one cannot pass, say so.
- The toolchain is pinned in `rust-toolchain.toml`; bump it by hand and fix what clippy reports.
- Snapshots (`insta`): a changed render writes a `.snap.new` and fails. Check it, then accept with `INSTA_UPDATE=always cargo test`. Delete snapshots of renamed or removed tests by hand.
- **Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)**: `<type>: <summary>`, imperative and lower case (`feat: show cpu per pane`). Types: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `ci`, `build`, `chore`. **No `Co-Authored-By` trailers or "Generated with" lines**, in commits or pull requests.
- A change to the app bumps `version` in `Cargo.toml` and `cornercase-plugin.toml` together, removes the manifest's `[download.*]` tables (the release workflow writes them back, see [design/plugin.md](design/plugin.md)) and adds a `## <version>` section to `CHANGELOG.md`, written for users.

## Code conventions

- Everything in English: identifiers, test names, messages, UI text. Docs in `docs/` may be in Spanish.
- **No comments**, including `///` and `//!`. Rationale goes in the area's file in `design/`.
- Errors: `crate::error::Error` (`thiserror`) in the library; `anyhow` only in `src/main.rs`.
- No `unwrap()` outside tests (`clippy::unwrap_used`); in tests prefer `expect("what")`. Never panic on input from outside (cornercase's JSON, `/proc`, transcripts): unknown fields are ignored, missing ones become `None` and show as `—`.
- Lints live in `Cargo.toml` (clippy `all` deny, `pedantic` warn), as cornercase's. Silence one only locally with `#[expect(clippy::x, reason = "...")]`.
- **Small functions**: `clippy.toml` limits a function to 40 lines, 5 arguments (`self` counts), two levels of nesting inside a method (`excessive-nesting-threshold = 4`) and a cognitive complexity of 10. Split the function; never silence these four.
- Imports in three groups: `std`, external crates, then `super`/`crate`.
- `rustfmt.toml`: `max_width = 120`, `use_small_heuristics = "Max"`.
- At most 1,000 lines per file.
- **`pub` only for what `src/main.rs` or the integration tests use**; everything else is `pub(crate)` or narrower (`unreachable_pub`).

## Where code goes

By domain. Each domain is a module with its own types and tests, and knows nothing of the UI:

| Module | Holds | May use |
|---|---|---|
| `cornercase/` | The cornercase client: runs `$CORNERCASE_BIN` (`status --json`, `split`, `focus`, `notify`) and parses its answers into `Snapshot` (projects, workspaces, tabs, panes, `boot`). | leaves |
| `procs/` | The `ProcessSource` trait, `linux.rs` (`/proc`) and `macos.rs` (`libproc`, `sysctl`), one `Sample` of every process of the user (pid, parent, name, arguments, memory, CPU time, `CORNERCASE_PANE`/`CORNERCASE_SERVER`), CPU % between two samples. | leaves |
| `usage/` | Joins a `Snapshot` and a `Sample` into the tree project → workspace → tab → pane → processes, with totals per level and sorting. Pure. | `cornercase`, `procs`, leaves |
| `context/` | Claude sessions: finding a pane's session and transcript, reading the transcript incrementally, the `Breakdown` (fixed and dynamic parts, per file and command, the real total). | leaves |
| `alerts/` | Thresholds with hysteresis (pure) and the `watch` service loop. | `cornercase`, `procs`, `usage`, `config`, leaves |
| `config/` | `config.toml` (interval, thresholds, default sort), with defaults. | leaves |
| `ui/` | Rendering only: functions from a `View` (plain data) to ratatui widgets, and the hit areas the mouse uses. | domain types, leaves |
| `app/` | The TUI's state, input (mouse first, then keys), the loop and the glue between domains and `ui`. Only `app` and `main.rs` use `app`. | everything |
| leaves | `error.rs`, `format.rs` (sizes, percentages, durations), `paths.rs`. | nothing of the crate |

- `ui` never calls a domain's I/O; `app` builds a `View` and hands it to `ui`.
- Domains never use `app` or `ui`. `tests/layers.rs` checks the imports and the file sizes; its lists of exceptions only shrink.
- I/O sits behind small traits (`ProcessSource`, `Runner` for the cornercase binary, `Clock`) so the logic is tested with fakes.

## Interaction

- Mouse first, like cornercase: click selects a row, folds and unfolds it, switches views and sorts by a column; the wheel scrolls. Keys in its own pane: arrows, `Tab` (view), `s` (sort), `k` (kill, asks first), `q` (quit).
- Killing a process always asks first, naming the process and its pane.
- Estimated numbers show `≈`; real ones (the context total from Claude's `usage`) do not.

## Tests

- The tests of `foo.rs` (or `foo/mod.rs`) go in `foo/tests.rs`, declared as the last item with `#[cfg(test)] mod tests;`. Tests are `mod <unit_of_work>` groups with sentence-like names; tabular cases use `rstest` with named `#[case::...]`.
- Fixtures in `tests/fixtures/`: `status --json` answers, process samples, Claude transcripts (named by the Claude Code version they came from, with paths and personal data replaced). Read them with a `fixture!` helper, never a relative `include_str!`.
- No test runs the real cornercase, Claude or `kill`: the cornercase client takes a fake `Runner`, processes come from a fake `ProcessSource`. The real `ProcessSource` is tested on the test process itself (its own pid, its own environment, memory it allocates).
- UI: render a `View` into ratatui's `TestBackend`; `insta` snapshots for layout.
- Wait with a polling helper, never with sleeps.
- A safety-net test must fail without the code it protects.

## Plugin

- `cornercase-plugin.toml` at the root: the `cctop` action in every context (runs `cctop open`, which opens cctop in a split), the `watch` service, `[download.<target>]` for Linux and macOS (x86_64, aarch64) and `[[build]]` as a fallback. The cornercase rules for plugins: `design/plugins` in cornercase's repository.
- The plugin uses only cornercase's public commands. What it would need from the core (a pane's `pid` and `memory` in `status`) is noted in `design/cornercase-client.md` for a later pull request.

## Instruction files

`CLAUDE.md` only imports this file and `DESIGN.md`. `DESIGN.md` is the index of `design/`, one line per area; the area files are read on demand. Every rule and decision lives in one place, here or in one file of `design/`.
