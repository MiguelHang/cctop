# 002 — Procesos: trait, `Sample` y Linux

- **Estado**: pendiente
- **Depende de**: 001
- **Paralelo con**: 004, 005, 006, 009 (módulos disjuntos)
- **Estimación**: ~550 líneas (código ~300, tests ~250)

## Objetivo
Una muestra de todos los procesos del usuario con la métrica de memoria de cornercase y la CPU %,
detrás de un trait para que el resto se pruebe con fakes.

## Alcance
- `src/procs/mod.rs`: `ProcessSource` (`fn sample(&self) -> Result<Sample>`), `Sample` (instante, `Vec<Process>` o mapa por pid),
  `Process` (pid, ppid, pgid, name, args, `memory: Option<u64>` bytes, `cpu_time: Duration`, `pane: Option<u64>`,
  `server: Option<String>`, `claude_config_dir: Option<PathBuf>`), `cpu_percent(prev, next) -> HashMap<pid, f64>`
  (delta CPU / delta pared, 100 % = un núcleo; primer sample sin CPU; pid reutilizado → sin CPU), `Clock` si hace falta.
- `src/procs/linux.rs` (`cfg(target_os = "linux")`): recorre `/proc`, filtra por uid propio, lee `status`
  (`RssAnon + VmSwap`, `PPid`, `Uid`), `stat` (utime+stime, pgrp, en ticks → `sysconf(_SC_CLK_TCK)`), `cmdline`, `comm`,
  `environ` (solo `CORNERCASE_PANE`, `CORNERCASE_SERVER`, `CLAUDE_CONFIG_DIR`). Un proceso que desaparece o no se puede leer se descarta, nunca error.
- `src/procs/fake.rs` (`cfg(test)` o `pub(crate)` para otros módulos de test): `FakeSource` desde una lista.
- Parsers puros (`status`, `stat` con `comm` con espacios y paréntesis, `environ` con NULs) separados de la I/O.
- `tests/fixtures/proc/`: textos de `status`, `stat`, `environ` de ejemplo.

## Criterios de aceptación
- CA1: la muestra real contiene el pid del propio test con su ppid y nombre.
- CA2: un hijo `/bin/sleep` lanzado con `CORNERCASE_PANE=7` y `CORNERCASE_SERVER=x-1` aparece con esos valores.
- CA3: escribir en un mmap anónimo de 64 MiB sube `memory` ≥ 60 MiB; mapear y leer un fichero de 64 MiB no lo sube más de 8 MiB.
- CA4: `cpu_percent` con dos samples sintéticos da el % esperado; un pid nuevo o reutilizado da `None`.
- CA5: parsers tolerantes: campos que faltan → `None`, nunca panic (tests con entradas truncadas).

## Tests requeridos
- `procs/tests.rs` (cálculo de CPU, rstest con `#[case::...]`), `procs/linux/tests.rs` (parsers con fixtures + proceso propio).
- Esperar con `wait_until`, nunca sleeps.

## Design a actualizar
- `design/processes.md` si cambia algo (p.ej. cómo se trata un pid reutilizado, `pgid`).

## Notas
- Dependencias: `libc` (uid, clk_tck) o `rustix`; documenta la elección en `design/processes.md`.
