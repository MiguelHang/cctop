# 013 — Alertas y servicio `cctop watch`

- **Estado**: hecho
- **Depende de**: 004, 005, 008
- **Paralelo con**: 010, 011, 012, 015
- **Estimación**: ~500 líneas (código ~250, tests ~250)

## Objetivo
El servicio avisa con `cornercase notify` cuando el árbol de un pane pasa el umbral de memoria o mantiene la CPU alta, una vez por cruce.

## Alcance
- `src/alerts/rules.rs` (puro): estado por pane + `Tree` + instante → alertas. Memoria: avisa al cruzar, rearma bajo el 80 %.
  CPU: por encima del umbral durante `cpu_sustain` seguidos; rearma bajo el 80 %. Panes que desaparecen se olvidan.
- `src/alerts/message.rs`: `cctop: <workspace> › <pane/agente> — <proceso más caro> 5.2 GB`.
- `src/alerts/watch.rs` + `run_watch()`: bucle cada `alert_interval` (muestra, `usage::build`, reglas, `notify`), con `Clock` inyectable; un fallo de `status` o `notify` se registra en stderr (el log del plugin) y sigue.
- Manifiesto: `[[service]] id = "watch"`, `command = ["target/release/cctop", "watch"]` (ver 015 para la ruta del binario).

## Criterios de aceptación
- CA1: una serie memoria 3 → 5 → 5 → 3.5 → 5 GB avisa una sola vez; 3 → 5 → 3 → 5 avisa dos.
- CA2: CPU 400 % durante 59 s no avisa; 61 s sí, una vez.
- CA3: el mensaje nombra workspace, pane y proceso más caro (test de texto).
- CA4: el bucle con `FakeRunner`, `FakeSource` y reloj falso llama a `notify` con el argv esperado.
- CA5: el bucle no muere si `status` falla.

## Tests requeridos
- `alerts/rules/tests.rs` (rstest con series), `alerts/watch/tests.rs`.

## Design a actualizar
- `design/alerts.md` (rearme, qué pasa con panes nuevos y desaparecidos).
