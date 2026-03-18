# Dimensiones del Gabinete — Máquina Expendedora de Fragancias

## Medidas finales

| Dimensión | Medida |
|---|---|
| **Ancho** | **800mm** |
| **Alto** | **380mm** |
| **Profundo** | **280mm** |

Formato: **sobremesa horizontal (landscape)**

---

## Layout frontal

```
┌─────────────────────────────────────────────────────────────────┐
│          │  [MP]  │  ventana iluminada — frascos visibles       │
│ PANTALLA │        │  [F1]   [F2]   [F3]   [F4]   [F5]         │
│  195×297 │ [COIN] │                                             │
│ portrait │        │  oooo   oooo   oooo   oooo   oooo          │
└─────────────────────────────────────────────────────────────────┘
  ←195mm→  ←100mm→ ←──────────────450mm──────────────→
```

Flujo del usuario: **pantalla → pago → recibe perfume** (izquierda a derecha)

---

## Desglose por zona

### Zona 1 — Pantalla (izquierda)
- Pantalla en modo **portrait (vertical)**
- Medidas del panel: 195mm ancho × 297mm alto
- Profundo: 20mm
- El usuario selecciona la fragancia aquí

### Zona 2 — Pago (centro-izquierda, pegada a la pantalla)
- Ancho: 100mm
- Alto: ~290mm (encaja dentro del alto de la pantalla)
- Contiene apilados verticalmente:
  - **Terminal Mercado Pago Point Smart 2** (tarjeta) — arriba
  - **Aceptador de monedas** — abajo
- Profundo estimado: 75mm (dictado por el coin collector)
- **Nota:** confirmar medidas exactas cuando llegue la terminal (2026-03-18)

### Zona 3 — Display de fragancias (derecha)
- Ancho: 450mm (5 slots × 80mm + gaps)
- Alto: 297mm (misma altura que la pantalla)
- **Ventana iluminada** con iluminación interior (LED)
- Cada slot muestra el frasco/contenedor de la fragancia para identificación visual
- 5 orificios de salida (nozzles) en la parte inferior de cada slot
- Profundo de la ventana de display: 120mm
- Detrás de la ventana: mecanismo Glade S2 oculto (80mm profundo)

---

## Profundo — Corte lateral

```
[panel frontal] [ventana display 120mm] [Glade 80mm] [electrónica 80mm]
     20mm             120mm               80mm            80mm
←——————————————————————————— 280mm total ————————————————————————————→
```

---

## Medidas de los componentes internos (referencia)

| Componente | Alto | Ancho | Profundo |
|---|---|---|---|
| Pantalla | 297mm | 195mm | 20mm |
| Dispensador Glade S2 (×5) | 202mm | 80mm | 80mm |
| Raspberry Pi 4 | 85mm | 56mm | 17mm |
| ESP32 DevKit V1 | 55mm | 28mm | 10mm |
| Aceptador de monedas (estimado) | 110mm | 75mm | 75mm |
| Terminal MP Point Smart 2 | ~180mm | ~80mm | ~22mm |

---

## Notas para el diseño del gabinete

1. **Ventana de display** — se recomienda acrílico o vidrio templado con iluminación LED interior cálida (como referencia: tono dorado/ámbar para estética premium)
2. **Acceso trasero** — puerta o panel desmontable en la parte trasera para dar mantenimiento a los Glades y electrónica
3. **Orificios de nozzle** — 5 orificios pequeños (~15mm diámetro) en la parte frontal inferior de la zona de display, uno por fragancia
4. **Ventilación** — orificios pequeños en la parte superior/lateral para disipar calor del Pi
5. **Base** — la máquina es de sobremesa; necesita gomas antideslizantes en la base o diseño de apoyo sobre mueble
6. **Cable management** — reservar canal interno para cables del Pi → ESP32 (UART) y alimentación
7. **Terminal MP** — confirmar dimensiones exactas cuando llegue (2026-03-18) antes de cortar el panel

---

## Referencia visual de inspiración

Formato tipo vitrina horizontal con pantalla portrait a la izquierda, zona de pago central y display iluminado de productos a la derecha.
