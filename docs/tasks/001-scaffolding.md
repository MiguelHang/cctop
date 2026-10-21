# 001 — Scaffolding

- **Estado**: pendiente
- **Depende de**: —
- **Paralelo con**: nada (todas dependen de esta)
- **Estimación**: ~650 líneas (Cargo/config ~120, layers.rs ~300, CI ~80, resto ~150)

## Objetivo
Un crate que compila en Linux y macOS con todas las reglas de AGENTS.md comprobadas por máquinas,
y la estructura de módulos fijada para que las tareas paralelas no se pisen en `lib.rs`.

## Alcance
- `Cargo.toml`: crate `cctop` 0.1.0, edition 2024, `[lib]` + `[[bin]]`. `[lints]` copiado de
  `../cornercase/Cargo.toml` (`[lints.rust]` y `[lints.clippy]`, sin los
  `[profile.dev.package.*]` de syntect/image). `[profile.release]` con `lto = "thin"`, `strip = true`.
  Dependencias solo las que use esta tarea: `clap` (derive), `anyhow`, `thiserror`; dev: `rstest`, `tempfile` si hace falta para `TempDir`.
- `clippy.toml` (40 líneas, 5 argumentos, nesting 4, complejidad 10), `rustfmt.toml`
  (`max_width = 120`, `use_small_heuristics = "Max"`), `rust-toolchain.toml` (`1.99.0`, clippy + rustfmt), `.jscpd.json` (copiar el de cornercase).
- `src/lib.rs` declarando **todos** los módulos de AGENTS.md: `cornercase`, `procs`, `usage`, `context`,
  `alerts`, `config`, `ui`, `app`, `error`, `format`, `paths`, y `#[cfg(test)] test_util`. Cada dominio
  es `src/<dominio>/mod.rs` vacío. Exporta `pub fn run_tui()`, `pub fn run_watch()`, `pub fn run_open()`
  que de momento devuelven `Ok(())` (las rellenan 010, 013 y 014).
- `src/error.rs`: `Error` con `thiserror` (variantes `Io`, `Json`, `Toml` se añaden cuando se usen; arranca con `Io`) y `Result<T>`.
- `src/main.rs`: clap con subcomando opcional (`None` → TUI, `watch`, `open`), `--version`, `anyhow` aquí solo.
- `src/test_util.rs` (solo `cfg(test)`): `fixture!` (lee `tests/fixtures/<ruta>` con `env!("CARGO_MANIFEST_DIR")`),
  `wait_until` (polling con timeout), `TempDir` canónico.
- `tests/layers.rs` adaptado del de cornercase (`../cornercase/tests/layers.rs`):
  tabla de capas de AGENTS.md (hojas: `error`, `format`, `paths`, `test_util`; `cornercase`, `procs`,
  `context`, `config` → hojas; `usage` → `cornercase`, `procs`; `alerts` → `cornercase`, `procs`,
  `usage`, `config`; `ui` → dominios + hojas, nunca `app`; `app` → todo; nadie salvo `main.rs` usa `app`),
  y 1.000 líneas por fichero en `src/` y `tests/`. Listas de excepciones vacías. Un módulo desconocido falla.
- `.github/workflows/ci.yml`: job `check` en `ubuntu-latest` y `macos-latest` (clippy `-D warnings` + `cargo test --locked`,
  toolchain leída de `rust-toolchain.toml`, `Swatinem/rust-cache`), job `guardrails` en Linux (`cargo fmt --check`, `cargo-machete`, `npx -y jscpd@4.3.0`).
- `CHANGELOG.md` con `## 0.1.0` (ver "Versiones durante el MVP" en ROADMAP).
- `cornercase-plugin.toml` mínimo: `id = "cctop"`, `name`, `version = "0.1.0"`, `api = 1`, `description`,
  `platforms = ["linux", "macos"]`, `permissions = ["read", "notify", "open"]`, `[[build]] command = ["cargo", "build", "--release", "--locked"]`.
  Sin acciones ni servicios aún (los añaden 013 y 014). Formato: `src/plugins/manifest.rs` de cornercase.
- `tests/fixtures/` con un `.gitkeep`; `.gitignore` ya existe.

## Criterios de aceptación
- CA1: `cargo build`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo test --locked`, `cargo fmt --check`, `cargo-machete` y jscpd pasan.
- CA2: `cargo run -- --help` lista `watch` y `open`; `cargo run -- watch` y `cargo run -- open` salen con 0.
- CA3: `tests/layers.rs` falla si se añade `use crate::app` en `src/procs/mod.rs` o `use crate::ui` en `src/usage/mod.rs` (comprobado a mano y con un test unitario dentro de `layers.rs` sobre fuentes sintéticas).
- CA4: `tests/layers.rs` falla con un fichero de 1.001 líneas (test sobre fuente sintética).
- CA5: `cornercase plugin link .` acepta el manifiesto (comprobación manual, opcional en el agente).
- CA6: el workflow es YAML válido y usa la toolchain fijada.

## Tests requeridos
- Tests de `layers.rs` (capas, tamaño, módulo desconocido) que fallan sin la comprobación.
- Test de `main.rs`/clap: `Cli::command().debug_assert()`.

## Design a actualizar
- Ninguno salvo que cambie alguna capa; si `layers.rs` difiere de la tabla de AGENTS.md, actualizar AGENTS.md.

## Notas
- Las pruebas de integración solo ven lo `pub` de `lib.rs`; todo lo demás `pub(crate)`.
- Sin comentarios en el código. Commits Conventional, sin `Co-Authored-By`.
