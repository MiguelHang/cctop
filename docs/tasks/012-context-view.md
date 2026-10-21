# 012 — Vista de contexto

- **Estado**: pendiente
- **Depende de**: 007, 010
- **Paralelo con**: 011, 013, 015
- **Estimación**: ~600 líneas (código ~330, tests ~270)

## Objetivo
La pestaña Context muestra qué llena el contexto de la sesión de Claude del pane seleccionado.

## Alcance
- `src/ui/context.rs`: total real y % (con el % de cornercase al lado), barra fijo/dinámico, lista de la parte fija
  (archivos de instrucciones con ruta, system prompt, skills, tools, MCP, agentes) y ranking de archivos, comandos y categorías dinámicos con `≈`; scroll con rueda.
- `src/app/context.rs`: para el pane seleccionado, encontrar su proceso `claude` (el de cornercase si es agente Claude; el proceso cuyo nombre o argv es `claude`), su pid y `CLAUDE_CONFIG_DIR`; pedir el `Breakdown` a `context::Sessions` en el hilo de muestreo solo para ese pane.
- Estados vacíos: sin pane seleccionado, pane sin Claude, sesión no encontrada, transcript sin `usage` (solo estimaciones, sin total).

## Criterios de aceptación
- CA1: snapshot con el fixture 2.1.306: AGENTS.md y DESIGN.md con su peso y `≈`, total sin `≈`.
- CA2: cambiar de pane cambia de sesión; volver a Processes conserva la selección.
- CA3: cada estado vacío tiene su texto (snapshots).
- CA4: solo se lee el transcript del pane seleccionado (test con `Sessions` fake o contador).
- CA5 (manual): en cornercase real el total coincide ±2 puntos con el % del pane.

## Tests requeridos
- Snapshots en `ui/tests.rs` (o `ui/context/tests.rs`), `app/context/tests.rs` con `TempDir` como `CLAUDE_CONFIG_DIR`.

## Design a actualizar
- `design/tui.md` y `design/context.md` (cómo se elige el proceso `claude` de un pane).
