# 006 — Contexto: parseo del transcript y `Breakdown`

- **Estado**: pendiente
- **Depende de**: 001
- **Paralelo con**: 002, 003, 004, 005, 008, 009
- **Estimación**: ~700 líneas (código ~400, tests ~300; fixtures aparte)

## Objetivo
Dado el texto de un transcript de Claude Code (JSONL), calcular qué ocupa el contexto: parte fija,
dinámica desde el último `compact_boundary`, agrupada por archivo, comando y categoría, escalada al total real.

## Alcance
- `src/context/record.rs`: parseo tolerante línea a línea a un enum `Record` (attachments `instructions`,
  `skill_listing`, `deferred_tools_delta`, `mcp_instructions_delta`, `agent_listing_delta`, `prompt_snapshot`;
  `system/compact_boundary`; mensajes de usuario y asistente con `tool_use`, `tool_result`, texto, `usage`; `isSidechain`). Líneas desconocidas o rotas → `Record::Other`.
- `src/context/breakdown.rs`: un acumulador incremental (`Accumulator::push(Record)`) que reinicia la parte dinámica en cada
  `compact_boundary`, empareja `tool_use` y `tool_result` por id, agrupa (Read/Edit/Write por `file_path`, Bash por comando,
  Grep/Glob por patrón, prompts, respuestas, subagentes, otros) y produce `Breakdown { total: Option<u64>, fixed: Vec<Part>, dynamic: Vec<Part> }`
  con estimación caracteres/4 escalada para sumar el total real (`input + cache_creation + cache_read` del último `usage`).
  Sin `usage` → estimaciones sin escalar y total `None`.
- `tests/fixtures/claude/2.1.306/`: transcript real anonimizado (con `instructions` con CLAUDE.md, AGENTS.md, DESIGN.md, un compact y herramientas) y uno sintético mínimo.

## Criterios de aceptación
- CA1: la parte fija del fixture lista CLAUDE.md, AGENTS.md y DESIGN.md con su ruta y peso, y system prompt, skills, tools, MCP y agentes.
- CA2: la dinámica solo cuenta lo posterior al último `compact_boundary`.
- CA3: las partes suman el total real (± redondeo) y van marcadas como estimadas.
- CA4: líneas de sidechain no cuentan.
- CA5: líneas truncadas, JSON inválido y tipos desconocidos no rompen nada.
- CA6: alimentar el transcript en dos trozos da el mismo `Breakdown` que de una vez.

## Tests requeridos
- `context/record/tests.rs`, `context/breakdown/tests.rs` (rstest para la agrupación).

## Design a actualizar
- `design/context.md`: campos exactos usados y cómo se cuenta cada parte.

## Notas
- Puro, sin I/O: 007 le da las líneas. Leer `design/context-use.md` de cornercase para el total.
