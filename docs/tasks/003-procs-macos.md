# 003 — Procesos en macOS

- **Estado**: hecho
- **Depende de**: 002
- **Paralelo con**: 004, 005, 006, 007, 008, 009, 014 (solo toca `procs/macos*`)
- **Estimación**: ~400 líneas (código ~250, tests ~150)

## Objetivo
La misma `Sample` en macOS: footprint de `proc_pid_rusage`, CPU de `PROC_PIDTASKINFO`, entorno de `KERN_PROCARGS2`.

## Alcance
- `src/procs/macos.rs` (`cfg(target_os = "macos")`): `proc_listallpids`, `proc_pidinfo(PROC_PIDTBSDINFO)` (ppid, pgid, uid, nombre),
  `proc_pid_rusage(RUSAGE_INFO_V4)` → `ri_phys_footprint`, `PROC_PIDTASKINFO` → `pti_total_user + pti_total_system`
  (convertir con `mach_timebase_info`, en Apple Silicon no son ns), `sysctl([CTL_KERN, KERN_PROCARGS2, pid])` → argc, exec path, args y entorno.
- Parser puro de `KERN_PROCARGS2` (buffer → args + vars) probado en cualquier plataforma con buffers sintéticos (`cfg(test)` sin `cfg(target_os)`).
- `unsafe` mínimo y local, con su comprobación de longitudes; sin `unwrap`.

## Criterios de aceptación
- CA1 (CI macOS): el pid del test aparece con su ppid; un hijo con `CORNERCASE_PANE=7` se lee de vuelta.
- CA2 (CI macOS): 64 MiB anónimos escritos suben `memory` ≥ 60 MiB.
- CA3: el parser de `KERN_PROCARGS2` sobrevive a buffers truncados, sin NUL final o con argc mayor que lo que hay (tests en Linux y macOS).
- CA4: CPU de un proceso que gira 200 ms > 0 entre dos samples (CI macOS).

## Tests requeridos
- `procs/macos/tests.rs`; el parser en un fichero propio sin `cfg(target_os)` para que corra también en Linux.

## Design a actualizar
- `design/processes.md`: timebase de CPU, crate elegido (`libc` frente a `libproc`), límites (procesos de otro usuario, zombies).

## Notas
- No hay máquina macOS: se valida en el runner de CI. Que el implementador lo diga en su informe.
