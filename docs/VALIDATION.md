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

- Hace falta cornercase 0.12.187 o posterior (`cornercase --version`); uno anterior rechaza el manifiesto por `min_cornercase` y lo dice en `cornercase plugin list`.
- Al activarlo (`enable`) o con `cornercase plugin list`, cornercase muestra las acciones `cctop` y `split`, el servicio `watch` y los permisos `read`, `notify`, `split`, `focus`.
- Aparece el botón **cctop** al pie de la columna de proyectos (con la barra de iconos, `◔`, con su nombre al pasar el ratón). Con un pane de Claude en pantalla, pulsarlo: se abre cctop en un popup del 80 % de la ventana, con el proyecto, el workspace y la pestaña de ese pane desplegados, el pane seleccionado (y plegado) y el resto plegado; `Tab` muestra el contexto de ese Claude. El ratón funciona dentro (clic, rueda, cabeceras). Desplegar el pane: ni `cctop` ni sus `cornercase status` aparecen en él.
- `q` cierra el popup sin mensaje de error en la ventana (salida con código 0); abrirlo otra vez y cerrarlo con `Esc`: igual. Cerrarlo con la `×`: `pgrep -a cctop` ya no muestra el TUI (solo `cctop watch`).
- Elegir **cctop** en el menú de un proyecto, un workspace, una pestaña y un pane: el popup se abre con esa fila seleccionada.
- Elegir **cctop in a split** en el menú de un pane: se abre un split a su derecha que ejecuta cctop y queda enfocado.
- Repetir la acción del split en la misma pestaña (desde cualquier pane suyo): enfoca el split de cctop, sin abrir otro.
- Salir de cctop con `q` (o `Esc`): el pane se cierra sin dejar una shell. Repetir la acción: abre un split nuevo.
- Con el binario en una ruta con espacios (`cornercase plugin link` desde una carpeta con espacio), la acción sigue abriendo cctop.
- El servicio corre: `pgrep -a cctop` muestra un `cctop watch`, y `cornercase plugin logs cctop` no muestra errores (`cctop watch: …`).

## 2. Un proceso de varios GB (criterio de éxito 1)

En un pane donde corra un agente (o una shell de un workspace con worktree), reservar y tocar 5 GiB (hace falta esa memoria libre):

```sh
python3 -c "b = bytearray(5 * 2**30); b[::4096] = b'x' * (len(b) // 4096); import time; time.sleep(120)"
```

- En cctop (Processes, orden por memoria), el proceso `python3` aparece en menos de 5 s bajo su pane, su pestaña y el workspace correcto (marcado `(worktree)` si lo es), con unos 5 GB, y la ruta caliente (amarillo) llega hasta él. Los totales del pane, la pestaña, el workspace y el proyecto lo incluyen.
- El servicio avisa una sola vez (toast y notificación del escritorio): `cctop: <workspace> › <pane> 5.x GB — python3 -c … 5.x GB`. No repite mientras siga por encima.
- `k` (o clic derecho) sobre el proceso: el diálogo nombra el proceso, su pid, su grupo y su pane; `Esc` lo cierra sin matar nada; `Process` lo termina y desaparece en el siguiente refresco.
- Repetir con el umbral de CPU si se quiere: `cpu_threshold = 50` y `cpu_sustain = "10s"` en `$CORNERCASE_PLUGIN_CONFIG/config.toml` (la carpeta de configuración que cornercase da al plugin; ver design/config.md), reiniciar el servicio (`cornercase plugin disable cctop && cornercase plugin enable cctop`) y lanzar `yes > /dev/null` en un pane.
- Evidencia: captura de cctop con el árbol desplegado y del aviso.

## 3. El contexto frente a cornercase (criterio de éxito 2)

En un pane con una sesión de Claude en este repositorio (o en el de cornercase), tras al menos una respuesta:

