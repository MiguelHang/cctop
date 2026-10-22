# 018 — Aspecto tipo btop y Context con su lista de agentes

- **Estado**: en curso
- **Depende de**: 017
- **Paralelo con**: 019 (no toca `src/cornercase/open*`, `cornercase-plugin.toml` ni `design/plugin.md`)
- **Estimación**: ~900 líneas (histórico ~150, ui ~400, app ~150, tests y snapshots aparte)

## Objetivo
Que cctop se lea de un vistazo, como btop: gráficas, barras y colores por nivel. Y que la vista Context no dependa de haber seleccionado antes un pane en Processes: tiene su propia lista de agentes.

Maqueta acordada con el usuario (2026-10-22):

```
╭─ cpu ─────────────────── 23% ─╮╭─ mem ──────────── 9.4 / 31 GB ─╮
│⣀⣀⣠⣤⣀⣀⡀⢀⣀⣤⣶⣿⣷⣤⣀⣀⣀⣠⣤⣤⣀⡀  ⢀⣀⣀⣤⣴⣶ ││⣤⣤⣤⣴⣴⣴⣶⣶⣶⣶⣶⣶⣷⣷⣷⣷⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿│
╰────────────────────── -60m ─╯╰──────────────────────── -60m ─╯
╭─ processes ──── [Processes] Context ── sort: mem ▾ ─────────────╮
│ name                    memory               cpu   cpu 60s     │
│▾ cornercase-terminal    ██████████▊░░░ 4.8 GB 38%  ▁▂▃▅▇█▇▅▃▂▁ │
│    ▾ %233 claude · 41%  ████████▍░░░░░ 3.6 GB 29%  ▂▃▅▇█▇▅▃▂▂▃ │
╰─ ↑↓ move  ⏎ fold  Tab view  s sort  k kill  q/Esc close ───────╯

╭─ agents ─────────────────────╮╭─ shopfront › search-bar › %149 ── 168k · 16% ─╮
│▶ search-bar › %149  ███▏░ 16%││ ████████████▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░ │
│  checkout-flow › %155 ██ 11% ││ █ fixed ≈68k  ▓ dynamic ≈100k  ░ free  cc 16% │
│  cctop › main › %233  █▊  9% │├─ fixed ──────── ≈68k ─┬─ dynamic ──── ≈100k ─┤
│                              ││ CLAUDE.md ████  ≈31k  │ gh run view… ██ ≈6k  │
╰─ ↑↓ choose ──────────────────╯╰───────────────────────┴──────────────────────╯
```

## Alcance
- **Histórico en memoria** (sin persistir): por fila (proyecto, workspace, tab, pane, proceso, por id estable) la CPU de los últimos 60 s, y para el total CPU y memoria de la última hora (a la resolución que quepa: el ancho de la gráfica manda). Acotado en tamaño; las filas que desaparecen se olvidan.
- **Cabecera** con dos gráficas en braille (CPU total en % de la máquina y memoria total frente a la de la máquina), cada una con su valor actual. Se oculta si no cabe en altura (terminal bajo) antes que recortar la tabla.
- **Processes**: barra de memoria por fila (proporcional al máximo de su nivel o al total, decidir y documentar) coloreada verde/amarillo/rojo según el umbral de memoria de `config.toml`; sparkline de CPU de 60 s por fila. Las columnas se añaden y quitan por ancho como hoy (el nombre conserva su mínimo). Bordes redondeados con títulos en el borde y la ayuda de teclas en el borde inferior.
- **Context sin selección previa**: a la izquierda la lista de todas las sesiones de Claude del servidor (proyecto › workspace › pane, barra y %), ordenada por contexto; a la derecha el desglose de la marcada (barra apilada fijo/dinámico/libre, listas con barras). Clic o `↑`/`↓` eligen. Al entrar: si en Processes la selección tiene un Claude debajo (pane, su proceso, su tab o su workspace) se marca ese; si no, el de más contexto. Doble clic en un pane con Claude en Processes abre su Context. Solo se lee el transcript del agente marcado (como hoy, la lista usa el % de `status`).
- **Comandos del desglose sin prefijos `cd <ruta>; ` / `cd <ruta> && `**: se muestra el comando real.
- `Esc` cierra cctop como `q` cuando no hay un diálogo abierto (con el diálogo de kill abierto sigue cancelando).
- Sin coste de CPU apreciable en reposo: el histórico se actualiza en cada lectura que ya se hace; mantener el redibujo solo con cambios (017). Volver a medir con el método de `docs/VALIDATION.md` §4 y anotarlo.

## Criterios de aceptación
- CA1: snapshots de Processes y Context con gráficas, barras y sparklines a 120×40 y a 80×24; a 40 columnas sigue funcionando (sin gráficas ni sparklines).
- CA2: entrar en Context sin nada seleccionado muestra el agente con más contexto; con un tab seleccionado que tiene un Claude, ese.
- CA3: clic en un agente de la lista cambia el desglose; doble clic en un pane con Claude en Processes abre su Context.
- CA4: `cd /x/y; git diff` se muestra como `git diff`.
- CA5: el histórico no crece sin límite (test con muchas filas que aparecen y desaparecen).
- CA6: CPU en reposo medida y anotada en `design/tui.md`; no peor que 017 en más de unas décimas.
- CA7: `Esc` sale; con el diálogo de kill abierto, cancela.

## Tests requeridos
- Histórico: anillo acotado, olvido de filas, agregados del total.
- ui: snapshots nuevos (los viejos que cambian se revisan y aceptan, los de tests renombrados se borran).
- app: elección del agente al entrar en Context, clics en la lista, doble clic, `Esc`.
- context: limpieza del prefijo `cd`.

## Design a actualizar
- `design/tui.md` (layout nuevo, histórico, colores, Context con lista, Esc, coste en reposo). `design/context.md` si cambia cómo se nombran los comandos.
