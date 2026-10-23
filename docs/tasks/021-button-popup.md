# 021 — Botón propio y cctop en un popup

- **Estado**: en curso
- **Depende de**: 019, 020; cornercase ≥ 0.12.187 (#457: botón de plugin y `placement = "popup"`)
- **Estimación**: ~400 líneas (manifiesto, arranque, tests, docs)

## Objetivo
Pedido por el usuario (2026-10-23): un botón **cctop** en la ventana de cornercase que abre cctop en un popup sobre todo, y que `q`/`Esc` lo cierre. Referencia en el repo de cornercase (`~/me/cornercase-terminal`, solo lectura; el commit 9a91cff1 está en `origin/main`, léelo con `git show origin/main:<ruta>`): `site/src/content/docs/docs/guides/plugins.mdx` (Buttons, Popups), `design/plugins/manifest.md`, `design/plugins/running.md`.

## Alcance
- Manifiesto:
  - `min_cornercase = "0.12.187"`;
  - `[button] title = "cctop"` con un `icon` de una celda (elegir uno legible, p. ej. `▤`; documentar);
  - una acción `contexts = ["button", "project", "workspace", "tab", "pane"]`, `placement = "popup"`, sin `width`/`height` (80 % de la ventana), que ejecuta la TUI directamente (`target/release/cctop`, sin `open`);
  - se mantiene en el menú del pane una segunda acción **cctop in a split** (la de 019, `cctop open`, `placement` por defecto).
- Al arrancar dentro del popup, cctop usa el contexto que da cornercase (`CORNERCASE_CONTEXT`, `CORNERCASE_PANE`/`_TAB`/`_WORKSPACE`/`_PROJECT`): selecciona esa fila y despliega solo la ruta hasta ella (el resto sigue plegado, 020), y si tiene un Claude, ese es el agente marcado al entrar en Context. Sin esas variables, como hoy.
- El popup corre en el servidor y hereda las variables del pane mostrado: cctop y sus hijos (los `cornercase status`) no deben contarse dentro de ese pane. Excluir del árbol el grupo de procesos de cctop (ya se conoce por `getpgrp` para el kill) o lo que se decida; documentar.
- `q` y `Esc` ya salen (018): al salir, cornercase cierra el popup. Comprobar que la salida es con código 0 (un código distinto muestra un error en la ventana).
- Ratón: el popup se lo pasa al programa cuando lo pide; nada que cambiar salvo comprobarlo en la documentación.
- Docs: `design/plugin.md` (botón, popup, por qué se mantiene el split, `min_cornercase`), `docs/VALIDATION.md` §1, `README.md`, `AGENTS.md` (sección Plugin), `CHANGELOG.md` (al final de `## 0.1.0`, sin subir versión: 0.1.0 no está publicada).

## Criterios de aceptación
- CA1: el manifiesto es válido para cornercase 0.12.187 (las reglas de `plugins/manifest.rs` de `origin/main`: icono de una celda, `width`/`height` solo en popup y en rango; test que lea el manifiesto y compruebe esos campos).
- CA2: con `CORNERCASE_CONTEXT=button` y `CORNERCASE_PANE=N`, el primer dibujo tiene el pane N seleccionado y su ruta desplegada (snapshot o test de estado).
- CA3: los procesos del grupo de cctop no aparecen en el árbol (test con `ProcessSource` falso).
- CA4: `q` y `Esc` terminan con código 0.

## Design a actualizar
- `design/plugin.md`, `design/tui.md` (arranque con contexto), `design/usage-tree.md` o `design/processes.md` (exclusión del propio cctop).
