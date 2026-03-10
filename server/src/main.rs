use actix_files as files;
use actix_web::{web, App, HttpServer};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

mod api;
mod db;

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse()
        .expect("PORT must be a number");

    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "central.db".to_string());
    let conn = Connection::open(&db_path).expect("Failed to open central database");
    db::init_db(&conn).expect("Failed to initialize central database");
    let db = Arc::new(Mutex::new(conn));

    log::info!("Central server listening on http://0.0.0.0:{port}  (db={db_path})");

    HttpServer::new(move || {
        let state = web::Data::new(AppState { db: db.clone() });

        App::new()
            .app_data(state)
            .app_data(web::JsonConfig::default().error_handler(|err, _req| {
                let response = actix_web::HttpResponse::BadRequest()
                    .json(serde_json::json!({ "error": err.to_string() }));
                actix_web::error::InternalError::from_response(err, response).into()
            }))
            .route("/health",  web::get().to(api::get_health))
            .route("/sync",    web::post().to(api::post_sync))
            .route("/metrics", web::get().to(api::get_metrics))
            .service(files::Files::new("/", "server/frontend").index_file("metrics.html"))
    })
    .bind(format!("0.0.0.0:{port}"))?
    .run()
    .await
}
