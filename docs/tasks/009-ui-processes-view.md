# 009 — UI: vista de procesos

- **Estado**: pendiente
- **Depende de**: 001
- **Paralelo con**: 002, 003, 004, 005, 006, 007, 008, 014 (solo toca `ui/` y `format.rs`)
- **Estimación**: ~650 líneas (código ~350, tests ~150, snapshots excluidos)

## Objetivo
Dibujar la vista Processes a partir de datos planos y devolver las áreas de clic, sin I/O y sin depender del árbol de `usage`.

## Alcance
- `src/ui/view.rs`: `View { tab: ViewTab, rows: Vec<Row>, scroll, selected, sort, header info, dialog: Option<...> }`,
  `Row { depth, kind (project/workspace/tab/pane/process), label, folded: Option<bool>, memory, cpu, agent cols, hot, estimated }`.
- `src/ui/mod.rs`: `render(frame, &View) -> Hits` con barra de pestañas arriba (`Processes` / `Context`), cabecera con columnas
  clicables (nombre, memoria, CPU, agente, estado, contexto), filas con sangría y flecha de plegado, fila seleccionada, camino caliente resaltado, scroll.
- `src/ui/hits.rs`: `Hits` → `Hit::{Tab(ViewTab), Sort(Sort), Row(usize), Fold(usize)}` por coordenada.
- `src/format.rs`: tamaños (`5.2 GB`), porcentajes (`312%`), `—` para `None`, `≈` para estimados.
- Dependencias: `ratatui` (con crossterm), dev `insta`.

## Criterios de aceptación
- CA1: snapshot de un árbol de ejemplo a 100×20 con proyecto, workspace con worktree, pestaña, pane con agente y procesos.
- CA2: la fila seleccionada y el camino caliente tienen estilo distinto (test de estilos de celda).
- CA3: `Hits` devuelve la fila, la flecha, la pestaña y la columna correctas para coordenadas dadas.
- CA4: nombres largos se cortan al final y no rompen columnas; ancho mínimo (40 columnas) se dibuja sin panic.
- CA5: `format.rs` probado con rstest (bytes, %, `None`).

## Tests requeridos
- `ui/tests.rs` con `TestBackend` + `insta`; `format/tests.rs`.

## Design a actualizar
- `design/tui.md`: columnas y colores elegidos.

## Notas
- `ui` no importa `usage` ni `app`: `View` es suyo. 010 convierte `usage::Tree` en `View`.
