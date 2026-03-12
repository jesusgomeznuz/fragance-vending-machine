// ─── Fragrance Vending Machine — ESP32 Firmware ───────────────────────────
// UART2 (GPIO16=RX2, GPIO17=TX2) → Pi
// UART0 (USB)                    → Serial Monitor / debug
// GPIO2                          → motor output (built-in LED por ahora)
// ──────────────────────────────────────────────────────────────────────────

#define MOTOR_PIN      2      // GPIO2 → Base del transistor → botón del Glade
#define DISPENSE_MS    200    // pulso corto — el 555 maneja la duración del motor
#define PING_INTERVAL  1000   // heartbeat al Pi cada 1s

unsigned long lastPing   = 0;
String        piBuffer   = "";
String        usbBuffer  = "";

void setup() {
  Serial.begin(115200);                          // USB debug
  Serial2.begin(9600, SERIAL_8N1, 16, 17);      // Pi UART

  pinMode(MOTOR_PIN, OUTPUT);
  digitalWrite(MOTOR_PIN, LOW);

  Serial.println("ESP32 listo.");
  Serial.println("Comandos via Serial Monitor: DISPENSE, PING");
}

void loop() {
  // Heartbeat → Pi
  if (millis() - lastPing >= PING_INTERVAL) {
    Serial2.println("PING");
    Serial.println("[→ Pi] PING");
    lastPing = millis();
  }

  // Leer del Pi (UART2)
  while (Serial2.available()) {
    char c = Serial2.read();
    if (c == '\n') {
      handleCommand(piBuffer, "Pi");
      piBuffer = "";
    } else if (c != '\r') {
      piBuffer += c;
    }
  }

  // Leer del Serial Monitor (para probar sin Pi)
  while (Serial.available()) {
    char c = Serial.read();
    if (c == '\n') {
      handleCommand(usbBuffer, "USB");
      usbBuffer = "";
    } else if (c != '\r') {
      usbBuffer += c;
    }
  }
}

void handleCommand(String cmd, const char* source) {
  cmd.trim();
  if (cmd.length() == 0) return;

  Serial.print("["); Serial.print(source); Serial.print("] → ");
  Serial.println(cmd);

  if (cmd == "PONG") {
    Serial.println("  Pi respondio PONG OK");

  } else if (cmd == "DISPENSE") {
    Serial.println("  Activando motor...");
    digitalWrite(MOTOR_PIN, HIGH);
    delay(DISPENSE_MS);
    digitalWrite(MOTOR_PIN, LOW);
    Serial.println("  Motor apagado. Dispense completado.");
    Serial2.println("OK");  // confirmar al Pi

  } else if (cmd == "PING") {
    Serial2.println("PONG");
    Serial.println("  PONG enviado.");

  } else {
    Serial.print("  Comando desconocido: ");
    Serial.println(cmd);
  }
}
