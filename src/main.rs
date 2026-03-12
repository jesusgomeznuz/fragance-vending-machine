mod api;
mod database;
mod hardware;
mod metrics;
mod payment;
mod sync;

use actix_files as files;
use actix_web::{middleware, web, App, HttpServer};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use api::routes::AppState;
use database::db::init_db;
use hardware::uart::start_uart;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let simulation_mode = std::env::var("MODE")
        .map(|v| v.to_uppercase() == "SIMULATION")
        .unwrap_or(true);

    let machine_id: i64 = std::env::var("MACHINE_ID")
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .expect("MACHINE_ID must be a number");

    log::info!(
        "Starting Vending Machine | machine_id={machine_id} mode={}",
        if simulation_mode { "SIMULATION" } else { "PRODUCTION" }
    );

    std::fs::create_dir_all("logs").ok();

    let conn = Connection::open("vending_machine.db").expect("Failed to open database");
    init_db(&conn).expect("Failed to initialize database");

    let db = Arc::new(Mutex::new(conn));

    // --- Background sync to central server ---
    let sync_url = std::env::var("SYNC_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:8081".to_string());
    log::info!("Sync target: {sync_url} (every 30s)");
    tokio::spawn(sync::start_sync_loop(db.clone(), machine_id, sync_url));

    // --- Background metrics collector ---
    tokio::spawn(metrics::start_metrics_loop(db.clone(), machine_id));

    // --- UART (Pi ↔ ESP32) ---
    let uart = std::env::var("UART_PORT").ok().map(|port| {
        log::info!("UART enabled on {port}");
        std::sync::Arc::new(start_uart(&port))
    });
    if uart.is_none() {
        log::info!("UART disabled (set UART_PORT=/dev/ttyS0 to enable)");
    }

    log::info!("Server listening on http://0.0.0.0:8080");

    HttpServer::new(move || {
        let state = web::Data::new(AppState {
            db: db.clone(),
            simulation_mode,
            machine_id,
            uart: uart.clone(),
        });

        App::new()
            .app_data(state)
            .app_data(web::JsonConfig::default().error_handler(|err, _req| {
                let response = actix_web::HttpResponse::BadRequest()
                    .json(serde_json::json!({ "error": err.to_string() }));
                actix_web::error::InternalError::from_response(err, response).into()
            }))
            .route("/status",   web::get().to(api::routes::get_status))
            .route("/products", web::get().to(api::routes::get_products))
            .route("/pay",      web::post().to(api::routes::post_pay))
            .route("/dispense", web::post().to(api::routes::post_dispense))
            // Operator endpoints
            .route("/inventory",          web::get().to(api::inventory::get_inventory))
            .route("/inventory/purchase", web::post().to(api::inventory::post_purchase))
            .route("/inventory/transfer", web::post().to(api::inventory::post_transfer))
            .route("/metrics",            web::get().to(api::metrics_route::get_metrics))
            .service(
                web::scope("")
                    .wrap(middleware::DefaultHeaders::new()
                        .add(("Cache-Control", "no-store")))
                    .service(files::Files::new("/", "frontend").index_file("index.html"))
            )
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
