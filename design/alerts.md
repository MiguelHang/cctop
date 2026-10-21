# Alerts

Code: `alerts/`

- **The service** (`cctop watch`, the plugin's `[[service]]`, kept running by the cornercase server) samples every `alert_interval` and checks each pane's tree.
- **Thresholds** (`config.toml`, defaults in [config](config.md)): memory per pane and CPU per pane sustained for `cpu_sustain`. A pane alerts once when it crosses, then not again until it drops below 80 % of the threshold (hysteresis), so a tree hovering on the line does not spam.
- **The message** names the pane's place and the heaviest process: `cctop: feat/x › claude — cargo test 5.2 GB`, sent with `cornercase notify` (the window's toast and a desktop notification).
- The decision logic is pure (state per pane + sample → alerts) and tested on its own; the loop only samples, decides and runs `notify`.
