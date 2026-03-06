use actix_web::{web, HttpResponse, Responder};

use crate::db::{self, SyncPayload};
use crate::AppState;

pub async fn get_health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
}

pub async fn post_sync(
    state: web::Data<AppState>,
    body: web::Json<SyncPayload>,
) -> impl Responder {
    let conn = state.db.lock().unwrap();
    match db::store_sync(&conn, &body) {
        Ok(n) => {
            log::info!(
                "Sync received | machine_id={} sales={} events={} new_records={}",
                body.machine_id,
                body.sales.len(),
                body.events.len(),
                n,
            );
            HttpResponse::Ok().json(serde_json::json!({ "synced": n }))
        }
        Err(e) => {
            log::error!("Sync store failed: {e}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "sync failed" }))
        }
    }
}
