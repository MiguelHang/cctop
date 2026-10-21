# 015 — Release con binarios y `[download]`

- **Estado**: hecho
- **Depende de**: 001 (y 014 para el manifiesto final); idealmente al final del MVP para publicar 0.1.0
- **Paralelo con**: 010, 011, 012, 013
- **Estimación**: ~250 líneas (YAML + script + manifiesto + docs)

## Objetivo
Un tag `v<version>` publica archivos `.tar.gz` para los cuatro targets y el manifiesto apunta a ellos, de modo que `cornercase plugin install MiguelHang/cctop` no compila.

## Alcance
- `.github/workflows/release.yml`: en push de tag `v*`, matriz `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` (runner arm o `cross`), `x86_64-apple-darwin`, `aarch64-apple-darwin`; `cargo build --release --locked --target`; empaqueta el binario como `target/release/cctop` dentro del archivo (misma ruta que `[[build]]`, así los comandos del manifiesto valen para ambos); sube los archivos y sus sha256 a la release de GitHub.
- Actualizar el manifiesto: cornercase vuelve a leer el manifiesto del commit instalado y lo compara, así que las URLs y sha256 deben estar en el repo. Flujo recomendado: el workflow, tras publicar, abre un commit `chore: point downloads at v<version>` (o una PR) con `[download.<target>] url/sha256` (script `.github/downloads.sh`). Ver la decisión pendiente en el informe del planner.
- `README.md` (inglés): qué es, instalación (`cornercase plugin install`, `enable`), uso, configuración; topic `cornercase-plugin` en el repo (manual).
- Comprobación en CI de que `version` de `Cargo.toml` y `cornercase-plugin.toml` coinciden.

## Criterios de aceptación
- CA1: el workflow pasa `actionlint` (o validación YAML) y un dry-run en una rama/tag de prueba genera los cuatro archivos.
- CA2: cada archivo contiene `target/release/cctop` ejecutable y su sha256 coincide con el del manifiesto.
- CA3: el check de versiones falla si difieren.
- CA4 (manual): `cornercase plugin install MiguelHang/cctop` en Linux descarga en vez de compilar.

## Tests requeridos
- Script de comprobación de versiones en el job de guardrails.

## Design a actualizar
- `design/plugin.md` (flujo de release, ruta del binario en el archivo). Nuevo `design/releases.md` si crece.
