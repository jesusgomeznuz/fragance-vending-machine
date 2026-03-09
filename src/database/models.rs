use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    pub id:              i64,
    pub name:            String,
    pub price:           f64,
    pub g_per_dispense: f64,
    pub stock_g:        f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id:           i64,
    pub name:         String,
    pub price:        f64,
    pub warehouse_g: f64,
    pub machine_g:   f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sale {
    pub id:             i64,
    pub product_id:     i64,
    pub timestamp:      String,
    pub status:         String,
    pub payment_method: String,
}
