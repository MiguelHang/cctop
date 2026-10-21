# 017 — CPU en reposo

- **Estado**: hecho
- **Depende de**: 016 (la medida)
- **Paralelo con**: —
- **Estimación**: ~300 líneas (muestra de `/proc`, sampler, bucle, tests, design)

## Objetivo
Bajar la CPU del TUI en reposo hacia el requisito no funcional de IDEA.md (< 1 % con ~30 panes y 2 s). 016 midió ~4 % de un núcleo: ~30 ms por muestra de `/proc` y ~45 ms por cada `cornercase status --json`.

## Alcance
- Linux: filtrar por dueño con los metadatos de `/proc/<pid>` antes de leer nada; `cmdline` y `environ` una vez por proceso (caché por pid, inicio y nombre); leer los ficheros de `/proc` con un búfer de 4 KiB; `statm` en lugar de `status` cuando no hay nada en swap.
- `status --json` cada 10 s (o al aparecer un pane nuevo) en lugar de en cada tick.
- El bucle de la UI solo redibuja cuando algo cambia.
- Test ignorado `usage::tests::cycle_cost` (el de 016) para medir la muestra.
- Solo lectura de `/proc` y de `cornercase status --json`.

## Criterios de aceptación
- CA1: CPU en reposo medida antes y después, con fecha, en `design/`.
- CA2: comportamiento igual; tests existentes en verde.
- CA3: un pid reutilizado (otro inicio) o un `exec` (otro nombre) vuelve a leer argumentos y variables.

## Tests requeridos
- Caché de lanzamiento: se conserva, pid reutilizado, `exec`, proceso terminado.
- `statm`, `meminfo` y la elección entre `statm` y `status`.
- Sampler: `status` reutilizado, preguntado al pasar 10 s, con un pane nuevo, no con uno ya desconocido ni de otro servidor, y tras un fallo.
- Bucle: no redibuja sin cambios; el aviso de "no contesta" pide dibujo solo al cambiar.

## Design a actualizar
- `design/processes.md` (dueño, memoria, caché, coste de una muestra) y `design/tui.md` (ritmo de `status`, dibujo, coste en reposo).
