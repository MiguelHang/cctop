# 014 — `cctop open` y la acción del plugin

- **Estado**: pendiente
- **Depende de**: 004
- **Paralelo con**: 003, 005, 007, 008, 009, 010, 013
- **Estimación**: ~350 líneas (código ~150, tests ~200)

## Objetivo
La acción `cctop` (en project, workspace, tab y pane) enfoca el proyecto "cctop" o lo crea con una pestaña que ejecuta `cctop`; siempre una instancia y en el mismo sitio.

## Alcance
- `src/cornercase/open.rs` (solo usa `Snapshot` y `Runner`, así respeta las capas sin módulo nuevo) + `run_open()`:
  `open <CORNERCASE_PLUGIN_STATE> --focus`, después `status --json` para encontrar el proyecto por ruta (canónica), y `new-tab --workspace <id> --name cctop -- <ruta absoluta del binario>` solo si ninguna pestaña ejecuta ya cctop; si existe, `focus` de esa pestaña.
- Cómo saber que una pestaña ya ejecuta cctop: por el comando del pane en `status --json` (si cornercase lo da) o, si no, por el nombre de pestaña `cctop`; sin leer procesos. Documentar la elección.
- Ruta del binario: `std::env::current_exe()` (funciona igual para `[[build]]` y `[download]`).
- Manifiesto: `[[actions]] id = "cctop"`, `title = "cctop"`, `contexts = ["project", "workspace", "tab", "pane"]`, `command = ["target/release/cctop", "open"]`.

## Criterios de aceptación
- CA1: sin proyecto cctop: llama a `open` y `new-tab` con el argv esperado (`FakeRunner`).
- CA2: con proyecto y pestaña cctop: no crea otra, enfoca la existente.
- CA3: con proyecto sin pestaña cctop (la cerraron): crea la pestaña.
- CA4: sin `CORNERCASE_PLUGIN_STATE` falla con un mensaje claro (fuera de cornercase).
- CA5 (manual): `cornercase plugin link .` + acción desde el menú de un pane, dos veces → un solo proyecto y una pestaña.

## Tests requeridos
- `cornercase/open/tests.rs` con `FakeRunner`.

## Design a actualizar
- `design/plugin.md` (cómo se detecta la pestaña existente).
