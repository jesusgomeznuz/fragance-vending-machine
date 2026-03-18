# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Contexto del desarrollador

Al iniciar una sesión, leer `~/Library/Mobile Documents/iCloud~md~obsidian/Documents/Obsidian Vault/Máquina expendedora.md` para ver el estado actual del proyecto, decisiones tomadas y qué sigue. Al terminar, actualizar esa nota con lo que se avanzó, decidió o quedó pendiente.

## Commands

```bash
# Compilar y ejecutar
cargo build                                      # Compilar todo (workspace)
cargo run                                        # Máquina dispensadora (puerto 8080)
cargo run -p server                              # Servidor central (puerto 8081)
cargo run -p kiosk                               # Ventana portrait nativa (wry, 450×720)

# Con variables de entorno (Pi en producción)
UART_PORT=/dev/ttyAMA0 SYNC_SERVER_URL=http://192.168.1.131:8081 cargo run

# Checks rápidos
cargo check                                      # Verifica sin compilar binarios
cargo clippy                                     # Linter
cargo test                                       # No hay tests actualmente
```

## Arquitectura

El proyecto es un **workspace Rust** con tres binarios:

| Binario | Crate | Puerto | Descripción |
|---------|-------|--------|-------------|
| `machine` (default) | `src/` | 8080 | Pi — dispensadora + API + UI cliente |
| `server` | `server/` | 8081 | Mac/VPS — agrega métricas de todas las máquinas |
| `kiosk` | `kiosk/` | — | Ventana wry que apunta a localhost:8080 |

### Máquina dispensadora (`src/`)

**AppState** (`src/api/routes.rs`) — compartido entre workers Actix via `Arc<Mutex<Connection>>`:
- `db` — conexión SQLite
- `simulation_mode` — `false` si `MODE=PRODUCTION`
- `machine_id` — de `MACHINE_ID` env var (default: `"1"`)
- `uart` — `Option<UartHandle>`, presente solo si `UART_PORT` está seteado

Al arrancar (`src/main.rs`) se lanzan tres loops Tokio en background:
1. **UART** — solo si `UART_PORT` está seteado; escucha PING/PONG del ESP32
2. **Métricas** — cada 30s lee `/proc/` y guarda en `system_metrics`
3. **Sync** — cada 30s sube registros con `synced_at IS NULL` al servidor central

### Protocolo UART Pi ↔ ESP32 (`src/hardware/uart.rs`)

```
ESP32 → Pi: PING\n           (heartbeat cada 1s)
Pi → ESP32: PONG\n           (respuesta)
Pi → ESP32: DISPENSE\n       (pago confirmado)
ESP32 → Pi: OK\n / FAIL:motivo\n
```

Si no llega PING por >5s, `UartHandle::is_online()` devuelve `false`. El flujo de dispensado usa UART si está online; si no, cae en `Dispenser` (simulación).

### Base de datos SQLite

- **`vending_machine.db`** — local en el Pi; todas las tablas tienen columna `synced_at`
- **`central.db`** — en el servidor; usa restricción UNIQUE `(machine_id, local_id)` para deduplicar reintentos

Tablas principales: `products`, `machine_stock`, `warehouse_stock`, `sales`, `logs`, `purchases`, `stock_transfers`, `sensor_readings`, `system_metrics`.

`db.rs::init_db()` crea el esquema y siembra 5 productos con stock inicial si la DB está vacía.

### Sync offline-first

`src/sync.rs` recoge hasta 100–200 filas por tabla con `synced_at IS NULL`, construye un `SyncPayload` y hace `POST /sync` al servidor central. Solo marca `synced_at` cuando recibe 200 OK. La máquina funciona sin conexión al servidor.

## Variables de entorno

| Variable | Default | Descripción |
|----------|---------|-------------|
| `MACHINE_ID` | `"1"` | ID de la máquina |
| `SYNC_SERVER_URL` | `http://localhost:8081` | URL del servidor central |
| `MODE` | `SIMULATION` | `SIMULATION` o `PRODUCTION` |
| `UART_PORT` | *(desactivado)* | Puerto UART, e.g. `/dev/ttyAMA0` |
| `PORT` *(server)* | `8081` | Puerto del servidor central |
| `DB_PATH` *(server)* | `central.db` | Ruta del DB central |

## Frontend

Servido por `actix-files` desde `frontend/` (máquina) y `server/frontend/` (servidor):
- `index.html` / `app.js` — UI de cliente: carousel → detalle → pago → dispensado
- `operator.html` / `operator.js` — Panel de operador: stock de bodega y transferencias
- `server/frontend/metrics.html` — Dashboard de métricas en tiempo real

## Hardware / ESP32

Firmware en `esp32/dispenser/`. Flujo del dispensador:
1. Pi envía `DISPENSE\n`
2. ESP32 enciende LED (GPIO4)
3. Cliente presiona botón físico (GPIO15)
4. ESP32 activa motor (GPIO2) y responde `OK\n`

Arquitectura multi-dispensador planeada: `DISPENSE:N` para posición 1–5; el Pi mapea `product_id → posición` en DB (el ESP32 no conoce el producto).
## Gestión de tokens

Al iniciar cada sesión, pedir al usuario que corra `/usage` y comparta el output. Con eso estimar cuántos prompts quedan disponibles y priorizar en consecuencia. Si el uso semanal supera el 70%, enfocarse solo en lo que desbloquea avance real — no exploración ni refactors opcionales.
