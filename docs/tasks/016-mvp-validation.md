# 016 — Validación del MVP

- **Estado**: pendiente
- **Depende de**: todas
- **Paralelo con**: —
- **Estimación**: ~150 líneas (fixtures nuevos, ajustes)

## Objetivo
Comprobar los criterios de éxito de IDEA.md en cornercase real y los requisitos no funcionales, y corregir lo que falle.

## Alcance
- Linux: `cornercase plugin link .`, activar, acción `cctop`, lanzar un proceso que reserve varios GB en un pane de agente → aparece en < 5 s en su pane y worktree; el servicio avisa.
- Vista de contexto en una sesión real: AGENTS.md/DESIGN.md con su peso; total ±2 puntos del % de cornercase. Capturar un fixture nuevo si la versión de Claude Code cambió.
- CPU en reposo con ~30 panes y 2 s: < 1 % (medir con `ps`/`/proc` durante 60 s); si no, perfilar y ajustar.
- macOS: lo que se pueda en CI; anotar lo no validado en uso real.
- Lista de bugs encontrados → tareas nuevas `017+` si no son triviales.

## Criterios de aceptación
- CA1–CA3: los tres criterios de éxito de IDEA.md, con evidencia (capturas `tmux capture-pane`, medidas) en el informe.
- CA4: CPU en reposo medida y bajo el 1 %.

## Design a actualizar
- Los que cambien por los ajustes; conclusiones de rendimiento con fecha en el área correspondiente.
