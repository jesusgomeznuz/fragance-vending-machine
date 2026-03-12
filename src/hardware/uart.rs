// ─── UART — Pi ↔ ESP32 ────────────────────────────────────────────────────
// Protocol:
//   ESP32 → Pi : PING  (every 1 s)
//   Pi → ESP32 : PONG  (reply to PING)
//   Pi → ESP32 : DISPENSE\n  (when a sale is confirmed)
//   ESP32 → Pi : OK    (after motor fires)
//
// Pi detects ESP32 offline when no PING is received for > 5 s.
// ──────────────────────────────────────────────────────────────────────────

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct UartHandle {
    dispense_tx: mpsc::SyncSender<()>,
    pub esp_online: Arc<AtomicBool>,
    last_ping_ms: Arc<AtomicI64>,
}

impl UartHandle {
    /// Returns true if the channel accepted the command (ESP32 may still be offline).
    pub fn send_dispense(&self) -> bool {
        self.dispense_tx.try_send(()).is_ok()
    }

    /// True when a PING was received within the last 5 seconds.
    pub fn is_online(&self) -> bool {
        self.esp_online.load(Ordering::Relaxed)
    }
}

pub fn start_uart(port_path: &str) -> UartHandle {
    let (tx, rx) = mpsc::sync_channel(1);
    let esp_online   = Arc::new(AtomicBool::new(false));
    let last_ping_ms = Arc::new(AtomicI64::new(0));

    let esp_online_c   = esp_online.clone();
    let last_ping_ms_c = last_ping_ms.clone();
    let port_path      = port_path.to_string();

    std::thread::spawn(move || loop {
        match run_uart_loop(&port_path, &rx, &esp_online_c, &last_ping_ms_c) {
            Ok(_) => {}
            Err(e) => {
                log::warn!("UART error: {e} — retrying in 3 s");
                esp_online_c.store(false, Ordering::Relaxed);
                std::thread::sleep(Duration::from_secs(3));
            }
        }
    });

    UartHandle { dispense_tx: tx, esp_online, last_ping_ms }
}

// ── internal ───────────────────────────────────────────────────────────────

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn run_uart_loop(
    port_path:    &str,
    rx:           &mpsc::Receiver<()>,
    esp_online:   &Arc<AtomicBool>,
    last_ping_ms: &Arc<AtomicI64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut port = serialport::new(port_path, 9600)
        .timeout(Duration::from_millis(100))
        .open()?;

    log::info!("UART opened on {port_path} @ 9600 baud");

    let mut buf  = Vec::<u8>::new();
    let mut byte = [0u8; 1];

    loop {
        // ── outbound: dispense command queued by web handler ──────────────
        if rx.try_recv().is_ok() {
            if let Err(e) = port.write_all(b"DISPENSE\n") {
                log::warn!("UART write error: {e}");
            } else {
                log::info!("UART → ESP32: DISPENSE");
            }
        }

        // ── inbound: read one byte at a time ──────────────────────────────
        match port.read(&mut byte) {
            Ok(1) => {
                if byte[0] == b'\n' {
                    if let Ok(line) = std::str::from_utf8(&buf) {
                        handle_line(line.trim(), &mut *port, esp_online, last_ping_ms);
                    }
                    buf.clear();
                } else if byte[0] != b'\r' {
                    buf.push(byte[0]);
                }
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                // Mark offline if silent for > 5 s
                let last = last_ping_ms.load(Ordering::Relaxed);
                if last > 0 && now_ms() - last > 5_000 {
                    if esp_online.swap(false, Ordering::Relaxed) {
                        log::warn!("UART: ESP32 went offline (no PING)");
                    }
                }
            }
            Err(e) => {
                log::warn!("UART read error: {e} — continuing");
                esp_online.store(false, Ordering::Relaxed);
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn handle_line(
    cmd:          &str,
    port:         &mut dyn std::io::Write,
    esp_online:   &Arc<AtomicBool>,
    last_ping_ms: &Arc<AtomicI64>,
) {
    match cmd {
        "PING" => {
            let _ = port.write_all(b"PONG\n");
            last_ping_ms.store(now_ms(), Ordering::Relaxed);
            if !esp_online.swap(true, Ordering::Relaxed) {
                log::info!("UART: ESP32 online");
            }
        }
        "OK" => {
            log::info!("UART ← ESP32: dispense confirmed OK");
        }
        other if !other.is_empty() => {
            log::debug!("UART ← ESP32: '{other}'");
        }
        _ => {}
    }
}
