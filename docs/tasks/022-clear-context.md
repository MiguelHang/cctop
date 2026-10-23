# 022 — Cabecera fija y un Context que se entiende

- **Estado**: en curso
- **Depende de**: 018, 021
- **Estimación**: ~700 líneas (ui ~350, app ~150, context ~100; tests y snapshots aparte)

## Objetivo
Dos quejas del usuario tras probar 021 (2026-10-23):
1. Al cambiar entre Processes y Context la vista "baila": la cabecera de gráficas desaparece en Context y todo sube.
2. No se entienden los porcentajes ni qué es el contexto. Context debe contestar: ¿cómo de lleno está este agente?, ¿qué lo llena?, ¿qué puedo hacer?

Maqueta acordada (los nombres son de ejemplo; en tests usar nombres sintéticos):

```
╭─ shop › main · claude Opus 5.5 · working ───────────────────────────────────────────────────╮
│ Context used  196k of 1M tokens                                                         20% │
│ ███████████████████▌░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│ ▓ loaded on every message 32k   ▒ this conversation 164k   ░ free 804k                      │
╰─────────────────────────────────────────────────────────────────────────────────────────────╯
╭─ agents ─────────────────────────╮╭─ what fills this conversation (164k) ─────────────────────╮
│▶ shop › main          196k  20% ││ ▸ Command output     98k  60%  ██████████████████         │
│  checkout             190k  19% ││     grep -n -i -E "window pr…   18k                       │
│  search-bar           160k  16% ││     cat docs/VALIDATION.md       7k                       │
│                                  ││     … 37 more                   73k                       │
│                                  ││ ▸ Claude's replies   17k  10%  ███                        │
│                                  ││ ▸ Your messages      10k   6%  ██                         │
│                                  ││ ▸ Files read          8k   5%  █▌                         │
│ sorted by context used           │├─ loaded on every message (32k) ────────────────────────────┤
│                                  ││   Skills 18k · System prompt 5k · AGENTS.md 5k · Agents 3k │
│                                  ││ ▸ /compact would free about 164k; the 32k stays.            │
╰─ ↑↓ choose agent ────────────────╯╰─ ⏎ open group · Tab processes · q close ───────────────────╯
```

## Alcance
- **Cabecera fija**: la franja superior tiene la misma altura en las dos vistas (y la misma regla de ocultarse en terminales bajos, decidida una vez para ambas). En Processes: las gráficas de CPU y memoria. En Context: el agente marcado (lugar, agente y modelo, estado; "Context used N of W tokens", %, barra coloreada verde < 50 %, amarillo < 80 %, rojo ≥ 80 %; leyenda en palabras). Al cambiar de pestaña no se mueve ninguna línea que no cambie de contenido. Snapshot o test que lo compruebe (mismas filas de la caja inferior en ambas vistas).
- **Palabras**: "fixed" → "loaded on every message"; "dynamic" → "this conversation" (desde la última compactación). Mismas palabras en el README y en `design/`.
- **Agrupar por tipo**: comandos, ficheros leídos, búsquedas, respuestas de Claude, mensajes del usuario, resultados de subagentes, otras herramientas… (los que da hoy `context::Breakdown`; nombres claros en inglés). Cada grupo: total, % sobre la conversación, barra contra la conversación (no contra el más grande) y sus 3 elementos mayores + "… N more"; plegado/desplegado con clic o `Enter`, los grupos empiezan desplegados solo los dos mayores (o decidir y documentar).
- **Lo fijo en una línea**: las partes cargadas en cada mensaje, en una línea de mayor a menor (se parte si no cabe).
- **Consejo útil**: "/compact would free about N; the M stays." y, si un solo elemento pesa > 10 % de la conversación, nombrarlo ("one command, `grep …`, took 18k"). Texto corto, sin inventar cifras: solo lo que da el desglose.
- **Lista de agentes**: etiqueta legible (nombre de la pestaña o del workspace; `%id` solo para desempatar), "196k  20%" con el color de nivel, ordenada por tokens; pie "sorted by context used".
- **Fuera**: "cornercase N%" en la vista (el % de cornercase sigue usándose solo para elegir la ventana, ver `design/tui.md`); los `≈` por línea, con una sola nota de que el reparto es estimado y el total es el real de Claude.
- En Processes, la columna Context muestra el mismo "20%" con el color de nivel.

## Criterios de aceptación
- CA1: cambiar de pestaña no mueve la caja inferior (test).
- CA2: snapshots de Context a 120×40, 80×24 y 40 columnas con los textos nuevos; sin "fixed"/"dynamic"/"cornercase N%".
- CA3: grupos con su total, % sobre la conversación y top 3; plegar/desplegar con clic y `Enter`.
- CA4: la línea de `/compact` usa las cifras del desglose; el aviso de un elemento grande aparece solo por encima del 10 %.
- CA5: lista de agentes con etiquetas legibles y desempate por `%id`.
- CA6: CPU en reposo sin empeorar (medida rápida como en 018).

## Design a actualizar
- `design/tui.md` (cabecera fija, Context nuevo, colores de nivel), `design/context.md` (grupos y palabras), `README.md`, `docs/VALIDATION.md` §3, `CHANGELOG.md` (al final de `## 0.1.0`).
