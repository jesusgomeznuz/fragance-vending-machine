use actix_web::{web, HttpResponse, Responder};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::database::db;
use crate::hardware::dispenser::Dispenser;
use crate::hardware::uart::UartHandle;
use crate::payment::payment_simulator::PaymentSimulator;

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub simulation_mode: bool,
    pub machine_id: i64,
    pub uart: Option<Arc<UartHandle>>,
}

// --- Request / Response types ---

#[derive(Deserialize)]
pub struct PayRequest {
    pub product_id: i64,
}

#[derive(Deserialize)]
pub struct DispenseRequest {
    pub product_id: i64,
}

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    mode: String,
    machine_id: i64,
}

#[derive(Serialize)]
struct PayResponse {
    success: bool,
    message: String,
    sale_id: Option<i64>,
    product_name: Option<String>,
    amount: Option<f64>,
}

#[derive(Serialize)]
struct DispenseResponse {
    success: bool,
    message: String,
}

// --- Handlers ---

pub async fn get_status(data: web::Data<AppState>) -> impl Responder {
    let mode = if data.simulation_mode { "SIMULATION" } else { "PRODUCTION" };
    HttpResponse::Ok().json(StatusResponse {
        status: "ok".to_string(),
        mode: mode.to_string(),
        machine_id: data.machine_id,
    })
}

pub async fn get_products(data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    match db::get_products(&conn, data.machine_id) {
        Ok(products) => HttpResponse::Ok().json(products),
        Err(e) => {
            log::error!("Failed to fetch products: {e}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Failed to retrieve products" }))
        }
    }
}

pub async fn post_pay(
    data: web::Data<AppState>,
    body: web::Json<PayRequest>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let machine_id = data.machine_id;

    let product = match db::get_product(&conn, body.product_id, machine_id) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return HttpResponse::NotFound().json(PayResponse {
                success: false,
                message: format!("Product {} not found", body.product_id),
                sale_id: None,
                product_name: None,
                amount: None,
            });
        }
        Err(e) => {
            log::error!("DB error: {e}");
            return HttpResponse::InternalServerError().json(PayResponse {
                success: false,
                message: "Database error".to_string(),
                sale_id: None,
                product_name: None,
                amount: None,
            });
        }
    };

    if product.stock_g <= 0.0 {
        return HttpResponse::BadRequest().json(PayResponse {
            success: false,
            message: format!("{} is out of stock", product.name),
            sale_id: None,
            product_name: Some(product.name),
            amount: Some(product.price),
        });
    }

    log::info!(
        "Payment attempt | machine={machine_id} product='{}' price=${:.2}",
        product.name, product.price
    );
    db::log_event(
        &conn,
        machine_id,
        "PAYMENT_ATTEMPT",
        &format!("product_id={} amount={:.2}", product.id, product.price),
    )
    .ok();

    let result = PaymentSimulator::new(data.simulation_mode).process(product.price);

    if result.success {
        match db::record_sale(&conn, product.id, machine_id, "SUCCESS", &result.method) {
            Ok(sale_id) => {
                db::decrement_stock(&conn, product.id, machine_id).ok();
                log::info!(
                    "Payment success | machine={machine_id} product='{}' sale_id={sale_id}",
                    product.name
                );
                db::log_event(
                    &conn,
                    machine_id,
                    "PAYMENT_SUCCESS",
                    &format!(
                        "sale_id={sale_id} product='{}' amount={:.2}",
                        product.name, product.price
                    ),
                )
                .ok();

                HttpResponse::Ok().json(PayResponse {
                    success: true,
                    message: result.message,
                    sale_id: Some(sale_id),
                    product_name: Some(product.name),
                    amount: Some(product.price),
                })
            }
            Err(e) => {
                log::error!("Failed to record sale: {e}");
                HttpResponse::InternalServerError().json(PayResponse {
                    success: false,
                    message: "Failed to record sale".to_string(),
                    sale_id: None,
                    product_name: None,
                    amount: None,
                })
            }
        }
    } else {
        db::log_event(
            &conn,
            machine_id,
            "PAYMENT_FAILURE",
            &format!("product_id={}", product.id),
        )
        .ok();
        HttpResponse::Ok().json(PayResponse {
            success: false,
            message: result.message,
            sale_id: None,
            product_name: Some(product.name),
            amount: Some(product.price),
        })
    }
}

pub async fn post_dispense(
    data: web::Data<AppState>,
    body: web::Json<DispenseRequest>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let machine_id = data.machine_id;

    log::info!("Dispense request | machine={machine_id} product_id={}", body.product_id);
    db::log_event(
        &conn,
        machine_id,
        "DISPENSE_REQUEST",
        &format!("product_id={}", body.product_id),
    )
    .ok();

    let ok = if let Some(uart) = &data.uart {
        if uart.is_online() {
            uart.send_dispense()
        } else {
            log::warn!("UART: ESP32 offline, falling back to simulation");
            Dispenser::new(data.simulation_mode).dispense(body.product_id)
        }
    } else {
        Dispenser::new(data.simulation_mode).dispense(body.product_id)
    };

    if ok {
        db::log_event(
            &conn,
            machine_id,
            "DISPENSE_SUCCESS",
            &format!("product_id={}", body.product_id),
        )
        .ok();
        HttpResponse::Ok().json(DispenseResponse {
            success: true,
            message: format!("Product {} dispensed successfully", body.product_id),
        })
    } else {
        db::log_event(
            &conn,
            machine_id,
            "DISPENSE_FAILURE",
            &format!("product_id={}", body.product_id),
        )
        .ok();
        HttpResponse::InternalServerError().json(DispenseResponse {
            success: false,
            message: "Dispense failed".to_string(),
        })
    }
}
