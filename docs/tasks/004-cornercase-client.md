# 004 — Cliente de cornercase

- **Estado**: pendiente
- **Depende de**: 001
- **Paralelo con**: 002, 003, 005, 006, 007, 009
- **Estimación**: ~500 líneas (código ~280, tests ~150, fixtures aparte)

## Objetivo
Leer la jerarquía de cornercase (`status --json`) en un `Snapshot` tolerante y ejecutar los comandos
públicos que usan el TUI, la acción y las alertas, sin arrancar nunca cornercase en los tests.

## Alcance
- `src/cornercase/mod.rs`: trait `Runner` (`fn run(&self, argv: &[String]) -> Result<String>`), `CommandRunner`
  real (binario de `$CORNERCASE_BIN`, si no `cornercase` en el `PATH`), `Client<R: Runner>` con `status()`,
  `open(path, focus)`, `new_tab(workspace, name, command)`, `focus(...)`, `notify(text)`.
- `src/cornercase/snapshot.rs`: `Snapshot { boot, projects }`, `Project { id, name, path, workspaces }`,
  `Workspace { id, name, branch, worktree: bool, tabs }`, `Tab { id, name, panes }`,
  `Pane { id, agent, status, model, context_percent, subagents, parent, command }`. Todo opcional salvo ids;
  campos desconocidos ignorados (`serde` sin `deny_unknown_fields`).
- `src/cornercase/fake.rs`: `FakeRunner` que responde por argv y registra las llamadas.
- `tests/fixtures/cornercase/status-<version>.json`: capturado de un cornercase real (`cornercase status --json`)
  y anonimizado (rutas, nombres). Un segundo fixture con campos de más y de menos.

## Criterios de aceptación
- CA1: el fixture real se parsea con todos sus proyectos, workspaces, pestañas y panes en el orden del sidebar, y `boot`.
- CA2: un JSON con campos desconocidos y opcionales ausentes se parsea; los ausentes quedan `None`.
- CA3: un JSON inválido o un comando que falla da `Error`, nunca panic.
- CA4: cada comando construye exactamente el argv documentado en `design/cornercase-client.md` (comprobado con `FakeRunner`).
- CA5: sin `CORNERCASE_BIN` se usa `cornercase`.

## Tests requeridos
- `cornercase/tests.rs` (parseo con rstest, argv de cada comando, errores).

## Design a actualizar
- `design/cornercase-client.md`: campos exactos que se leen de `status --json` y versión de cornercase del fixture.

## Notas
- Comprueba los nombres de los campos y flags en el repo de cornercase (`src/control/`, `src/cli/`), no los adivines.
- Variante de error `Command { argv, stderr }` y `Json` en `error.rs`.
