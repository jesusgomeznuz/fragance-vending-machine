use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::api::routes::AppState;
use crate::database::db;

#[derive(Deserialize)]
pub struct PurchaseRequest {
    pub product_id:  i64,
    pub quantity_g: f64,
    pub cost:        Option<f64>,
    pub notes:       Option<String>,
}

#[derive(Deserialize)]
pub struct TransferRequest {
    pub product_id:  i64,
    pub quantity_g: f64,
    pub notes:       Option<String>,
}

pub async fn get_inventory(data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    match db::get_inventory(&conn, data.machine_id) {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => {
            log::error!("Failed to fetch inventory: {e}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Failed to retrieve inventory" }))
        }
    }
}

pub async fn post_purchase(
    data: web::Data<AppState>,
    body: web::Json<PurchaseRequest>,
) -> impl Responder {
    if body.quantity_g <= 0.0 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "quantity_g must be greater than 0" }));
    }

    let mut conn = data.db.lock().unwrap();
    let notes = body.notes.as_deref().unwrap_or("");

    match db::add_purchase(&mut conn, body.product_id, body.quantity_g, body.cost, notes) {
        Ok(_) => {
            log::info!(
                "Purchase recorded | product_id={} qty={:.1}g",
                body.product_id, body.quantity_g
            );
            db::log_event(
                &conn,
                data.machine_id,
                "PURCHASE",
                &format!("product_id={} qty={:.1}g", body.product_id, body.quantity_g),
            )
            .ok();
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": format!("{:.1}g added to warehouse", body.quantity_g)
            }))
        }
        Err(e) => {
            log::error!("Purchase failed: {e}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

pub async fn post_transfer(
    data: web::Data<AppState>,
    body: web::Json<TransferRequest>,
) -> impl Responder {
    if body.quantity_g <= 0.0 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "quantity_g must be greater than 0" }));
    }

    let mut conn = data.db.lock().unwrap();
    let notes = body.notes.as_deref().unwrap_or("");

    match db::transfer_stock(&mut conn, body.product_id, data.machine_id, body.quantity_g, notes) {
        Ok(_) => {
            log::info!(
                "Stock transferred | product_id={} qty={:.1}g -> machine={}",
                body.product_id, body.quantity_g, data.machine_id
            );
            db::log_event(
                &conn,
                data.machine_id,
                "STOCK_TRANSFER",
                &format!(
                    "product_id={} qty={:.1}g warehouse->machine={}",
                    body.product_id, body.quantity_g, data.machine_id
                ),
            )
            .ok();
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": format!("{:.1}g transferred to machine {}", body.quantity_g, data.machine_id)
            }))
        }
        Err(e) => {
            log::warn!("Transfer failed: {e}");
            HttpResponse::BadRequest().json(serde_json::json!({ "error": e }))
        }
    }
}
