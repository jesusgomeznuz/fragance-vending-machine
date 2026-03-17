use actix_web::{web, HttpResponse, Responder};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::database::db;
use crate::hardware::dispenser::Dispenser;
use crate::hardware::uart::UartHandle;
use crate::payment::mercadopago::MercadoPagoClient;
use crate::payment::payment_simulator::PaymentSimulator;

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub simulation_mode: bool,
    pub machine_id: i64,
    pub uart: Option<Arc<UartHandle>>,
    pub mp: Option<Arc<MercadoPagoClient>>,
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

#[derive(Deserialize)]
pub struct PaymentStatusQuery {
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
    pending: bool,
    message: String,
    sale_id: Option<i64>,
    order_id: Option<String>,
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
    let machine_id = data.machine_id;

    // --- Leer producto (con lock) ---
    let product = {
        let conn = data.db.lock().unwrap();
        match db::get_product(&conn, body.product_id, machine_id) {
            Ok(Some(p)) => p,
            Ok(None) => {
                return HttpResponse::NotFound().json(PayResponse {
                    success: false,
                    pending: false,
                    message: format!("Product {} not found", body.product_id),
                    sale_id: None,
                    order_id: None,
                    product_name: None,
                    amount: None,
                });
            }
            Err(e) => {
                log::error!("DB error: {e}");
                return HttpResponse::InternalServerError().json(PayResponse {
                    success: false,
                    pending: false,
                    message: "Database error".to_string(),
                    sale_id: None,
                    order_id: None,
                    product_name: None,
                    amount: None,
                });
            }
        }
    }; // lock liberado aquí

    if product.stock_g <= 0.0 {
        return HttpResponse::BadRequest().json(PayResponse {
            success: false,
            pending: false,
            message: format!("{} is out of stock", product.name),
            sale_id: None,
            order_id: None,
            product_name: Some(product.name),
            amount: Some(product.price),
        });
    }

    log::info!(
        "Payment attempt | machine={machine_id} product='{}' price=${:.2}",
        product.name, product.price
    );
    {
        let conn = data.db.lock().unwrap();
        db::log_event(
            &conn,
            machine_id,
            "PAYMENT_ATTEMPT",
            &format!("product_id={} amount={:.2}", product.id, product.price),
        )
        .ok();
    }

    // --- Modo producción con Mercado Pago ---
    if !data.simulation_mode {
        if let Some(mp) = &data.mp {
            let external_ref = format!("m{machine_id}-p{}-{}", product.id, uuid::Uuid::new_v4());
            match mp
                .create_order(product.price, &product.name, &external_ref)
                .await
            {
                Ok(order_id) => {
                    log::info!(
                        "MP order created | machine={machine_id} product='{}' order_id={order_id}",
                        product.name
                    );
                    let conn = data.db.lock().unwrap();
                    db::log_event(
                        &conn,
                        machine_id,
                        "MP_ORDER_CREATED",
                        &format!("order_id={order_id} product_id={}", product.id),
                    )
                    .ok();
                    return HttpResponse::Ok().json(PayResponse {
                        success: false,
                        pending: true,
                        message: "Paga en la terminal".to_string(),
                        sale_id: None,
                        order_id: Some(order_id),
                        product_name: Some(product.name),
                        amount: Some(product.price),
                    });
                }
                Err(e) => {
                    log::error!("MP create_order failed: {e}");
                    let conn = data.db.lock().unwrap();
                    db::log_event(
                        &conn,
                        machine_id,
                        "MP_ORDER_ERROR",
                        &format!("product_id={} error={e}", product.id),
                    )
                    .ok();
                    return HttpResponse::InternalServerError().json(PayResponse {
                        success: false,
                        pending: false,
                        message: "Error al conectar con la terminal de pago".to_string(),
                        sale_id: None,
                        order_id: None,
                        product_name: Some(product.name),
                        amount: Some(product.price),
                    });
                }
            }
        } else {
            // PRODUCTION pero sin MP configurado
            return HttpResponse::ServiceUnavailable().json(PayResponse {
                success: false,
                pending: false,
                message: "Terminal de pago no configurada (falta MP_ACCESS_TOKEN / MP_TERMINAL_ID)"
                    .to_string(),
                sale_id: None,
                order_id: None,
                product_name: Some(product.name),
                amount: Some(product.price),
            });
        }
    }

    // --- Modo simulación ---
    let result = PaymentSimulator::new(data.simulation_mode).process(product.price);

    if result.success {
        let conn = data.db.lock().unwrap();
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
                    pending: false,
                    message: result.message,
                    sale_id: Some(sale_id),
                    order_id: None,
                    product_name: Some(product.name),
                    amount: Some(product.price),
                })
            }
            Err(e) => {
                log::error!("Failed to record sale: {e}");
                HttpResponse::InternalServerError().json(PayResponse {
                    success: false,
                    pending: false,
                    message: "Failed to record sale".to_string(),
                    sale_id: None,
                    order_id: None,
                    product_name: None,
                    amount: None,
                })
            }
        }
    } else {
        let conn = data.db.lock().unwrap();
        db::log_event(
            &conn,
            machine_id,
            "PAYMENT_FAILURE",
            &format!("product_id={}", product.id),
        )
        .ok();
        HttpResponse::Ok().json(PayResponse {
            success: false,
            pending: false,
            message: result.message,
            sale_id: None,
            order_id: None,
            product_name: Some(product.name),
            amount: Some(product.price),
        })
    }
}

/// GET /payment/{order_id}?product_id={id}
/// Consulta el estado de una orden MP y, si está procesada, registra la venta.
pub async fn get_payment_status(
    data: web::Data<AppState>,
    order_id: web::Path<String>,
    query: web::Query<PaymentStatusQuery>,
) -> impl Responder {
    let mp = match &data.mp {
        Some(mp) => mp,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "MP not configured" }));
        }
    };

    let status = match mp.get_order_status(&order_id).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("MP get_order_status failed: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e }));
        }
    };

    if status == "processed" {
        let machine_id = data.machine_id;
        let conn = data.db.lock().unwrap();

        let sale_id = db::record_sale(
            &conn,
            query.product_id,
            machine_id,
            "SUCCESS",
            "MERCADO_PAGO",
        )
        .ok();

        if let Some(sid) = sale_id {
            db::decrement_stock(&conn, query.product_id, machine_id).ok();
            db::log_event(
                &conn,
                machine_id,
                "PAYMENT_SUCCESS",
                &format!(
                    "sale_id={sid} order_id={order_id} product_id={} method=MERCADO_PAGO",
                    query.product_id
                ),
            )
            .ok();
            log::info!("MP payment confirmed | order_id={order_id} sale_id={sid}");
        }

        return HttpResponse::Ok().json(serde_json::json!({
            "status": status,
            "sale_id": sale_id,
        }));
    }

    HttpResponse::Ok().json(serde_json::json!({ "status": status }))
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
