# Validación manual del MVP (0.1.0)

Lo que queda de [016](tasks/016-mvp-validation.md) y no puede hacerse sin el cornercase real, sin una máquina macOS o sin publicar el repositorio. Cada paso dice qué comprobar y qué evidencia guardar para el informe de 016 (capturas con `tmux capture-pane -p`, medidas). Lo que falle y no sea trivial pasa a una tarea `017+`.

## 0. Antes de empezar

```sh
cargo fmt --check && cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked && cargo-machete && npx -y jscpd@4.3.0 && .github/versions.sh
cargo build --release --locked
```

## 1. Instalar el plugin desde el checkout (Linux)

```sh
cornercase plugin link .
cornercase plugin enable cctop
```

- Al activarlo (`enable`) o con `cornercase plugin list`, cornercase muestra la acción `cctop`, el servicio `watch` y los permisos `read`, `notify`, `open`.
- Elegir **cctop** en el menú desde un proyecto, un workspace, una pestaña y un pane: la primera vez crea el proyecto `cctop` (carpeta: el estado del plugin) con una pestaña que ejecuta cctop; las siguientes solo lo enfocan, sin abrir otra pestaña.
- Cerrar el proyecto `cctop` y repetir la acción: vuelve a crearlo.
- Salir de cctop con `q` en su pestaña y repetir la acción: añade una pestaña nueva en vez de enfocar la que quedó en la shell.
- El servicio corre: `pgrep -a cctop` muestra un `cctop watch`, y `cornercase plugin logs cctop` no muestra errores (`cctop watch: …`).

## 2. Un proceso de varios GB (criterio de éxito 1)

En un pane donde corra un agente (o una shell de un workspace con worktree), reservar y tocar 5 GiB (hace falta esa memoria libre):

```sh
python3 -c "b = bytearray(5 * 2**30); b[::4096] = b'x' * (len(b) // 4096); import time; time.sleep(120)"
```

- En cctop (Processes, orden por memoria), el proceso `python3` aparece en menos de 5 s bajo su pane, su pestaña y el workspace correcto (marcado `(worktree)` si lo es), con unos 5 GB, y la ruta caliente (amarillo) llega hasta él. Los totales del pane, la pestaña, el workspace y el proyecto lo incluyen.
- El servicio avisa una sola vez (toast y notificación del escritorio): `cctop: <workspace> › <pane> 5.x GB — python3 -c … 5.x GB`. No repite mientras siga por encima.
- `k` (o clic derecho) sobre el proceso: el diálogo nombra el proceso, su pid, su grupo y su pane; `Esc` no hace nada; `Process` lo termina y desaparece en el siguiente refresco.
- Repetir con el umbral de CPU si se quiere: `cpu_threshold = 50` y `cpu_sustain = "10s"` en `$CORNERCASE_PLUGIN_CONFIG/config.toml` (la carpeta de configuración que cornercase da al plugin; ver design/config.md), reiniciar el servicio (`cornercase plugin disable cctop && cornercase plugin enable cctop`) y lanzar `yes > /dev/null` en un pane.
- Evidencia: captura de cctop con el árbol desplegado y del aviso.

## 3. El contexto frente a cornercase (criterio de éxito 2)

En un pane con una sesión de Claude en este repositorio (o en el de cornercase), tras al menos una respuesta:

- Seleccionar el pane en Processes y pulsar `Tab`: la vista Context muestra `AGENTS.md` y `DESIGN.md` (y `CLAUDE.md`, `MEMORY.md`…) en `Fixed` con su peso `≈`, y en `Dynamic` los ficheros y comandos leídos desde la última compactación.
- `Total N tokens` y su % frente al % de cornercase que aparece al lado y en la barra de cornercase: deben quedar a ±2 puntos. Unos puntos de diferencia son normales (cornercase redondea y refresca a su ritmo); repetir la lectura tras una respuesta nueva.
- Hacer `/compact` en la sesión: tras la siguiente respuesta, la parte dinámica se vacía y vuelve a crecer.
- Si la versión de Claude Code (`claude --version`) ya no es 2.1.306, capturar un transcript nuevo como fixture **sintética** (rutas y datos personales reemplazados) en `tests/fixtures/claude/<versión>/`.
- Evidencia: captura de la vista Context y del % de cornercase en el mismo momento.

## 4. CPU en reposo con unos 30 panes (requisito no funcional)

