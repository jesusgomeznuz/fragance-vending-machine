use actix_web::{web, HttpResponse, Responder};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::api::routes::AppState;

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub period: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct MetricRow {
    pub timestamp:        String,
    pub cpu_temp_c:       Option<f64>,
    pub cpu_load_1m:      Option<f64>,
    pub cpu_usage_pct:    Option<f64>,
    pub mem_total_mb:     Option<f64>,
    pub mem_used_mb:      Option<f64>,
    pub mem_available_mb: Option<f64>,
    pub disk_total_gb:    Option<f64>,
    pub disk_used_gb:     Option<f64>,
    pub uptime_s:         Option<f64>,
}

#[derive(Serialize)]
pub struct OfflineGap {
    pub from:         String,
    pub to:           String,
    pub duration_min: f64,
}

#[derive(Serialize)]
pub struct MetricsResponse {
    pub period:        String,
    pub is_online:     bool,
    pub metrics:       Vec<MetricRow>,
    pub latest:        Option<MetricRow>,
    pub offline_gaps:  Vec<OfflineGap>,
}

pub async fn get_metrics(
    data:  web::Data<AppState>,
    query: web::Query<MetricsQuery>,
) -> impl Responder {
    let period = query.period.as_deref().unwrap_or("1h").to_string();
    let minutes: i64 = match period.as_str() {
        "6h"  => 360,
        "24h" => 1440,
        "7d"  => 10080,
        _     => 60,
    };

    let conn = data.db.lock().unwrap();

    // Fetch rows for the selected period
    let mut stmt = match conn.prepare(
        "SELECT timestamp, cpu_temp_c, cpu_load_1m, cpu_usage_pct,
                mem_total_mb, mem_used_mb, mem_available_mb,
                disk_total_gb, disk_used_gb, uptime_s
         FROM system_metrics
         WHERE machine_id = ?
           AND timestamp >= datetime('now', ? || ' minutes')
         ORDER BY timestamp ASC",
    ) {
        Ok(s)  => s,
        Err(e) => {
            log::error!("metrics query: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "db error"}));
        }
    };

    let neg = format!("-{minutes}");
    let all_rows: Vec<MetricRow> = stmt
        .query_map(params![data.machine_id, neg], |row| {
            Ok(MetricRow {
                timestamp:        row.get(0)?,
                cpu_temp_c:       row.get(1)?,
                cpu_load_1m:      row.get(2)?,
                cpu_usage_pct:    row.get(3)?,
                mem_total_mb:     row.get(4)?,
                mem_used_mb:      row.get(5)?,
                mem_available_mb: row.get(6)?,
                disk_total_gb:    row.get(7)?,
                disk_used_gb:     row.get(8)?,
                uptime_s:         row.get(9)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Downsample to ~200 points max for large periods
    let metrics = if all_rows.len() > 200 {
        let step = all_rows.len() / 200;
        all_rows.iter().step_by(step).cloned().collect()
    } else {
        all_rows.clone()
    };

    // Most recent reading ever (to show current status)
    let latest: Option<MetricRow> = conn
        .query_row(
            "SELECT timestamp, cpu_temp_c, cpu_load_1m, cpu_usage_pct,
                    mem_total_mb, mem_used_mb, mem_available_mb,
                    disk_total_gb, disk_used_gb, uptime_s
             FROM system_metrics
             WHERE machine_id = ?
             ORDER BY id DESC LIMIT 1",
            params![data.machine_id],
            |row| Ok(MetricRow {
                timestamp:        row.get(0)?,
                cpu_temp_c:       row.get(1)?,
                cpu_load_1m:      row.get(2)?,
                cpu_usage_pct:    row.get(3)?,
                mem_total_mb:     row.get(4)?,
                mem_used_mb:      row.get(5)?,
                mem_available_mb: row.get(6)?,
                disk_total_gb:    row.get(7)?,
                disk_used_gb:     row.get(8)?,
                uptime_s:         row.get(9)?,
            }),
        )
        .ok();

    // Online = last metric was recorded within the past 2 minutes
    let is_online = latest.as_ref().map_or(false, |r| {
        conn.query_row(
            "SELECT (strftime('%s','now') - strftime('%s',?)) < 120",
            params![r.timestamp],
            |row| row.get::<_, bool>(0),
        ).unwrap_or(false)
    });

    // Detect offline gaps (consecutive rows with gap > 2 min = 120 s)
    let mut offline_gaps: Vec<OfflineGap> = Vec::new();
    for pair in all_rows.windows(2) {
        let gap_s: Option<f64> = conn.query_row(
            "SELECT (strftime('%s',?) - strftime('%s',?))",
            params![pair[1].timestamp, pair[0].timestamp],
            |row| row.get(0),
        ).ok();
        if let Some(s) = gap_s {
            if s > 120.0 {
                offline_gaps.push(OfflineGap {
                    from:         pair[0].timestamp.clone(),
                    to:           pair[1].timestamp.clone(),
                    duration_min: (s / 60.0 * 10.0).round() / 10.0,
                });
            }
        }
    }

    HttpResponse::Ok().json(MetricsResponse {
        period,
        is_online,
        metrics,
        latest,
        offline_gaps,
    })
}
