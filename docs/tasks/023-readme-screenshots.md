# 023 — Capturas en el README

- **Estado**: pendiente
- **Depende de**: 022
- **Estimación**: ~200 líneas de script, sin cambios en `src/`

## Objetivo
Pedido por el usuario (2026-10-25) antes de publicar: que el README sea visual, con capturas de las dos vistas.

## Alcance
- Dos imágenes en `docs/screenshots/`: `processes.svg` (vista Processes con gráficas, barras, sparklines, un proyecto desplegado hasta sus procesos y la ruta caliente) y `context.svg` (vista Context con la franja, la lista de agentes, los grupos de la conversación y la línea de lo cargado en cada mensaje). Tamaño de terminal ~120×34.
- **Solo datos sintéticos**: proyectos `shop`, `checkout`, `search-bar` o similares; nada de la máquina real (ni proyectos, ni rutas, ni `/home/<usuario>`, ni procesos ajenos, ni transcripts reales).
- Se capturan con el binario de release real (no un render aparte), en un tmux privado (`tmux -L shots`), con:
  - un `CORNERCASE_BIN` falso (script) que responde `status --json` sintético con un `boot` propio y no hace nada con el resto de subcomandos;
  - procesos sintéticos con `CORNERCASE_PANE`/`CORNERCASE_SERVER` de ese `boot` (copias de binarios con nombres como `claude`, `node`, `cargo`, `rust-analyzer`, unos que gasten algo de CPU y memoria, de forma acotada y que se paren solos);
  - un `CLAUDE_CONFIG_DIR` temporal con `sessions/<pid>.json` y transcripts sintéticos para los `claude`;
  - `HOME`/`XDG_CONFIG_HOME` temporales para que no lea la config real.
- Captura con `tmux capture-pane -e -p` y conversión ANSI → SVG (una celda por posición fija, fondo oscuro, fuente monoespaciada, colores de ratatui), sin dependencias que no haya en la máquina (Python 3 sí está).
- Todo reproducible con un script `docs/screenshots/make.sh` (+ su conversor), que limpia todo lo que lanza al salir (trap). Nunca toca el cornercase real.
- README: las dos imágenes arriba (tras la primera frase) o junto a cada vista, con texto alternativo.

## Criterios de aceptación
- CA1: `docs/screenshots/make.sh` regenera las dos SVG desde cero y deja la máquina como estaba (sin procesos ni tmux sueltos).
- CA2: las SVG se ven bien en un navegador (alineación de bordes, braille y bloques) y en GitHub (`<img>`).
- CA3: `grep` en las SVG y scripts no encuentra nombres de la máquina real ni `/home/`.
- CA4: los checks del repo siguen en verde (jscpd incluido).

## Design a actualizar
- `design/plugin.md` (Distribución): cómo se generan las capturas y por qué sintéticas.
