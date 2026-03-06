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
    pub local_id:    i64,
    pub product_id:  i64,
    pub quantity_ml: f64,
    pub cost:        Option<f64>,
    pub timestamp:   String,
    pub notes:       String,
}

#[derive(Deserialize, Serialize)]
pub struct TransferSyncRecord {
    pub local_id:    i64,
    pub product_id:  i64,
    pub quantity_ml: f64,
    pub timestamp:   String,
    pub notes:       String,
}

#[derive(Deserialize)]
pub struct SyncPayload {
    pub machine_id: i64,
    pub sales:      Vec<SaleSyncRecord>,
    pub events:     Vec<EventSyncRecord>,
    pub purchases:  Vec<PurchaseSyncRecord>,
    pub transfers:  Vec<TransferSyncRecord>,
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
            quantity_ml REAL    NOT NULL,
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
            quantity_ml REAL    NOT NULL,
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
            quantity_ml REAL    NOT NULL,
            source      TEXT    NOT NULL DEFAULT 'SENSOR',
            timestamp   TEXT    NOT NULL,
            received_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
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
                payload.machine_id,
                sale.local_id,
                sale.product_id,
                sale.timestamp,
                sale.status,
                sale.payment_method
            ],
        )?;
    }

    for event in &payload.events {
        stored += conn.execute(
            "INSERT OR IGNORE INTO events
                (machine_id, local_id, timestamp, event, details)
             VALUES (?, ?, ?, ?, ?)",
            params![
                payload.machine_id,
                event.local_id,
                event.timestamp,
                event.event,
                event.details
            ],
        )?;
    }

    for purchase in &payload.purchases {
        stored += conn.execute(
            "INSERT OR IGNORE INTO purchases
                (machine_id, local_id, product_id, quantity_ml, cost, timestamp, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                payload.machine_id,
                purchase.local_id,
                purchase.product_id,
                purchase.quantity_ml,
                purchase.cost,
                purchase.timestamp,
                purchase.notes
            ],
        )?;
    }

    for transfer in &payload.transfers {
        stored += conn.execute(
            "INSERT OR IGNORE INTO stock_transfers
                (machine_id, local_id, product_id, quantity_ml, timestamp, notes)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                payload.machine_id,
                transfer.local_id,
                transfer.product_id,
                transfer.quantity_ml,
                transfer.timestamp,
                transfer.notes
            ],
        )?;
    }

    Ok(stored)
}
