# Roadmap — cctop

Producto y alcance: [IDEA.md](IDEA.md). Reglas: [../AGENTS.md](../AGENTS.md). Decisiones: [../DESIGN.md](../DESIGN.md).
Cada tarea de `tasks/` es una PR pequeña y verificable. El MVP está hecho cuando se cierra el hito M5.

## Hitos del MVP (en orden)

### M0 — Base (001)
Repositorio que compila en Linux y macOS con las reglas de AGENTS.md aplicadas por máquinas: lints en
`Cargo.toml`, `clippy.toml`, `rustfmt.toml`, toolchain 1.99.0, `.jscpd.json`, `tests/layers.rs`
(capas y 1.000 líneas), `error.rs`, `lib.rs` con los módulos de dominio vacíos, `main.rs` con clap
(`cctop` = TUI, `cctop watch`, `cctop open`, sin hacer nada aún), helpers de test, CI de GitHub Actions,
`CHANGELOG.md` y un `cornercase-plugin.toml` mínimo que se instala con `[[build]]`.
- **Entrega**: `cornercase plugin link .` lo acepta; CI en verde.

### M1 — Dominios de datos (002–008)
Todo lo que no es UI, puro o detrás de traits, probado con fixtures:
- 002 `procs` en Linux (trait `ProcessSource`, `Sample`, CPU %).
- 003 `procs` en macOS (`libproc` + `KERN_PROCARGS2`).
- 004 `cornercase`: `Runner`, `status --json` → `Snapshot`, comandos `open`/`new-tab`/`focus`/`notify`.
- 005 `config`: `config.toml` con valores por defecto.
- 006 `context`: parseo tolerante del transcript y `Breakdown` (puro).
- 007 `context`: localizar la sesión de un pane y leer el transcript de forma incremental.
- 008 `usage`: árbol proyecto → workspace → pestaña → pane → procesos, totales, orden, camino caliente.
- **Entrega**: un test de integración construye el árbol completo a partir de fixtures reales anonimizados.

### M2 — Vista de procesos (009–011)
- 009 `ui`: render de la vista Processes desde un `View` + áreas de clic (snapshots).
- 010 `app`: bucle, hilo de muestreo cada `interval`, selección/plegado/scroll con ratón y teclas, orden.
  `cctop` ya es usable.
- 011 matar un proceso o su grupo con confirmación.
- **Entrega**: criterio de éxito 1 (un `cargo test` de varios GB aparece bajo su pane y worktree en < 5 s).

### M3 — Vista de contexto (012)
- 012 pestaña Context: total real, barra fijo/dinámico, top archivos y comandos con `≈`.
- **Entrega**: criterio de éxito 2 (AGENTS.md/DESIGN.md con su peso; total ±2 puntos del % de cornercase).

### M4 — Alertas (013)
- 013 histéresis pura + `cctop watch` + `[[service]]` en el manifiesto.
- **Entrega**: el aviso del criterio de éxito 1 llega por `cornercase notify`.

### M5 — Plugin y distribución (014–017)
- 014 `cctop open` y la acción `cctop` en todos los contextos.
- 015 workflow de release con binarios para los cuatro targets y `[download.<target>]` en el manifiesto.
- 016 validación del MVP en cornercase real (Linux; macOS vía CI) y medida de CPU en reposo.
- 017 CPU en reposo: muestra de `/proc` más barata, `status` cada 10 s, dibujar solo al cambiar.
- **Entrega**: `cornercase plugin install MiguelHang/cctop` instala el binario descargado y todo lo anterior funciona.

### M6 — Tras la primera prueba del usuario (018–019)
- 018 aspecto tipo btop (gráficas, barras, sparklines) y Context con su propia lista de agentes.
- 019 la acción abre cctop en un split del pane y se cierra al salir, sin proyecto "cctop".

## Grafo de dependencias y oleadas paralelas

```
001 ─┬─ 002 ─┬─ 003
     │       └─────────┐
     ├─ 004 ─┬─────────┼─ 008 ─┬─ 010 ─┬─ 011
     │       │         │       │       └─ 012 (también 007)
     │       └─ 014 ─ 015      └─ 013 (también 005, 004)
     ├─ 005 ───────────────────── 010, 013
     ├─ 006 ─ 007 ───────────────── 012
     └─ 009 ─────────────────── 010
                                         016 (al final, todo)
```

| Oleada | Tareas en paralelo | Requisito |
|---|---|---|
| 1 | 001 | — |
| 2 | 002 ∥ 004 ∥ 005 ∥ 006 ∥ 009 | 001 |
| 3 | 003 ∥ 007 ∥ 008 ∥ 014 | 002 / 006 / 002+004 / 004 |
| 4 | 010 ∥ 013 ∥ 015 | 008+009+005 / 008+005 / 014 |
| 5 | 011 ∥ 012 | 010 (012 también 007) |
| 6 | 016 | todo |
| 7 | 017 | 016 (medida) |
| 8 | 018 ∥ 019 | 017 / 014 |

Ficheros compartidos que todas tocan (fuente de conflictos al trabajar en paralelo): `Cargo.toml`
(dependencias), `Cargo.lock`, `CHANGELOG.md`, `src/lib.rs`. 001 declara ya todos los módulos para
que `lib.rs` no cambie; cada tarea añade solo sus dependencias. Ver "Versiones durante el MVP".

## Versiones durante el MVP

`version = "0.1.0"` desde 001 y una sola sección `## 0.1.0` en `CHANGELOG.md` a la que cada tarea
añade sus líneas; no se sube versión hasta la primera release (015). Tras 0.1.0 rige la regla de
AGENTS.md (cada cambio de la app sube versión en `Cargo.toml` y `cornercase-plugin.toml`).

## Más adelante (fuera del MVP, de IDEA.md)

- Desglose de contexto para Codex, Gemini, opencode y demás agentes (la vista de procesos ya los muestra).
- Histórico y gráficas de memoria y CPU en el tiempo (y persistirlo).
- PR al core de cornercase: `pid` y `memory` por pane en `status` (ver `design/cornercase-client.md`),
  para no leer el entorno de procesos ajenos.
- Un panel global dentro de la UI de cornercase (exige trabajo en el core).
- Recuento exacto de tokens con la API `count_tokens`.
- Atribuir un proceso al subagente concreto de Claude que lo lanzó.
