// ─── Fragrance Vending Machine — ESP32 Firmware ───────────────────────────
// UART2 (GPIO16=RX2, GPIO17=TX2) → Pi
// UART0 (USB)                    → Serial Monitor / debug
// GPIO2                          → motor output (transistor → Glade)
// GPIO4                          → LED indicador (cliente ve y presiona botón)
// GPIO15                         → botón físico del cliente
// ──────────────────────────────────────────────────────────────────────────

#define MOTOR_PIN      2      // GPIO2 → Base del transistor → botón del Glade
#define LED_PIN        4      // GPIO4 → LED indicador para el cliente
#define BUTTON_PIN     15     // GPIO15 → botón físico del cliente
#define DISPENSE_MS    200    // pulso corto — el 555 maneja la duración del motor
#define PING_INTERVAL  1000   // heartbeat al Pi cada 1s
#define BUTTON_TIMEOUT 30000  // 30s máximo esperando que el cliente presione

unsigned long lastPing      = 0;
bool          waitingButton = false;  // true cuando esperamos que el cliente presione
String        piBuffer      = "";
String        usbBuffer     = "";

void setup() {
  Serial.begin(115200);                          // USB debug
  Serial2.begin(9600, SERIAL_8N1, 16, 17);      // Pi UART

  pinMode(MOTOR_PIN, OUTPUT);
  digitalWrite(MOTOR_PIN, LOW);

  pinMode(LED_PIN, OUTPUT);
  digitalWrite(LED_PIN, LOW);

  pinMode(BUTTON_PIN, INPUT_PULLUP);  // botón conectado a GND

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

  // Esperando que el cliente presione el botón
  if (waitingButton) {
    if (digitalRead(BUTTON_PIN) == LOW) {  // botón presionado (INPUT_PULLUP)
      Serial.println("  Botón presionado — activando motor...");
      digitalWrite(LED_PIN, LOW);
      waitingButton = false;

      digitalWrite(MOTOR_PIN, HIGH);
      delay(DISPENSE_MS);
      digitalWrite(MOTOR_PIN, LOW);

      Serial.println("  Motor apagado. Dispense completado.");
      Serial2.println("OK");  // confirmar al Pi
    }
    return;  // no leer comandos nuevos mientras espera botón
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
    Serial.println("  Pago confirmado — encendiendo LED, esperando botón del cliente...");
    digitalWrite(LED_PIN, HIGH);
    waitingButton = true;

  } else if (cmd == "PING") {
    Serial2.println("PONG");
    Serial.println("  PONG enviado.");

  } else {
    Serial.print("  Comando desconocido: ");
    Serial.println(cmd);
  }
}
