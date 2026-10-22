# 020 — El árbol empieza plegado

- **Estado**: hecho
- **Depende de**: 018
- **Estimación**: ~150 líneas (sobre todo tests y snapshots)

## Objetivo
Con muchos proyectos, el árbol desplegado entero es una pared de procesos. Pedido por el usuario (2026-10-22): que salga plegado por defecto.

## Alcance
- Al abrir cctop solo se ven los proyectos (cada uno con sus totales, barra y sparkline); un clic en la flecha, `Enter` o `→` despliega.
- Las filas que aparecen después (un proyecto, workspace, pestaña, pane o proceso nuevo) también llegan plegadas; lo que el usuario desplegó se conserva por id como hoy.
- La ruta caliente (amarillo) se sigue viendo en la fila plegada que la contiene.
- Doble clic en Processes y la elección de agente al entrar en Context no dependen de que la fila esté desplegada.
- Si la selección apunta a una fila oculta, la selección no se pierde ni salta a otra (o se elige el ancestro visible; decidir y documentar).

## Criterios de aceptación
- CA1: primer dibujo con solo proyectos (snapshot).
- CA2: desplegar un proyecto muestra sus workspaces plegados; una lectura nueva no lo vuelve a plegar.
- CA3: un proceso nuevo bajo un pane desplegado llega plegado si tiene hijos.

## Design a actualizar
- `design/tui.md` ("everything starts unfolded" pasa a plegado y por qué).
