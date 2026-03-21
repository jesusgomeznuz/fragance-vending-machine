# Fragrance Vending Machine

Máquina expendedora de fragancias — Raspberry Pi 4 + ESP32 + Mercado Pago Point.

---

## Iniciar la máquina (Raspberry Pi)

```bash
ssh pi@192.168.1.157
# contraseña: minecraft21

# Si el servicio de autostart está corriendo, detenerlo primero:
sudo systemctl stop fragrance-vending

# Iniciar manualmente con todas las variables:
cd ~/fragance-vending-machine
source .env
UART_PORT=/dev/ttyAMA0 SYNC_SERVER_URL=http://192.168.1.131:8081 cargo run
```

Abre `http://192.168.1.157:8080` — UI del cliente.

---

## Iniciar el servidor central (Mac)

```bash
cd ~/fragance-vending-machine
cargo run -p server
```

Dashboard de métricas: `http://localhost:8081`

---

## Modo simulación (sin Pi, sin ESP32)

```bash
cargo run
```

Abre `http://localhost:8080`. El pago y el dispensado son simulados.

---

## Conexiones hardware

```
RASPBERRY PI 4                    ESP32 DevKit V1
─────────────────                 ─────────────────
Pin 8  │ GPIO14 (TX) ──────────→ GPIO16 (RX2)
Pin 10 │ GPIO15 (RX) ←────────── GPIO17 (TX2)
Pin 6  │ GND         ──────────── GND
                                  │
                                  ├── GPIO18 → [1kΩ] → Base 2N2222A
                                  │            Colector → Botón Glade S2
                                  │            Emisor   → GND
                                  │
                                  ├── GPIO4  → [220Ω] → LED → GND
                                  │
                                  └── GPIO15 → Botón cliente → GND
```

- Pi y ESP32 son 3.3V — sin level shifter
- TX siempre al RX del otro lado (cruzado)
- GND compartido obligatorio

---

## Reset (base de datos limpia)

```bash
rm -f vending_machine.db central.db logs/system.log
```
