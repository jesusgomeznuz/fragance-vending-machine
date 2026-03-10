use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

// --- Sync payload types (shared with machine via JSON) ---

#[derive(Deserialize, Serialize)]
pub struct SaleSyncRecord {
    pub local_id:       i64,
    pub product_id:     i64,
    pub timestamp:      String,
    pub status:         String,
    pub payment_method: String,
}

#[derive(Deserialize, Serialize)]
pub struct EventSyncRecord {
    pub local_id:  i64,
    pub timestamp: String,
    pub event:     String,
    pub details:   String,
}

#[derive(Deserialize, Serialize)]
pub struct PurchaseSyncRecord {
    pub local_id:   i64,
    pub product_id: i64,
    pub quantity_g: f64,
    pub cost:       Option<f64>,
    pub timestamp:  String,
    pub notes:      String,
}

#[derive(Deserialize, Serialize)]
pub struct TransferSyncRecord {
    pub local_id:   i64,
    pub product_id: i64,
    pub quantity_g: f64,
    pub timestamp:  String,
    pub notes:      String,
}

#[derive(Deserialize, Serialize)]
pub struct MetricSyncRecord {
    pub local_id:         i64,
    pub timestamp:        String,
    pub cpu_temp_c:       Option<f64>,
    pub cpu_load_1m:      Option<f64>,
    pub cpu_load_5m:      Option<f64>,
    pub cpu_load_15m:     Option<f64>,
    pub cpu_usage_pct:    Option<f64>,
    pub mem_total_mb:     Option<f64>,
    pub mem_used_mb:      Option<f64>,
    pub mem_available_mb: Option<f64>,
    pub disk_total_gb:    Option<f64>,
    pub disk_used_gb:     Option<f64>,
    pub uptime_s:         Option<f64>,
}

#[derive(Deserialize)]
pub struct SyncPayload {
    pub machine_id: i64,
    pub sales:      Vec<SaleSyncRecord>,
    pub events:     Vec<EventSyncRecord>,
    pub purchases:  Vec<PurchaseSyncRecord>,
    pub transfers:  Vec<TransferSyncRecord>,
    #[serde(default)]
    pub metrics:    Vec<MetricSyncRecord>,
}

// --- Schema ---

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sales (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            machine_id     INTEGER NOT NULL,
            local_id       INTEGER NOT NULL,
            product_id     INTEGER NOT NULL,
            timestamp      TEXT    NOT NULL,
            status         TEXT    NOT NULL,
            payment_method TEXT    NOT NULL,
            received_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(machine_id, local_id)
        );

        CREATE TABLE IF NOT EXISTS events (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            machine_id  INTEGER NOT NULL,
            local_id    INTEGER NOT NULL,
            timestamp   TEXT    NOT NULL,
            event       TEXT    NOT NULL,
            details     TEXT,
            received_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(machine_id, local_id)
        );

        CREATE TABLE IF NOT EXISTS purchases (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            machine_id  INTEGER NOT NULL,
            local_id    INTEGER NOT NULL,
            product_id  INTEGER NOT NULL,
            quantity_g  REAL    NOT NULL,
            cost        REAL,
            timestamp   TEXT    NOT NULL,
            notes       TEXT,
            received_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(machine_id, local_id)
        );

        CREATE TABLE IF NOT EXISTS stock_transfers (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            machine_id  INTEGER NOT NULL,
            local_id    INTEGER NOT NULL,
            product_id  INTEGER NOT NULL,
            quantity_g  REAL    NOT NULL,
            timestamp   TEXT    NOT NULL,
            notes       TEXT,
            received_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(machine_id, local_id)
        );

        CREATE TABLE IF NOT EXISTS sensor_readings (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            machine_id  INTEGER NOT NULL,
            local_id    INTEGER NOT NULL,
            product_id  INTEGER NOT NULL,
            quantity_g  REAL    NOT NULL,
            source      TEXT    NOT NULL DEFAULT 'SENSOR',
            timestamp   TEXT    NOT NULL,
            received_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(machine_id, local_id)
        );

        CREATE TABLE IF NOT EXISTS system_metrics (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            machine_id       INTEGER  NOT NULL,
            local_id         INTEGER  NOT NULL,
            timestamp        TEXT     NOT NULL,
            cpu_temp_c       REAL,
            cpu_load_1m      REAL,
            cpu_load_5m      REAL,
            cpu_load_15m     REAL,
            cpu_usage_pct    REAL,
            mem_total_mb     REAL,
            mem_used_mb      REAL,
            mem_available_mb REAL,
            disk_total_gb    REAL,
            disk_used_gb     REAL,
            uptime_s         REAL,
            received_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(machine_id, local_id)
        );
        ",
    )
}

/// Store synced records from a machine. Uses INSERT OR IGNORE so duplicate
/// deliveries (e.g. network retry) are silently skipped.
pub fn store_sync(conn: &Connection, payload: &SyncPayload) -> Result<usize> {
    let mut stored = 0;

    for sale in &payload.sales {
        stored += conn.execute(
            "INSERT OR IGNORE INTO sales
                (machine_id, local_id, product_id, timestamp, status, payment_method)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                payload.machine_id, sale.local_id, sale.product_id,
                sale.timestamp, sale.status, sale.payment_method
            ],
        )?;
    }

    for event in &payload.events {
        stored += conn.execute(
            "INSERT OR IGNORE INTO events
                (machine_id, local_id, timestamp, event, details)
             VALUES (?, ?, ?, ?, ?)",
            params![
                payload.machine_id, event.local_id,
                event.timestamp, event.event, event.details
            ],
        )?;
    }

    for purchase in &payload.purchases {
        stored += conn.execute(
            "INSERT OR IGNORE INTO purchases
                (machine_id, local_id, product_id, quantity_g, cost, timestamp, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                payload.machine_id, purchase.local_id, purchase.product_id,
                purchase.quantity_g, purchase.cost, purchase.timestamp, purchase.notes
            ],
        )?;
    }

    for transfer in &payload.transfers {
        stored += conn.execute(
            "INSERT OR IGNORE INTO stock_transfers
                (machine_id, local_id, product_id, quantity_g, timestamp, notes)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                payload.machine_id, transfer.local_id, transfer.product_id,
                transfer.quantity_g, transfer.timestamp, transfer.notes
            ],
        )?;
    }

    for metric in &payload.metrics {
        stored += conn.execute(
            "INSERT OR IGNORE INTO system_metrics
                (machine_id, local_id, timestamp, cpu_temp_c, cpu_load_1m, cpu_load_5m,
                 cpu_load_15m, cpu_usage_pct, mem_total_mb, mem_used_mb, mem_available_mb,
                 disk_total_gb, disk_used_gb, uptime_s)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                payload.machine_id, metric.local_id, metric.timestamp,
                metric.cpu_temp_c, metric.cpu_load_1m, metric.cpu_load_5m,
                metric.cpu_load_15m, metric.cpu_usage_pct,
                metric.mem_total_mb, metric.mem_used_mb, metric.mem_available_mb,
                metric.disk_total_gb, metric.disk_used_gb, metric.uptime_s,
            ],
        )?;
    }

    Ok(stored)
}
