# 024 — Aviso de versión nueva

- **Estado**: pendiente (después de publicar 0.1.0)
- **Depende de**: release 0.1.0

## Objetivo
cornercase no avisa de que un plugin tiene versión nueva: hay que lanzar `cornercase plugin update`. Pedido por el usuario (2026-10-25): que cctop lo diga.

## Alcance (a concretar al planificar)
- Como mucho una vez al día, consultar la última release de `MiguelHang/cctop` en GitHub (sin token), cachear el resultado en la carpeta de estado y no bloquear nunca el arranque ni el refresco.
- Si es mayor que la versión propia, mostrar en el pie de la TUI `0.2.0 available · cornercase plugin update cctop`.
- Sin red o con error: nada visible. Desactivable en `config.toml`.
- Valorar también proponer a cornercase que avise él de updates de plugins (mejor para todos los plugins).
