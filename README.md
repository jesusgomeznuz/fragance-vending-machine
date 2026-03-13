# Fragrance Vending Machine

Sistema de vending machine de perfumes con inventario en mililitros, panel de operador y sincronización a servidor central.

## Requisitos

- [Rust](https://rustup.rs) (instala `rustup`, incluye `cargo`)
- [Git](https://git-scm.com)

## Instalación

```bash
git clone https://github.com/jesusgomeznuz/fragance-vending-machine.git
cd fragance-vending-machine
```

---

## Correr la máquina

La máquina crea su propia base de datos local (`vending_machine.db`) al primer arranque.

**Mac / Linux**
```bash
cargo run
```

**Windows (PowerShell)**
```powershell
cargo run
```

Abre `http://localhost:8080` — UI del cliente.
Abre `http://localhost:8080/operator.html` — Panel del operador.

---

## Correr el servidor central (simulación de nube)

En una segunda terminal, desde la misma carpeta:

**Mac / Linux**
```bash
cargo run -p server
```

**Windows (PowerShell)**
```powershell
cargo run -p server
```

El servidor escucha en `http://localhost:8081` y crea `central.db`.
La máquina sincroniza automáticamente cada 30 segundos.

---

## Variables de entorno

| Variable | Default | Descripción |
|---|---|---|
| `MACHINE_ID` | `1` | ID numérico de esta máquina |
| `SYNC_SERVER_URL` | `http://localhost:8081` | URL del servidor central |
| `MODE` | `SIMULATION` | `SIMULATION` o `PRODUCTION` |
| `UART_PORT` | *(desactivado)* | Puerto serial del ESP32, e.g. `/dev/ttyAMA0` |
| `PORT` *(server)* | `8081` | Puerto del servidor central |

**Mac / Linux**
```bash
MACHINE_ID=2 cargo run
MACHINE_ID=2 SYNC_SERVER_URL=https://mi-servidor.com cargo run
```

**Windows (PowerShell)**
```powershell
$env:MACHINE_ID=2; cargo run
$env:MACHINE_ID=2; $env:SYNC_SERVER_URL="https://mi-servidor.com"; cargo run
```

**Windows (CMD)**
```cmd
set MACHINE_ID=2 && cargo run
```

---

## Reset (base de datos limpia)

**Mac / Linux**
```bash
rm -f vending_machine.db central.db logs/system.log
```

**Windows (PowerShell)**
```powershell
Remove-Item -ErrorAction SilentlyContinue vending_machine.db, central.db, logs/system.log
```

---

## Estructura del proyecto

```
fragrance-vending-machine/
  src/          — binario machine (puerto 8080)
  server/       — binario server  (puerto 8081)
  frontend/     — UI cliente + panel operador (HTML/CSS/JS)
  database/     — schema SQL de referencia
```

## Raspberry Pi (producción)

El servidor corre automáticamente al encender el Pi via systemd.

**Ver logs en tiempo real:**
```bash
ssh pi@192.168.1.157
sudo journalctl -u vending.service -f
```

**Arrancar/detener manualmente:**
```bash
sudo systemctl start vending.service
sudo systemctl stop vending.service
```

**Correr con UART y sync (desarrollo):**
```bash
SYNC_SERVER_URL=http://192.168.1.131:8081 UART_PORT=/dev/ttyAMA0 cargo run
```

**Actualizar tras un git pull:**
```bash
cargo build --release && sudo systemctl restart vending.service
```

---

## Kiosk (ventana portrait nativa)

```bash
cargo run -p kiosk
```

Abre una ventana portrait apuntando a `http://localhost:8080`.

---

## Flujo de demo

1. Correr el servidor central (Terminal 1)
2. Correr la máquina (Terminal 2)
3. Abrir el panel de operador → agregar stock al warehouse → transferir a la máquina
4. Abrir la UI del cliente → seleccionar producto → comprar
5. Después de 30s, la venta aparece en `central.db`
