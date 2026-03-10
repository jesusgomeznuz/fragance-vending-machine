use std::sync::{Arc, Mutex};
use std::time::Duration;
use rusqlite::{params, Connection};
use tokio::time::interval;

struct CpuStat { total: u64, idle: u64 }

fn read_cpu_stat() -> Option<CpuStat> {
    let content = std::fs::read_to_string("/proc/stat").ok()?;
    let line = content.lines().next()?;
    let mut p = line.split_whitespace();
    p.next();
    let user:    u64 = p.next()?.parse().ok()?;
    let nice:    u64 = p.next()?.parse().ok()?;
    let system:  u64 = p.next()?.parse().ok()?;
    let idle:    u64 = p.next()?.parse().ok()?;
    let iowait:  u64 = p.next()?.parse().ok()?;
    let irq:     u64 = p.next()?.parse().ok()?;
    let softirq: u64 = p.next()?.parse().ok()?;
    Some(CpuStat {
        total: user + nice + system + idle + iowait + irq + softirq,
        idle:  idle + iowait,
    })
}

fn cpu_usage_pct(prev: &CpuStat, curr: &CpuStat) -> f64 {
    let total = curr.total.saturating_sub(prev.total) as f64;
    let idle  = curr.idle.saturating_sub(prev.idle) as f64;
    if total == 0.0 { return 0.0; }
    ((total - idle) / total * 1000.0).round() / 10.0
}

fn read_cpu_temp() -> Option<f64> {
    let raw: f64 = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
        .ok()?.trim().parse().ok()?;
    Some((raw / 100.0).round() / 10.0)
}

fn read_loadavg() -> Option<(f64, f64, f64)> {
    let s = std::fs::read_to_string("/proc/loadavg").ok()?;
    let mut p = s.split_whitespace();
    Some((p.next()?.parse().ok()?, p.next()?.parse().ok()?, p.next()?.parse().ok()?))
}

fn read_mem() -> Option<(f64, f64, f64)> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb: Option<f64> = None;
    let mut avail_kb: Option<f64> = None;
    for line in content.lines() {
        if line.starts_with("MemTotal:")     { total_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()); }
        if line.starts_with("MemAvailable:") { avail_kb  = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()); }
        if total_kb.is_some() && avail_kb.is_some() { break; }
    }
    let total = total_kb? / 1024.0;
    let avail = avail_kb? / 1024.0;
    let used  = total - avail;
    Some(((total * 10.0).round() / 10.0, (used * 10.0).round() / 10.0, (avail * 10.0).round() / 10.0))
}

fn read_disk() -> Option<(f64, f64)> {
    let out = std::process::Command::new("df").args(["-k", "/"]).output().ok()?;
    let stdout = String::from_utf8(out.stdout).ok()?;
    let line   = stdout.lines().nth(1)?;
    let mut p  = line.split_whitespace();
    p.next(); // filesystem
    let total_kb: f64 = p.next()?.parse().ok()?;
    let used_kb:  f64 = p.next()?.parse().ok()?;
    Some((
        (total_kb / 1_048_576.0 * 10.0).round() / 10.0,
        (used_kb  / 1_048_576.0 * 10.0).round() / 10.0,
    ))
}

fn read_uptime_s() -> Option<f64> {
    std::fs::read_to_string("/proc/uptime").ok()?
        .split_whitespace().next()?.parse().ok()
}

pub async fn start_metrics_loop(db: Arc<Mutex<Connection>>, machine_id: i64) {
    let mut tick     = interval(Duration::from_secs(30));
    let mut prev_cpu = read_cpu_stat();

    loop {
        tick.tick().await;

        let curr_cpu = read_cpu_stat();
        let usage    = prev_cpu.as_ref().zip(curr_cpu.as_ref()).map(|(p, c)| cpu_usage_pct(p, c));
        prev_cpu     = curr_cpu;

        let temp   = read_cpu_temp();
        let load   = read_loadavg();
        let mem    = read_mem();
        let disk   = read_disk();
        let uptime = read_uptime_s();

        let (load_1m, load_5m, load_15m) = match load {
            Some((a, b, c)) => (Some(a), Some(b), Some(c)),
            None            => (None, None, None),
        };
        let (mem_total, mem_used, mem_avail) = match mem {
            Some((a, b, c)) => (Some(a), Some(b), Some(c)),
            None            => (None, None, None),
        };
        let (disk_total, disk_used) = match disk {
            Some((a, b)) => (Some(a), Some(b)),
            None         => (None, None),
        };

        let conn = db.lock().unwrap();
        if let Err(e) = conn.execute(
            "INSERT INTO system_metrics (
                machine_id, cpu_temp_c, cpu_load_1m, cpu_load_5m, cpu_load_15m,
                cpu_usage_pct, mem_total_mb, mem_used_mb, mem_available_mb,
                disk_total_gb, disk_used_gb, uptime_s
             ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                machine_id, temp, load_1m, load_5m, load_15m,
                usage, mem_total, mem_used, mem_avail,
                disk_total, disk_used, uptime,
            ],
        ) {
            log::warn!("Failed to store system metrics: {e}");
        }
    }
}
