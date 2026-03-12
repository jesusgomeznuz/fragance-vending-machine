# Fragrance Vending Machine — Roadmap

Última actualización: 2026-03-07

---

## Lo que está hecho hoy

### Software (máquina)
- Servidor Rust con actix-web, SQLite local
- Flujo completo: selección → pago simulado → dispense
- UI kiosk en portrait: carrusel libre, panel de detalle con imagen hero, panel de pago, panel de dispense
- Imágenes de producto como fondo de card (`frontend/images/{id}.png`)
- Descripciones de ocasión por perfume (en panel de detalle, no en el carrusel)
- Operador panel con inventario, compras, transferencias

### Software (servidor central)
- Servidor Rust separado (`cargo run -p server`, puerto 8081)
- Recibe sync de ventas, eventos, compras y transferencias cada 30s
- `central.db` con todos los registros de todas las máquinas
- `synced_at` por registro — si el sync falla, reintenta automáticamente

### Protocolo de sync
- Máquina funciona offline, empuja cuando hay conexión
- Solo marca `synced_at` cuando el servidor confirma 200 OK

---

## Hardware confirmado

| Componente | Estado |
|-----------|--------|
| Pantalla táctil | Llega domingo |
| Raspberry Pi 4 (4GB) | Llega siguiente semana |
| ESP32 | En mano |
| Mecanismo dispensador (Glade modificado, sin 555) | En mano |
| Celda de carga | Por comprar (~$3-8 USD) |
| Dongle USB 4G (Huawei E3372 o similar) | Por comprar |

---

## Pendientes inmediatos (esta semana)

### Domingo — llega pantalla
- [ ] Conectar pantalla a Mac
- [ ] Confirmar que la UI corre fluida en portrait en pantalla real
- [ ] Ajustar proporciones si algo se ve diferente en hardware real

### Siguiente semana — llega Raspberry Pi
- [ ] Instalar Raspberry Pi OS
- [ ] Instalar Rust en el Pi
- [ ] Clonar repo y `cargo run` en el Pi
- [ ] Conectar ESP32 al Pi por UART (TX/RX directo, no USB)
- [ ] Conectar mecanismo dispensador al ESP32
- [ ] Primer dispense físico end-to-end

---

## Pendientes en espera

- [ ] Respuesta de Clip (sdk@payclip.com) — precio y disponibilidad del Pin Pad para integración API
  - Si precio < $100 USD: incluir en el prototipo
  - Si precio razonable: integrar Clip PinPad API (POST /payment-intent → webhook → dispense)

---

## Decisiones de arquitectura tomadas

### Unidades de medición
- **Gramos** son la fuente de verdad en toda la cadena
- Máquina: pesas (celda de carga) miden antes y después de cada dispense
- Warehouse: estimado — no necesita precisión de frontline
- Los ml de la etiqueta del proveedor se ignoran; se pesa todo al recibir
- Conversión a ml solo para UI del cliente (estimada, no crítica)

### Comunicación Pi ↔ ESP32
- UART serial directo (no USB) — más robusto, sin drivers
- Protocolo heartbeat bidireccional cada 5s:
  ```
  Pi  → ESP32: PING
  ESP32 → Pi:  PONG
  ```
- Si ESP32 no recibe PING en 15s → apaga todos los motores (estado seguro)
- Si Pi no recibe PONG en 15s → loggea falla → alerta al servidor central
- Comandos de dispense:
  ```
  Pi  → ESP32: DISPENSE:3
  ESP32 → Pi:  OK:1.4g       (éxito, gramos medidos)
  ESP32 → Pi:  FAIL:no_flow  (sensor sin diferencia)
  ESP32 → Pi:  FAIL:timeout  (motor no respondió)
  ```

### Conectividad de la máquina en campo
- Dongle USB 4G (SIM propia) — no depender del WiFi del centro comercial
- El Pi puede actuar como hotspot para el Pin Pad de Clip si se necesita
- Idealmente cada dispositivo con su propia conexión

### Medición de dispense
- Celda de carga por canal (una por perfume)
- Lee peso antes del dispense → activa motor → lee peso después
- Delta = gramos dispensados
- Si delta < umbral mínimo → FAIL reportado al Pi

---

## Dashboard del operador (por construir)

Accesible desde cualquier dispositivo, por máquina:

```
Máquina #1 — Centro Comercial Galerías
  ESP32:        online / sin respuesta desde HH:MM
  Temperatura Pi: 42°C
  RAM:          23%
  Último sync:  hace 28s

  Stock por perfume (en gramos):
  Dior Sauvage      180g  ████████░░
  Acqua di Gio      120g  ██████░░░░
  YSL Black Opium    40g  ██░░░░░░░░  ⚠ stock bajo

  Hoy: 12 ventas / $54.00
```

### Notificaciones (solo cuando se requiere acción)
- Stock bajo → hay que recargar
- ESP32 sin respuesta → revisar físicamente
- 3 fallos de dispense consecutivos → falla mecánica
- Temperatura Pi > 75°C → riesgo de throttling

### Gráficas en tiempo real (datos cada 30s)
- Temperatura del Pi a lo largo del día
- Ventas por hora
- Stock de cada perfume bajando con el tiempo
- Ratio de éxito/fallo de dispenses

---

## Prueba de stress (antes de salir a campo)

Objetivo: validar límites reales del sistema antes de instalarlo en producción.

- Dispensar 200+ veces seguidas por canal
- Medir si el delta de gramos se mantiene consistente o deriva
- Monitorear temperatura del Pi durante operación sostenida
- Verificar que no hay memory leaks (RAM estable después de 8h)
- Verificar que el sync no acumula registros sin limpiar
- Verificar que el ESP32 no se cuelga después de X operaciones

Los datos de esta prueba definen los límites operacionales reales de la máquina.

---

## Optimizaciones futuras (sin prisa)

- Migrar a Pi Zero 2W ($15) si el uso de recursos en campo es < 30-40% sostenido
- Migrar a ESP32-C3 ($2) si el costo por máquina importa a escala
- Calcular densidad real de cada perfume con datos acumulados de recargas
  (peso conocido / volumen en etiqueta → densidad propia por producto)
- Múltiples máquinas en el dashboard con vista consolidada

---

## Stack técnico

| Capa | Tecnología |
|------|-----------|
| Máquina (backend) | Rust, actix-web 4, SQLite (rusqlite bundled) |
| Máquina (frontend) | HTML/CSS/JS vanilla, portrait kiosk |
| Servidor central | Rust, actix-web 4, SQLite |
| Comunicación Pi↔ESP32 | UART serial directo |
| Firmware ESP32 | Arduino IDE (C++) |
| Medición de producto | Celda de carga, HX711 (ADC para ESP32) |
| Pagos (producción) | Clip PinPad + Point of Sale API |
| Conectividad campo | Dongle USB 4G |
| Deploy Pi | systemd service + Chromium kiosk mode |
