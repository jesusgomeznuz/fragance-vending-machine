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
    local_id:   i64,
    product_id: i64,
    quantity_g: f64,
    cost:       Option<f64>,
    timestamp:  String,
    notes:      String,
}

#[derive(Serialize)]
struct TransferSyncRecord {
    local_id:   i64,
    product_id: i64,
    quantity_g: f64,
    timestamp:  String,
    notes:      String,
}

#[derive(Serialize)]
struct MetricSyncRecord {
    local_id:        i64,
    timestamp:       String,
    cpu_temp_c:      Option<f64>,
    cpu_load_1m:     Option<f64>,
    cpu_load_5m:     Option<f64>,
    cpu_load_15m:    Option<f64>,
    cpu_usage_pct:   Option<f64>,
    mem_total_mb:    Option<f64>,
    mem_used_mb:     Option<f64>,
    mem_available_mb: Option<f64>,
    disk_total_gb:   Option<f64>,
    disk_used_gb:    Option<f64>,
    uptime_s:        Option<f64>,
}

#[derive(Serialize)]
struct SyncPayload {
    machine_id: i64,
    sales:      Vec<SaleSyncRecord>,
    events:     Vec<EventSyncRecord>,
    purchases:  Vec<PurchaseSyncRecord>,
    transfers:  Vec<TransferSyncRecord>,
    metrics:    Vec<MetricSyncRecord>,
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
    let (sales, events, purchases, transfers, metrics) = {
        let conn = db.lock().unwrap();
        (
            fetch_unsynced_sales(&conn)?,
            fetch_unsynced_events(&conn)?,
            fetch_unsynced_purchases(&conn)?,
            fetch_unsynced_transfers(&conn)?,
            fetch_unsynced_metrics(&conn)?,
        )
    };

    if sales.is_empty() && events.is_empty() && purchases.is_empty()
        && transfers.is_empty() && metrics.is_empty()
    {
        return Ok(0);
    }

    let sale_ids:     Vec<i64> = sales.iter().map(|s| s.local_id).collect();
    let event_ids:    Vec<i64> = events.iter().map(|e| e.local_id).collect();
    let purchase_ids: Vec<i64> = purchases.iter().map(|p| p.local_id).collect();
    let transfer_ids: Vec<i64> = transfers.iter().map(|t| t.local_id).collect();
    let metric_ids:   Vec<i64> = metrics.iter().map(|m| m.local_id).collect();
    let total = sale_ids.len() + event_ids.len() + purchase_ids.len()
              + transfer_ids.len() + metric_ids.len();

    let payload = SyncPayload { machine_id, sales, events, purchases, transfers, metrics };

    let res = client
        .post(format!("{server_url}/sync"))
        .json(&payload)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(format!("server returned {}", res.status()).into());
    }

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
    for id in &metric_ids {
        conn.execute("UPDATE system_metrics SET synced_at = CURRENT_TIMESTAMP WHERE id = ?", params![id])?;
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
        "SELECT id, product_id, quantity_g, cost, timestamp, COALESCE(notes, '') AS notes
         FROM purchases
         WHERE synced_at IS NULL
         ORDER BY id
         LIMIT 100",
    )?;
    stmt.query_map([], |row| {
        Ok(PurchaseSyncRecord {
            local_id:   row.get(0)?,
            product_id: row.get(1)?,
            quantity_g: row.get(2)?,
            cost:        row.get(3)?,
            timestamp:   row.get(4)?,
            notes:       row.get(5)?,
        })
    })?
    .collect()
}

fn fetch_unsynced_transfers(conn: &Connection) -> rusqlite::Result<Vec<TransferSyncRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, product_id, quantity_g, timestamp, COALESCE(notes, '') AS notes
         FROM stock_transfers
         WHERE synced_at IS NULL
         ORDER BY id
         LIMIT 100",
    )?;
    stmt.query_map([], |row| {
        Ok(TransferSyncRecord {
            local_id:   row.get(0)?,
            product_id: row.get(1)?,
            quantity_g: row.get(2)?,
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

fn fetch_unsynced_metrics(conn: &Connection) -> rusqlite::Result<Vec<MetricSyncRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, cpu_temp_c, cpu_load_1m, cpu_load_5m, cpu_load_15m,
                cpu_usage_pct, mem_total_mb, mem_used_mb, mem_available_mb,
                disk_total_gb, disk_used_gb, uptime_s
         FROM system_metrics
         WHERE synced_at IS NULL
         ORDER BY id
         LIMIT 200",
    )?;
    stmt.query_map([], |row| {
        Ok(MetricSyncRecord {
            local_id:         row.get(0)?,
            timestamp:        row.get(1)?,
            cpu_temp_c:       row.get(2)?,
            cpu_load_1m:      row.get(3)?,
            cpu_load_5m:      row.get(4)?,
            cpu_load_15m:     row.get(5)?,
            cpu_usage_pct:    row.get(6)?,
            mem_total_mb:     row.get(7)?,
            mem_used_mb:      row.get(8)?,
            mem_available_mb: row.get(9)?,
            disk_total_gb:    row.get(10)?,
            disk_used_gb:     row.get(11)?,
            uptime_s:         row.get(12)?,
        })
    })?
    .collect()
}
