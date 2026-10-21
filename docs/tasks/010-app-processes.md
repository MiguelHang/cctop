# 010 — App: bucle, muestreo e interacción de la vista de procesos

- **Estado**: hecho
- **Depende de**: 004, 005, 008, 009 (y 002 en Linux / 003 para macOS)
- **Paralelo con**: 013, 015
- **Estimación**: ~750 líneas (código ~450, tests ~300)

## Objetivo
`cctop` usable: muestra el árbol de todo el servidor, refresca cada `interval` sin bloquear la UI y se maneja con ratón y teclas.

## Alcance
- `src/app/sampler.rs`: hilo de fondo que cada `interval` pide `status` al `Client` y una muestra a `ProcessSource`, calcula CPU % y envía `Tree` por un canal. Fallos → estado de error mostrado en la cabecera, sin salir.
- `src/app/state.rs`: estado (vista, selección por id estable de fila, plegados por id, scroll, orden) que sobrevive a cada refresco.
- `src/app/view.rs`: `usage::Tree` + estado → `ui::View` (filas visibles según plegado).
- `src/app/input.rs`: ratón (clic selecciona, clic en flecha pliega, clic en cabecera ordena, clic en pestaña cambia vista, rueda) y teclas (flechas, `Enter`, `Tab`, `s`, `q`). Puro sobre el estado + `Hits`.
- `src/app/mod.rs` + `run_tui()`: terminal crossterm (alternate screen, mouse capture, restaurar al salir y en panic), bucle de eventos con `poll` y el canal.
- Fuente de procesos según plataforma (`cfg`).

## Criterios de aceptación
- CA1: con `FakeRunner` + `FakeSource`, un tick produce un `View` con el árbol esperado (test sin terminal).
- CA2: la selección y los plegados se conservan tras un refresco aunque cambien los procesos.
- CA3: clic en la cabecera de CPU ordena por CPU; `s` alterna; clic en la flecha pliega y despliega; rueda hace scroll.
- CA4: `q` sale y el terminal queda restaurado.
- CA5: si `status` falla (cornercase parado), se muestra el error y se reintenta en el siguiente tick.
- CA6 (manual, Linux): `cargo run` dentro de cornercase muestra los panes del servidor con su memoria y CPU.

## Tests requeridos
- `app/tests.rs` (+ `app/input/tests.rs`): eventos de ratón y teclas sobre el estado; tick con fakes; esperar con `wait_until`.

## Design a actualizar
- `design/tui.md` (ids estables, comportamiento ante errores).
