# 007 — Contexto: localizar la sesión y lectura incremental

- **Estado**: hecho
- **Depende de**: 006 (usa `Accumulator`)
- **Paralelo con**: 002, 003, 004, 005, 008, 009, 014
- **Estimación**: ~500 líneas (código ~250, tests ~250)

## Objetivo
Del pid de un `claude` y su `CLAUDE_CONFIG_DIR`, encontrar su transcript y mantener su `Breakdown`
leyendo solo lo nuevo en cada tick.

## Alcance
- `src/context/session.rs`: `<config>/sessions/<pid>.json` → `sessionId`, `cwd` (releído en cada tick);
  carpeta del proyecto con la regla de cornercase (no alfanuméricos → `-`; > 200 caracteres cortado + `-` + hash), y si no existe,
  buscar `<sessionId>.jsonl` listando `projects/`. Config por defecto `~/.claude` (con `HOME` inyectado).
- `src/context/reader.rs`: `TranscriptReader` con offset y metadatos; tick tranquilo = un `stat`; fichero más corto o
  reemplazado (inodo) → relee desde el principio con un acumulador nuevo; línea final incompleta se guarda hasta el siguiente tick; cambio de `sessionId` → nuevo lector.
- `src/context/mod.rs`: `Sessions` que cachea un lector por pid (y olvida los pids que ya no se piden).

## Criterios de aceptación
- CA1: con un `CLAUDE_CONFIG_DIR` temporal (sesión + transcript del fixture de 006) se encuentra y se obtiene el `Breakdown`.
- CA2: rutas de más de 200 caracteres se resuelven por el listado de `projects/`.
- CA3: añadir líneas al transcript actualiza el `Breakdown` leyendo solo la cola (el test comprueba el offset).
- CA4: truncar o reemplazar el fichero hace que se relea desde cero.
- CA5: una línea a medio escribir no se pierde ni se parsea dos veces.
- CA6: sesión o transcript ausentes → `None`, no error.

## Tests requeridos
- `context/session/tests.rs`, `context/reader/tests.rs` con `TempDir`; nunca el `~/.claude` real.

## Design a actualizar
- `design/context.md` si cambia algo de la localización o la relectura.
