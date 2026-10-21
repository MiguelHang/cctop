# 005 — Configuración

- **Estado**: pendiente
- **Depende de**: 001
- **Paralelo con**: 002, 003, 004, 006, 007, 008, 009
- **Estimación**: ~250 líneas (código ~110, tests ~140)

## Objetivo
`config.toml` con valores por defecto, que nunca impide arrancar.

## Alcance
- `src/config/mod.rs`: `Config { interval, memory_threshold, cpu_threshold, cpu_sustain, alert_interval, sort }`
  con defaults (2 s, 4 GiB, 300 %, 60 s, 5 s, memoria). Tamaños legibles (`"4GiB"`, `"512MB"`) y duraciones (`"2s"`).
- `src/paths.rs`: carpeta de configuración (`CORNERCASE_PLUGIN_CONFIG`, si no `$XDG_CONFIG_HOME/cctop`, si no `~/.config/cctop`) y de estado (`CORNERCASE_PLUGIN_STATE`, si no `~/.local/state/cctop`), con el entorno inyectado para testear.
- Fichero ausente → defaults; clave desconocida ignorada; valor inválido → default de esa clave y un aviso devuelto (no error).
- Dependencias: `serde`, `toml`.

## Criterios de aceptación
- CA1: sin fichero, `Config::default()`.
- CA2: un fichero parcial cambia solo sus claves.
- CA3: un valor inválido (`interval = "x"`) deja el default de esa clave y devuelve un aviso.
- CA4: `paths` resuelve las tres precedencias.

## Tests requeridos
- `config/tests.rs` y `paths/tests.rs` (o `tests` dentro), rstest para tamaños y duraciones.

## Design a actualizar
- Nuevo `design/config.md` (claves, defaults, tolerancia) y su línea en `DESIGN.md`; quitar de `design/alerts.md` los defaults duplicados si los hay (una sola fuente).