- Seleccionar el pane en Processes y pulsar `Tab` (o la pestaña `Context`): a la izquierda, la lista de agentes con todos los Claude del servidor, el más lleno primero, y marcado (`▶`) el del pane seleccionado; sin selección, o si la selección no tiene Claude, el más lleno. Arriba, en la franja de las gráficas, el agente marcado (lugar, `claude`, modelo y estado), `Context used N of W tokens`, su % y la barra (verde, amarilla o roja), con la leyenda `loaded on every message` / `this conversation` / `free` en el borde. A la derecha, `what fills this conversation` por grupos (`Commands`, `Files`, `Searches`, `Claude's replies`, `Your messages`…) con su total, su % y sus tres partes mayores (sin el `cd <carpeta> && ` delante); abajo, `loaded on every message` en una línea con `AGENTS.md`, `DESIGN.md`, skills…, la línea de `/compact` y la nota de que el reparto es estimado.
- Pasar de Processes a Context y volver: la franja de arriba y la caja de abajo no se mueven, solo cambia su contenido. Clic en un grupo (o `Enter`, que abre o pliega todos) lo pliega y despliega.
- El % de la franja (el total real sobre la ventana) y el de ese agente en la lista de agentes (que es el de cornercase) deben quedar a ±2 puntos, y lo mismo frente a la barra de cornercase. Unos puntos de diferencia son normales (cornercase redondea y refresca a su ritmo); repetir la lectura tras una respuesta nueva.
- Clic en otro agente de la lista (o `↑`/`↓`): el desglose pasa a ese agente al momento. Volver a Processes y hacer doble clic en un pane con Claude: abre Context con ese agente; un doble clic en un pane sin Claude solo lo selecciona.
- Con un terminal alto y ancho, en Processes se ven las gráficas `cpu` y `mem` de la última hora, la barra de memoria y el sparkline `CPU 60s` por fila; al encoger el pane las gráficas desaparecen antes de que la tabla pierda líneas.
- `Esc` sale de cctop (como `q`); con el diálogo de matar abierto, solo lo cierra.
- Hacer `/compact` en la sesión: tras la siguiente respuesta, `this conversation` se vacía y vuelve a crecer, y `loaded on every message` se queda.
- Si la versión de Claude Code (`claude --version`) ya no es 2.1.306, capturar un transcript nuevo como fixture **sintética** (rutas y datos personales reemplazados) en `tests/fixtures/claude/<versión>/`.
- Evidencia: captura de la vista Context y del % de cornercase en el mismo momento.

## 4. CPU en reposo con unos 30 panes (requisito no funcional)

Medida en desarrollo (2026-10-21, VM de 8 vCPU, 58 panes, unos 126 procesos del usuario, TUI en una pseudo-terminal 60 s): antes de 017, **3–4 % de un núcleo** (muestreo de `/proc` y árbol unos 30 ms por ciclo, cada `cornercase status --json` unos 45 ms en el hijo); después, **0,9–1,2 %**. 017 abarató la muestra (dueño por los metadatos de la carpeta, `statm` en vez de `status` sin swap, `cmdline`/`environ` leídos una vez por proceso), pide `status` cada 10 s o al aparecer un pane nuevo (el agente, su estado y el % de contexto pueden ir hasta 10 s tarde) y solo redibuja cuando algo cambia. Falta medirlo con unos 30 panes, que es lo que pide el requisito; con la mitad de procesos debería quedar por debajo del 1 %. Detalle, medidas y lo descartado en [design/processes.md](../design/processes.md) (coste de una muestra) y [design/tui.md](../design/tui.md) (coste en reposo). Para repetir la parte de muestreo:

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
- Tras 018 (gráficas, barras y lista de agentes), medido en 120×40 frente a la build anterior: igual dentro del ruido en Processes; detalle en [design/tui.md](../design/tui.md) (coste en reposo).
- Si con unos 30 panes pasa del 1 %, perfilar (`perf record -g -p $pid`) y anotar la medida en `design/tui.md` antes de abrir otra tarea. `cctop watch` sigue pidiendo `status` cada 5 s (unos 0,9 % en la máquina medida).

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
