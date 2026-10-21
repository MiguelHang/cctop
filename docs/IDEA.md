# cctop — idea y requisitos

> Un `btop` para cornercase: qué recursos consume cada agente y cada worktree, y qué archivos llenan el contexto de cada sesión de Claude.

## Problema

Con varios agentes en paralelo, cada uno en su worktree, no hay forma rápida de saber:

1. **Quién se está comiendo la máquina.** Un agente lanza `cargo test`, `jest` o un dev server y ese árbol de procesos acapara la memoria o la CPU. `btop` muestra procesos, pero no sabe qué pane, workspace o worktree de cornercase los lanzó. La fila de memoria de cornercase da un total por pestaña, sin desglose ni CPU.
2. **Qué llena el contexto de una sesión.** cornercase muestra el % de contexto usado, pero no de dónde sale. Ejemplo real: `DESIGN.md` de cornercase se cargaba en cada sesión y se comía muchos tokens sin que fuese visible.

## Usuario

Quien trabaja con varios agentes de código en cornercase a la vez (en primer lugar, el autor). Plugin publicable en el marketplace de cornercase (topic `cornercase-plugin`).

## Alcance del MVP

### Vista de procesos (global)
- Árbol **proyecto → workspace (rama, worktree sí/no) → pestaña → pane → árbol de procesos** de todo el servidor, no solo del workspace actual.
- Por proceso y agregado por cada nivel: memoria (la misma métrica que cornercase: `RssAnon + VmSwap` en Linux, `ri_phys_footprint` en macOS) y CPU % (delta de tiempo de CPU entre muestras).
- Por pane: agente, estado, modelo y % de contexto (de `status --panes`), número de subagentes y `parent` (qué pane lanzó a cuál).
- Ordenar por memoria o CPU; resaltar el proceso más caro de cada pane, para ver "claude → bash → cargo test → binario de 4 GB" de un vistazo.
- Matar un proceso o su grupo (`k`, con confirmación).
- Refresco cada 2 s (configurable).

### Vista de contexto (Claude)
- Para el pane seleccionado con Claude: localizar la sesión (`$CLAUDE_CONFIG_DIR/sessions/<pid>.json` → `sessionId`, `cwd` → transcript `projects/<cwd-munged>/<sessionId>.jsonl`), leyendo `CLAUDE_CONFIG_DIR` del entorno del proceso (cuentas de cornercase).
- **Contexto fijo**: cada archivo del attachment `instructions` (CLAUDE.md, AGENTS.md, DESIGN.md, MEMORY.md, imports…) con su ruta y su peso; el listado de skills, las tools diferidas, las instrucciones MCP, el listado de agentes y el system prompt (`prompt_snapshot`).
- **Contexto dinámico** desde el último `compact_boundary`: agrupado por archivo (Read/Edit/Write por `file_path`), por comando (Bash/Grep/Glob) y por categoría (prompts del usuario, respuestas, resultados de subagentes).
- **Total real** del último `usage` del asistente (`input + cache_creation + cache_read`). El desglose es una estimación (caracteres/4), escalada para que sume el total real y marcada como "≈".
- Ranking "top archivos" y separación fijo/dinámico en %.

### Integración con cornercase
- `cornercase-plugin.toml` con:
  - una **acción** `cctop` en todos los contextos (project, workspace, tab, pane) que enfoca un proyecto **"cctop"** en el sidebar o, si no existe, lo crea (carpeta: `CORNERCASE_PLUGIN_STATE`) con una pestaña que ejecuta `cctop`. Una sola instancia y siempre en el mismo sitio.
  - un **`[[service]]`** de alertas: cada N s mide los árboles por pane y, si uno pasa del umbral (por defecto 4 GB) o se mantiene por encima de X % de CPU durante Y s, avisa con `cornercase notify` (una vez por cruce, con histéresis).
  - `[download.<target>]` con binarios para Linux y macOS (x86_64 y aarch64) y `[[build]]` (`cargo build --release`) como alternativa.
- Solo usa la API pública (`$CORNERCASE_BIN status --json` / `--panes`, `open`, `new-tab`, `focus`, `notify`). No necesita cambios en el core.
- El mapeo pane↔proceso se hace por el entorno de cada proceso: `CORNERCASE_PANE=<id>` y `CORNERCASE_SERVER=<token>`, que debe ser igual al `boot` de `status --json` (así se descartan los panes de otro servidor, por ejemplo uno de desarrollo).

