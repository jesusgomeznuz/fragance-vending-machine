use rusqlite::{params, Connection, Result};

use crate::database::models::{InventoryItem, Product};

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS machines (
            id       INTEGER PRIMARY KEY,
            name     TEXT NOT NULL,
            location TEXT
        );

        CREATE TABLE IF NOT EXISTS products (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            price           REAL NOT NULL,
            g_per_dispense REAL NOT NULL DEFAULT 1.5
        );

        CREATE TABLE IF NOT EXISTS warehouse_stock (
            product_id  INTEGER NOT NULL PRIMARY KEY,
            quantity_g REAL    NOT NULL DEFAULT 0.0,
            FOREIGN KEY (product_id) REFERENCES products(id)
        );

        CREATE TABLE IF NOT EXISTS machine_stock (
            machine_id  INTEGER NOT NULL,
            product_id  INTEGER NOT NULL,
            quantity_g REAL    NOT NULL DEFAULT 0.0,
            PRIMARY KEY (machine_id, product_id),
            FOREIGN KEY (machine_id) REFERENCES machines(id),
            FOREIGN KEY (product_id) REFERENCES products(id)
        );

        CREATE TABLE IF NOT EXISTS purchases (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id  INTEGER  NOT NULL,
            quantity_g REAL     NOT NULL,
            cost        REAL,
            timestamp   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            notes       TEXT,
            synced_at   DATETIME DEFAULT NULL,
            FOREIGN KEY (product_id) REFERENCES products(id)
        );

        CREATE TABLE IF NOT EXISTS stock_transfers (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id  INTEGER  NOT NULL,
            machine_id  INTEGER  NOT NULL,
            quantity_g REAL     NOT NULL,
            timestamp   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            notes       TEXT,
            synced_at   DATETIME DEFAULT NULL,
            FOREIGN KEY (product_id) REFERENCES products(id),
            FOREIGN KEY (machine_id) REFERENCES machines(id)
        );

        CREATE TABLE IF NOT EXISTS sales (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id     INTEGER  NOT NULL,
            machine_id     INTEGER  NOT NULL,
            timestamp      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            status         TEXT     NOT NULL,
            payment_method TEXT     NOT NULL,
            synced_at      DATETIME DEFAULT NULL,
            FOREIGN KEY (product_id) REFERENCES products(id),
            FOREIGN KEY (machine_id) REFERENCES machines(id)
        );

        CREATE TABLE IF NOT EXISTS logs (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            machine_id INTEGER  NOT NULL,
            timestamp  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            event      TEXT     NOT NULL,
            details    TEXT,
            synced_at  DATETIME DEFAULT NULL
        );

        -- Physical measurements from sensors (future hardware integration).
        -- Compare against machine_stock.quantity_g to detect losses or theft (in grams).
        -- source: 'SENSOR' (automated) | 'MANUAL' (operator measured by hand)
        CREATE TABLE IF NOT EXISTS sensor_readings (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            machine_id  INTEGER  NOT NULL,
            product_id  INTEGER  NOT NULL,
            quantity_g REAL     NOT NULL,
            source      TEXT     NOT NULL DEFAULT 'SENSOR',
            timestamp   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            synced_at   DATETIME DEFAULT NULL,
            FOREIGN KEY (machine_id) REFERENCES machines(id),
            FOREIGN KEY (product_id) REFERENCES products(id)
        );

        CREATE TABLE IF NOT EXISTS system_metrics (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            machine_id       INTEGER  NOT NULL,
            timestamp        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
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
            synced_at        DATETIME DEFAULT NULL
        );
        ",
    )?;

    // Migration: add synced_at if table existed before this column was added
    conn.execute(
        "ALTER TABLE system_metrics ADD COLUMN synced_at DATETIME DEFAULT NULL",
        [],
    ).ok();

    seed_if_empty(conn)?;
    Ok(())
}

fn seed_if_empty(conn: &Connection) -> Result<()> {
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))?;

    if count > 0 {
        return Ok(());
    }

    conn.execute_batch(
        "
        INSERT OR IGNORE INTO machines (id, name, location)
        VALUES (1, 'Machine 1', 'TBD');

        INSERT INTO products (name, price, g_per_dispense) VALUES ('Jean Paul Gaultier', 5.00, 1.5);
        INSERT INTO products (name, price, g_per_dispense) VALUES ('Dior Sauvage',       4.50, 1.5);
        INSERT INTO products (name, price, g_per_dispense) VALUES ('Versace Eros',       3.75, 1.5);
        INSERT INTO products (name, price, g_per_dispense) VALUES ('Acqua di Gio',       4.00, 1.5);
        INSERT INTO products (name, price, g_per_dispense) VALUES ('YSL Black Opium',    4.25, 1.5);

        -- Warehouse stock: 200g per product (approx. 2 x 100g bottles)
        INSERT INTO warehouse_stock (product_id, quantity_g) VALUES (1, 200.0);
        INSERT INTO warehouse_stock (product_id, quantity_g) VALUES (2, 200.0);
        INSERT INTO warehouse_stock (product_id, quantity_g) VALUES (3, 200.0);
        INSERT INTO warehouse_stock (product_id, quantity_g) VALUES (4, 200.0);
        INSERT INTO warehouse_stock (product_id, quantity_g) VALUES (5, 200.0);

        -- Machine 1 stock: 50g per product
        INSERT INTO machine_stock (machine_id, product_id, quantity_g) VALUES (1, 1, 50.0);
        INSERT INTO machine_stock (machine_id, product_id, quantity_g) VALUES (1, 2, 50.0);
        INSERT INTO machine_stock (machine_id, product_id, quantity_g) VALUES (1, 3, 50.0);
        INSERT INTO machine_stock (machine_id, product_id, quantity_g) VALUES (1, 4, 50.0);
        INSERT INTO machine_stock (machine_id, product_id, quantity_g) VALUES (1, 5, 50.0);
        ",
    )?;

    log::info!("Database seeded with products and initial stock");
    Ok(())
}