Medida previa sin cornercase real (2026-10-21, VM de 8 vCPU, 58 panes, 128 procesos del usuario): el muestreo de `/proc` y el árbol cuestan unos 30 ms de CPU por ciclo y cada `cornercase status --json` unos 47 ms en el cliente, así que con `interval = "2s"` cctop rondaría el **4 % de un núcleo**, por encima del 1 % pedido. Detalle y primeras ideas de mejora en [design/processes.md](../design/processes.md). Para repetir la parte de muestreo:

```sh
cornercase status --json > /tmp/status.json
CCTOP_CYCLE_STATUS=/tmp/status.json cargo test --release --lib cycle_cost -- --ignored --nocapture
```

Medida real con cctop abierto en Processes, sin tocarlo, 60 s:

```sh
pgrep -a cctop    # elegir el pid de la TUI (el que no es "cctop watch")
pid=<pid>
t0=$(awk '{print $14+$15+$16+$17}' /proc/$pid/stat); sleep 60
t1=$(awk '{print $14+$15+$16+$17}' /proc/$pid/stat)
echo "scale=2; ($t1 - $t0) * 100 / 60 / $(getconf CLK_TCK)" | bc    # % de un núcleo
```

- Los campos 16 y 17 suman los hijos ya esperados (los `cornercase status`); el trabajo del servidor de cornercase al responder no entra, medirlo aparte si importa (`pidstat -p <pid del servidor> 60 1`).
- Repetir con la vista Context sobre un pane con Claude, y para `cctop watch` (cada 5 s por defecto).
- Si pasa del 1 % (lo esperado según la medida previa), abrir `017-idle-cpu` con el perfil (`perf record -g -p $pid`) y las ideas de `design/processes.md`.

## 5. Publicar: repositorio, CI y release

Requiere confirmación explícita antes de cada paso que publica.

1. Crear el repositorio público `MiguelHang/cctop` y subir `main`:
   ```sh
   gh repo create MiguelHang/cctop --public --source . --remote origin --push
   gh repo edit MiguelHang/cctop --add-topic cornercase-plugin
   ```
2. En *Settings → Actions → General → Workflow permissions*: permitir que GitHub Actions cree pull requests (la release abre una con los `[download.*]`).
3. Esperar a que CI pase en `main` (Linux y macOS: `gh run watch`).
4. Ensayo sin publicar: `gh workflow run release.yml` y revisar los cuatro artefactos `cctop-<target>.tar.gz`.
5. Publicar: `git tag v0.1.0 && git push origin v0.1.0`. La release sale con los archivos, sus `.sha256` y la sección `## 0.1.0` de `CHANGELOG.md`.
6. Revisar y fusionar la PR `chore: point downloads at v0.1.0` (no lanza CI: cerrarla y reabrirla para pasar los checks).
7. Comprobar la instalación real: `cornercase plugin unlink cctop` (o en otra máquina), `cornercase plugin install MiguelHang/cctop` y `cornercase plugin enable cctop`; debe descargar el binario y verificar su sha256 en vez de compilar.

## 6. macOS

Sin máquina macOS, lo que se valida es lo que hace CI:

- **CI (`macos-latest`, aarch64)**: `clippy -D warnings` y todos los tests. Incluye la fuente real de procesos sobre el propio proceso de test (su pid, una variable puesta a un hijo `/bin/sleep` leída con `KERN_PROCARGS2`, la memoria que sube al escribir 64 MiB anónimos y no con un fichero mapeado, la CPU de 200 ms de trabajo) y la señal a un `/bin/sleep` hijo. Mirar en el primer run las dos dudas anotadas en [design/processes.md](../design/processes.md): que `KERN_PROCARGS2` de `/bin/sleep` (binario de plataforma bajo SIP) devuelva el entorno y que su `pbi_name` sea `sleep`.
- **Release**: compila `x86_64-apple-darwin` y `aarch64-apple-darwin` en el mismo runner arm; el binario x86_64 se compila pero no se ejecuta en ningún test.
- **Sin validar en uso real**: la TUI y el servicio con un cornercase real en macOS, el mapeo de procesos a panes con `CORNERCASE_PANE`/`CORNERCASE_SERVER` de procesos reales, `ri_phys_footprint` frente a la memoria que muestra cornercase, la notificación del escritorio y el coste de CPU. Anotarlo como limitación conocida de 0.1.0 hasta que alguien lo pruebe en un Mac.