### Interacción
- Ratón primero, como cornercase: clic para seleccionar, plegar y desplegar, y cambiar de vista; rueda para hacer scroll.
- Teclas tipo btop dentro de su propio pane: flechas, `Tab` (cambiar de vista), `s` (orden), `k` (matar), `q` (salir).

### Configuración
- `config.toml` en `CORNERCASE_PLUGIN_CONFIG` (o `~/.config/cctop/` si se ejecuta fuera de cornercase): intervalo, umbrales de memoria y CPU, orden por defecto.
- Sin persistencia de histórico en el MVP.

## Fuera de alcance (más adelante)
- Desglose de contexto para Codex, Gemini, opencode y demás agentes (la vista de procesos sí los muestra).
- Histórico y gráficas de memoria y CPU en el tiempo.
- PR al core de cornercase: exponer `pid` y `memory` por pane en `status`, para simplificar el mapeo y quitar la dependencia de leer el entorno de los procesos.
- Un panel global dentro de la propia UI de cornercase (exige trabajo en el core).
- Recuento exacto de tokens (API `count_tokens`).
- Atribuir un proceso al *subagente* concreto de Claude que lo lanzó (los subagentes de Claude corren dentro del mismo proceso).

## Requisitos no funcionales
- Linux y macOS desde el MVP.
- Ligero: menos del 1 % de CPU en reposo con 2 s de refresco y unos 30 panes. El transcript se lee de forma incremental (offset + `stat`), sin reparsear todo en cada tick.
- Solo lectura sobre cornercase y Claude, salvo matar un proceso (con confirmación) y crear o enfocar su proyecto.
- Tolerante a cambios de formato: los campos JSON desconocidos se ignoran y los que faltan se degradan a "—"; nunca hace panic.
- Testeable sin cornercase ni Claude reales: fixtures de `status --panes`, de árboles de procesos y de transcripts.

## Stack
- **Rust estable + ratatui + crossterm**: mismo stack que cornercase, binario único sin runtime, fácil de distribuir con `[download]`, y lo aprendido se lleva al core.
- Procesos: un trait `ProcessSource` con dos implementaciones:
  - **Linux**: `/proc` (`status`, `stat`, `environ`, `children`).
  - **macOS**: `libproc` (`proc_listallpids`, `proc_pid_rusage` para el footprint, `PROC_PIDTASKINFO` para la CPU) y `sysctl KERN_PROCARGS2` para el entorno.
  - Se descarta `sysinfo` como fuente única: su memoria es RSS, que según cornercase sobrestima en torno a ×2.
- `serde_json` para el status y el transcript; `toml` para la configuración.

## Criterios de éxito
- Ante un `cargo test` lanzado por un agente que ocupa varios GB, cctop lo señala en menos de 5 s bajo el pane y el worktree correctos, y el servicio de alertas avisa.
- En una sesión de cornercase, la vista de contexto muestra `AGENTS.md` / `DESIGN.md` con su peso, y el total coincide con el % que enseña cornercase (±2 puntos).
- Funciona en Linux y macOS con el mismo binario por plataforma, instalado con `cornercase plugin install`.

## Riesgos
- **Formato del transcript de Claude**: no es una API estable (attachments `instructions`, `prompt_snapshot`…). Mitigación: parseo tolerante, fixtures versionados por versión de Claude Code (validado con 2.1.306) y degradar a "solo total" si falta algo.
- **macOS sin máquina de pruebas**: `KERN_PROCARGS2` y `libproc` se validan solo en CI (runner macOS) y con tests sobre el propio proceso. Riesgo de bugs que solo aparezcan en uso real.
- **Leer el entorno de procesos ajenos**: solo funciona con procesos del mismo usuario (es el caso). Un proceso que limpie su entorno (`env -i`) cae fuera del pane; se le atribuye por ascendencia (el proceso hijo hereda el pane de su padre).
- **Estimación de tokens**: caracteres/4 es impreciso con código y JSON; escalar al total real lo corrige en conjunto, pero no por archivo.
- **Varias instancias del proyecto "cctop"**: si el usuario lo renombra o lo cierra, la acción lo vuelve a crear; se identifica por su ruta.
