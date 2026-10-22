# 019 — Abrir cctop en un split del pane

- **Estado**: hecho
- **Depende de**: 014
- **Paralelo con**: 018 (no toca `src/ui/`, `src/app/` ni `design/tui.md`)
- **Estimación**: ~300 líneas (código ~120, tests ~180)

## Objetivo
El proyecto "cctop" confunde (parece un proyecto más) y obliga a cambiar de sitio. La acción `cctop` del menú abre cctop en un split a la derecha del pane donde se pulsa, enfocado, y al salir (`q`/`Esc`) el pane se cierra solo.

## Alcance
- `cctop open` deja de crear o enfocar el proyecto "cctop". Con el contexto que cornercase da a la acción (leer en el repo de cornercase, `~/me/cornercase-terminal/design/plugins/running.md` y `permissions-and-environment.md`, qué variables recibe una acción: pane, tab, workspace, project):
  - pane → `split --pane <id> --focus -- exec <ruta del binario>`;
  - tab/workspace/project → su pane activo o el primero (de `status --json`), igual.
  - `exec` para que al salir cctop el pane se cierre en vez de quedar una shell (comprobar con cornercase si un pane cuya shell termina se cierra; si no, `close --pane` propio al salir no es aceptable desde la TUI, documentar la alternativa).
- Si en esa misma pestaña ya hay un pane ejecutando cctop (`program` en `status`, como en 014), se enfoca (`focus --pane`) en lugar de abrir otro.
- Quitar del código y de la documentación el proyecto en la carpeta de estado (`design/plugin.md`, `docs/VALIDATION.md` §1, `README`/`CHANGELOG` si lo mencionan). La carpeta de estado se sigue usando para lo que la use `watch`, si algo.
- Permisos del manifiesto honestos con lo que se ejecuta (`split`, `focus`).
- No ejecutar nunca `split`/`focus`/`open` contra el cornercase real en tests: `FakeRunner`.

## Criterios de aceptación
- CA1: acción desde un pane: un `split` con `--pane` de ese pane, `--focus` y `exec <binario>` (`FakeRunner`).
- CA2: acción desde tab/workspace/project: split sobre su pane activo (o el primero).
- CA3: con cctop ya en esa pestaña: `focus` de ese pane, ningún `split`.
- CA4: sin el contexto de cornercase falla con un mensaje claro.
- CA5 (manual, lo hace el usuario): menú → cctop abre el split; `q` lo cierra sin dejar shell; repetir en la misma pestaña enfoca el existente.

## Tests requeridos
- `src/cornercase/open/tests.rs` reescritos para el split.

## Design a actualizar
- `design/plugin.md` (por qué un split y no un proyecto, cómo se reutiliza, el cierre con `exec`).