pub fn get_products(conn: &Connection, machine_id: i64) -> Result<Vec<Product>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.price, p.g_per_dispense,
                COALESCE(ms.quantity_g, 0.0) AS stock_g
         FROM products p
         LEFT JOIN machine_stock ms ON ms.product_id = p.id AND ms.machine_id = ?
         ORDER BY p.id",
    )?;

    let products = stmt
        .query_map(params![machine_id], |row| {
            Ok(Product {
                id:              row.get(0)?,
                name:            row.get(1)?,
                price:           row.get(2)?,
                g_per_dispense: row.get(3)?,
                stock_g:        row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    Ok(products)
}

pub fn get_product(
    conn: &Connection,
    product_id: i64,
    machine_id: i64,
) -> Result<Option<Product>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.price, p.g_per_dispense,
                COALESCE(ms.quantity_g, 0.0) AS stock_g
         FROM products p
         LEFT JOIN machine_stock ms ON ms.product_id = p.id AND ms.machine_id = ?
         WHERE p.id = ?",
    )?;

    let mut rows = stmt.query_map(params![machine_id, product_id], |row| {
        Ok(Product {
            id:              row.get(0)?,
            name:            row.get(1)?,
            price:           row.get(2)?,
            g_per_dispense: row.get(3)?,
            stock_g:        row.get(4)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn record_sale(
    conn: &Connection,
    product_id: i64,
    machine_id: i64,
    status: &str,
    payment_method: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO sales (product_id, machine_id, status, payment_method)
         VALUES (?, ?, ?, ?)",
        params![product_id, machine_id, status, payment_method],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn decrement_stock(conn: &Connection, product_id: i64, machine_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE machine_stock
         SET quantity_g = quantity_g - (SELECT g_per_dispense FROM products WHERE id = ?)
         WHERE product_id = ? AND machine_id = ? AND quantity_g > 0",
        params![product_id, product_id, machine_id],
    )?;
    Ok(())
}

pub fn log_event(
    conn: &Connection,
    machine_id: i64,
    event: &str,
    details: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO logs (machine_id, event, details) VALUES (?, ?, ?)",
        params![machine_id, event, details],
    )?;
    write_log_file(machine_id, event, details);
    Ok(())
}

// --- Inventory ---

pub fn get_inventory(conn: &Connection, machine_id: i64) -> Result<Vec<InventoryItem>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.price,
                COALESCE(w.quantity_g,  0.0) AS warehouse_g,
                COALESCE(ms.quantity_g, 0.0) AS machine_g
         FROM products p
         LEFT JOIN warehouse_stock w  ON w.product_id  = p.id
         LEFT JOIN machine_stock   ms ON ms.product_id = p.id AND ms.machine_id = ?
         ORDER BY p.id",
    )?;

    let items = stmt
        .query_map(params![machine_id], |row| {
            Ok(InventoryItem {
                id:           row.get(0)?,
                name:         row.get(1)?,
                price:        row.get(2)?,
                warehouse_g: row.get(3)?,
                machine_g:   row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    Ok(items)
}

/// Register a purchase and add grams to warehouse stock.
pub fn add_purchase(
    conn: &mut Connection,
    product_id: i64,
    quantity_g: f64,
    cost: Option<f64>,
    notes: &str,
) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO purchases (product_id, quantity_g, cost, notes) VALUES (?, ?, ?, ?)",
        params![product_id, quantity_g, cost, notes],
    )?;
    tx.execute(
        "UPDATE warehouse_stock SET quantity_g = quantity_g + ? WHERE product_id = ?",
        params![quantity_g, product_id],
    )?;
    tx.commit()
}

/// Transfer grams from warehouse to a machine.
/// Returns Err if warehouse stock is insufficient.
pub fn transfer_stock(
    conn: &mut Connection,
    product_id: i64,
    machine_id: i64,
    quantity_g: f64,
    notes: &str,
) -> Result<(), String> {
    let available: f64 = conn
        .query_row(
            "SELECT quantity_g FROM warehouse_stock WHERE product_id = ?",
            params![product_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if available < quantity_g {
        return Err(format!(
            "Insufficient warehouse stock: {:.1}g available, {:.1}g requested",
            available, quantity_g
        ));
    }

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO stock_transfers (product_id, machine_id, quantity_g, notes) VALUES (?, ?, ?, ?)",
        params![product_id, machine_id, quantity_g, notes],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE warehouse_stock SET quantity_g = quantity_g - ? WHERE product_id = ?",
        params![quantity_g, product_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE machine_stock SET quantity_g = quantity_g + ? WHERE product_id = ? AND machine_id = ?",
        params![quantity_g, product_id, machine_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

fn write_log_file(machine_id: i64, event: &str, details: &str) {
    use std::io::Write;
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{timestamp}] machine={machine_id} {event} | {details}\n");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/system.log")
    {
        let _ = f.write_all(line.as_bytes());
    }
}
