# Config

Code: `config/`, `paths.rs`

- **One file**, `config.toml` in the config folder. All keys are optional; this file is the only place their defaults live:

  | Key | Default | Meaning |
  |---|---|---|
  | `interval` | `2s` | The TUI's refresh. |
  | `memory_threshold` | `4GiB` | Memory per pane that alerts. |
  | `cpu_threshold` | `300` | CPU % per pane that alerts once sustained. |
  | `cpu_sustain` | `60s` | How long the CPU must stay over its threshold. |
  | `alert_interval` | `5s` | How often the `watch` service samples. |
  | `sort` | `memory` | The TUI's starting sort: `memory` or `cpu`. |

- **Units**: sizes are strings with a unit, decimal (`B`, `kB`, `MB`, `GB`, `TB`) or binary (`KiB`, `MiB`, `GiB`, `TiB`), case-insensitive, or a bare integer of bytes. Durations are strings with `ms`, `s`, `m`/`min` or `h`, or a bare number of seconds. CPU is a number or a string such as `"300%"`. A size or duration string without a unit is rejected rather than guessed. Zero and negative values are invalid, and durations must be between `100ms` and `24h`: a shorter interval would spin the loop as a zero one would, a zero threshold would always alert, and a longer one is a typo rather than a choice (a refresh or a sustain of a day says nothing) and could overflow the clock when added to it.
- **Never stops cctop from starting**: a missing file means the defaults with no warning; an unreadable file or one that is not TOML means the defaults and one warning, which names the file's real path and, for bad TOML, the line; an invalid value keeps that key's default and adds one warning naming the key; unknown keys are ignored silently, so a newer file works with an older cctop. Warnings are returned, not printed: the caller decides where to show them.
- **Folders** (`paths.rs`), with the environment passed in so tests do not touch the real one. Config: `CORNERCASE_PLUGIN_CONFIG`, else `$XDG_CONFIG_HOME/cctop`, else `~/.config/cctop`. State: `CORNERCASE_PLUGIN_STATE`, else `$XDG_STATE_HOME/cctop`, else `~/.local/state/cctop`. Empty or relative values count as unset, as the XDG spec says.
- `Sort` is the config's own type; `app` maps it to the usage tree's, so `config` stays a leaf domain.
