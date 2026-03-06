-- Fragrance Vending Machine — SQLite Schema v2

-- Physical machines
CREATE TABLE IF NOT EXISTS machines (
    id       INTEGER PRIMARY KEY,
    name     TEXT NOT NULL,
    location TEXT
);

-- Product catalog (no stock here — stock lives in warehouse_stock / machine_stock)
CREATE TABLE IF NOT EXISTS products (
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    name  TEXT NOT NULL,
    price REAL NOT NULL
);

-- Global warehouse stock (what you own but hasn't been loaded into any machine)
CREATE TABLE IF NOT EXISTS warehouse_stock (
    product_id INTEGER NOT NULL PRIMARY KEY,
    quantity   INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (product_id) REFERENCES products(id)
);

-- Stock currently loaded inside each machine
CREATE TABLE IF NOT EXISTS machine_stock (
    machine_id INTEGER NOT NULL,
    product_id INTEGER NOT NULL,
    quantity   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (machine_id, product_id),
    FOREIGN KEY (machine_id) REFERENCES machines(id),
    FOREIGN KEY (product_id) REFERENCES products(id)
);

-- Record of merchandise purchases (replenishes warehouse_stock)
CREATE TABLE IF NOT EXISTS purchases (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER  NOT NULL,
    quantity   INTEGER  NOT NULL,
    cost       REAL,
    timestamp  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    notes      TEXT,
    FOREIGN KEY (product_id) REFERENCES products(id)
);

-- Transfers from warehouse → machine
CREATE TABLE IF NOT EXISTS stock_transfers (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER  NOT NULL,
    machine_id INTEGER  NOT NULL,
    quantity   INTEGER  NOT NULL,
    timestamp  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    notes      TEXT,
    FOREIGN KEY (product_id) REFERENCES products(id),
    FOREIGN KEY (machine_id) REFERENCES machines(id)
);

-- Sales (each transaction)
CREATE TABLE IF NOT EXISTS sales (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id     INTEGER  NOT NULL,
    machine_id     INTEGER  NOT NULL,
    timestamp      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status         TEXT     NOT NULL,   -- SUCCESS | FAILURE
    payment_method TEXT     NOT NULL,   -- SIMULATED | CARD | CASH | ...
    FOREIGN KEY (product_id) REFERENCES products(id),
    FOREIGN KEY (machine_id) REFERENCES machines(id)
);

-- System event log
CREATE TABLE IF NOT EXISTS logs (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    machine_id INTEGER  NOT NULL,
    timestamp  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    event      TEXT     NOT NULL,
    details    TEXT
);
