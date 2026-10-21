# 008 — Árbol de uso

- **Estado**: hecho
- **Depende de**: 002 (tipos de `procs`), 004 (`Snapshot`)
- **Paralelo con**: 003, 005, 007, 009, 014
- **Estimación**: ~600 líneas (código ~300, tests ~300)

## Objetivo
Unir `Snapshot` y `Sample` (+ CPU %) en el árbol proyecto → workspace → pestaña → pane → procesos, con totales,
orden y camino caliente. Puro.

## Alcance
- `src/usage/mod.rs`: `build(snapshot, sample, cpu) -> Tree`. Asignación de procesos a panes por
  `CORNERCASE_PANE` + `CORNERCASE_SERVER == boot`; un proceso sin variables hereda las de su ancestro más cercano.
  Raíces de pane = procesos cuyo padre no es del mismo pane; hijos por ppid.
- `src/usage/totals.rs`: memoria y CPU sumadas por nivel (memoria desconocida no suma y se marca).
- `src/usage/sort.rs`: `Sort::{Memory, Cpu}` dentro de cada nivel; sin orden activo, proyectos/workspaces en el orden de cornercase.
- `src/usage/hot.rs`: por pane, el proceso más caro según el orden y la cadena de ancestros marcada.
- Datos del pane (agente, estado, modelo, % contexto, subagentes, parent) copiados del `Snapshot`.
- Test de integración `tests/usage.rs` (o unitario) con fixtures de 004 y un `Sample` en JSON/TOML en `tests/fixtures/usage/`.

## Criterios de aceptación
- CA1: procesos de otro servidor (`CORNERCASE_SERVER` distinto de `boot`) no aparecen en ningún pane.
- CA2: un nieto con el entorno limpio cae en el pane de su ancestro.
- CA3: los totales de cada nivel son la suma de sus hijos.
- CA4: con `claude → bash → cargo → test-bin (4 GB)` el camino caliente marca esos cuatro.
- CA5: el orden por CPU reordena dentro de cada nivel; el de memoria por defecto también; panes sin procesos se muestran vacíos.
- CA6: un pane del snapshot sin procesos o un proceso con pane inexistente no rompen nada.

## Tests requeridos
- `usage/tests.rs` (rstest para asignación y orden), un test con los fixtures completos.

## Design a actualizar
- `design/usage-tree.md` (memoria desconocida, desempates de orden).
