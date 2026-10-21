# 011 — Matar un proceso o su grupo

- **Estado**: pendiente
- **Depende de**: 010
- **Paralelo con**: 012, 013, 015
- **Estimación**: ~400 líneas (código ~200, tests ~200)

## Objetivo
`k` (o el menú de la fila) mata un proceso o su grupo tras confirmar, nombrando el proceso y su pane.

## Alcance
- `src/procs/kill.rs`: trait `Killer` (`term(pid)`, `term_group(pgid)`) con implementación real (`SIGTERM`) y fake que registra.
- `src/ui/dialog.rs`: diálogo de confirmación (proceso, pid, pane; botones `Cancel`, `Process`, `Group`) con sus `Hits`.
- `src/app/kill.rs`: abrir el diálogo desde `k` o clic derecho/menú, confirmar con clic o `Enter`, cancelar con `Esc`; mientras está abierto, ningún otro clic llega al árbol.
- Solo procesos (no filas de proyecto/pane); nunca el propio cctop ni su grupo.

## Criterios de aceptación
- CA1: sin confirmar no se llama al `Killer` (test que falla si se mata directamente).
- CA2: `Process` llama a `term(pid)`, `Group` a `term_group(pgid)`.
- CA3: el diálogo nombra proceso, pid y pane (snapshot).
- CA4: intentar matar el propio cctop no abre el diálogo y avisa.
- CA5: test real: un `/bin/sleep` hijo muere con `term` (esperando con `wait_until`).

## Tests requeridos
- `procs/kill/tests.rs`, `app/kill/tests.rs`, snapshot del diálogo en `ui/tests.rs`.

## Design a actualizar
- `design/tui.md` (qué se puede matar y qué no).
