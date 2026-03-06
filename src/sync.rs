use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::{params, Connection};
use serde::Serialize;
use tokio::time::interval;

#[derive(Serialize)]
struct SaleSyncRecord {
    local_id:       i64,
    product_id:     i64,
    timestamp:      String,
    status:         String,
    payment_method: String,
}

#[derive(Serialize)]
struct EventSyncRecord {
    local_id:  i64,
    timestamp: String,
    event:     String,
    details:   String,
}

#[derive(Serialize)]
struct PurchaseSyncRecord {
    local_id:    i64,
    product_id:  i64,
    quantity_ml: f64,
    cost:        Option<f64>,
    timestamp:   String,
    notes:       String,
}

#[derive(Serialize)]
struct TransferSyncRecord {
    local_id:    i64,
    product_id:  i64,
    quantity_ml: f64,
    timestamp:   String,
    notes:       String,
}

#[derive(Serialize)]
struct SyncPayload {
    machine_id: i64,
    sales:      Vec<SaleSyncRecord>,
    events:     Vec<EventSyncRecord>,
    purchases:  Vec<PurchaseSyncRecord>,
    transfers:  Vec<TransferSyncRecord>,
}

/// Starts a background loop that tries to push unsynced records to the central
/// server every 30 seconds. If the server is unreachable, it logs a warning
/// and retries on the next tick — the machine keeps working either way.
pub async fn start_sync_loop(
    db:         Arc<Mutex<Connection>>,
    machine_id: i64,
    server_url: String,
) {
    let client = reqwest::Client::new();
    let mut tick = interval(Duration::from_secs(30));

    loop {
        tick.tick().await;
        match run_sync(&db, machine_id, &server_url, &client).await {
            Ok(0)  => {}
            Ok(n)  => log::info!("Sync: {n} records pushed to central server ({server_url})"),
            Err(e) => log::warn!("Sync unavailable (will retry in 30s): {e}"),
        }
    }
}

async fn run_sync(
    db:         &Arc<Mutex<Connection>>,
    machine_id: i64,
    server_url: &str,
    client:     &reqwest::Client,
) -> Result<usize, Box<dyn std::error::Error>> {
    // --- Read unsynced records (hold lock briefly, then release) ---
    let (sales, events, purchases, transfers) = {
        let conn = db.lock().unwrap();
        (
            fetch_unsynced_sales(&conn)?,
            fetch_unsynced_events(&conn)?,
            fetch_unsynced_purchases(&conn)?,
            fetch_unsynced_transfers(&conn)?,
        )
    };

    if sales.is_empty() && events.is_empty() && purchases.is_empty() && transfers.is_empty() {
        return Ok(0);
    }

    let sale_ids:     Vec<i64> = sales.iter().map(|s| s.local_id).collect();
    let event_ids:    Vec<i64> = events.iter().map(|e| e.local_id).collect();
    let purchase_ids: Vec<i64> = purchases.iter().map(|p| p.local_id).collect();
    let transfer_ids: Vec<i64> = transfers.iter().map(|t| t.local_id).collect();
    let total = sale_ids.len() + event_ids.len() + purchase_ids.len() + transfer_ids.len();

    let payload = SyncPayload { machine_id, sales, events, purchases, transfers };

    // --- POST to central server (no lock held during network call) ---
    let res = client
        .post(format!("{server_url}/sync"))
        .json(&payload)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(format!("server returned {}", res.status()).into());
    }

    // --- Mark as synced ---
    let conn = db.lock().unwrap();
    for id in &sale_ids {
        conn.execute("UPDATE sales SET synced_at = CURRENT_TIMESTAMP WHERE id = ?", params![id])?;
    }
    for id in &event_ids {
        conn.execute("UPDATE logs SET synced_at = CURRENT_TIMESTAMP WHERE id = ?", params![id])?;
    }
    for id in &purchase_ids {
        conn.execute("UPDATE purchases SET synced_at = CURRENT_TIMESTAMP WHERE id = ?", params![id])?;
    }
    for id in &transfer_ids {
        conn.execute("UPDATE stock_transfers SET synced_at = CURRENT_TIMESTAMP WHERE id = ?", params![id])?;
    }

    Ok(total)
}

fn fetch_unsynced_sales(conn: &Connection) -> rusqlite::Result<Vec<SaleSyncRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, product_id, timestamp, status, payment_method
         FROM sales
         WHERE synced_at IS NULL
         ORDER BY id
         LIMIT 100",
    )?;
    stmt.query_map([], |row| {
        Ok(SaleSyncRecord {
            local_id:       row.get(0)?,
            product_id:     row.get(1)?,
            timestamp:      row.get(2)?,
            status:         row.get(3)?,
            payment_method: row.get(4)?,
        })
    })?
    .collect()
}

fn fetch_unsynced_purchases(conn: &Connection) -> rusqlite::Result<Vec<PurchaseSyncRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, product_id, quantity_ml, cost, timestamp, COALESCE(notes, '') AS notes
         FROM purchases
         WHERE synced_at IS NULL
         ORDER BY id
         LIMIT 100",
    )?;
    stmt.query_map([], |row| {
        Ok(PurchaseSyncRecord {
            local_id:    row.get(0)?,
            product_id:  row.get(1)?,
            quantity_ml: row.get(2)?,
            cost:        row.get(3)?,
            timestamp:   row.get(4)?,
            notes:       row.get(5)?,
        })
    })?
    .collect()
}

fn fetch_unsynced_transfers(conn: &Connection) -> rusqlite::Result<Vec<TransferSyncRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, product_id, quantity_ml, timestamp, COALESCE(notes, '') AS notes
         FROM stock_transfers
         WHERE synced_at IS NULL
         ORDER BY id
         LIMIT 100",
    )?;
    stmt.query_map([], |row| {
        Ok(TransferSyncRecord {
            local_id:    row.get(0)?,
            product_id:  row.get(1)?,
            quantity_ml: row.get(2)?,
            timestamp:   row.get(3)?,
            notes:       row.get(4)?,
        })
    })?
    .collect()
}

fn fetch_unsynced_events(conn: &Connection) -> rusqlite::Result<Vec<EventSyncRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, event, COALESCE(details, '') AS details
         FROM logs
         WHERE synced_at IS NULL
         ORDER BY id
         LIMIT 100",
    )?;
    stmt.query_map([], |row| {
        Ok(EventSyncRecord {
            local_id:  row.get(0)?,
            timestamp: row.get(1)?,
            event:     row.get(2)?,
            details:   row.get(3)?,
        })
    })?
    .collect()
}
