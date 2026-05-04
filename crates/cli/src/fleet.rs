//! Fleet registry + supervisor (slice 015). Pure orchestration; no domain
//! traits — just SQLite-backed bookkeeping and OS process control.

use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS fleet (
    name           TEXT PRIMARY KEY,
    config_path    TEXT NOT NULL,
    port           INTEGER NOT NULL,
    pid            INTEGER,
    status         TEXT NOT NULL,
    started_at     INTEGER,
    last_ingest_at INTEGER
);
"#;

const PORT_BASE: u16 = 6700;
const PORT_RANGE: u16 = 100;

#[derive(Debug, Clone)]
pub struct Row {
    pub name: String,
    #[allow(dead_code)]
    pub config_path: String,
    pub port: u16,
    pub pid: Option<i32>,
    pub status: String,
    pub started_at: Option<i64>,
}

pub struct Registry {
    conn: Mutex<Connection>,
}

impl Registry {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("registry parent: {e}"))?;
        }
        let conn = Connection::open(path).map_err(|e| format!("registry open: {e}"))?;
        conn.execute_batch(SCHEMA).map_err(|e| format!("registry schema: {e}"))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn get(&self, name: &str) -> Result<Option<Row>, String> {
        let g = self.conn.lock().expect("poisoned");
        g.query_row(
            "SELECT name, config_path, port, pid, status, started_at FROM fleet WHERE name = ?1",
            params![name],
            row_from,
        ).optional().map_err(|e| e.to_string())
    }

    pub fn all(&self) -> Result<Vec<Row>, String> {
        let g = self.conn.lock().expect("poisoned");
        let mut stmt = g
            .prepare("SELECT name, config_path, port, pid, status, started_at FROM fleet ORDER BY name")
            .map_err(|e| e.to_string())?;
        let iter = stmt.query_map([], row_from).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in iter { out.push(r.map_err(|e| e.to_string())?); }
        Ok(out)
    }

    pub fn used_ports(&self) -> Result<Vec<u16>, String> {
        let g = self.conn.lock().expect("poisoned");
        let mut stmt = g.prepare("SELECT port FROM fleet").map_err(|e| e.to_string())?;
        let iter = stmt.query_map([], |r| Ok(r.get::<_, i64>(0)? as u16)).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in iter { out.push(r.map_err(|e| e.to_string())?); }
        Ok(out)
    }

    pub fn upsert(
        &self, name: &str, config_path: &str, port: u16, pid: Option<i32>,
        status: &str, started_at: Option<i64>,
    ) -> Result<(), String> {
        let g = self.conn.lock().expect("poisoned");
        g.execute(
            "INSERT INTO fleet(name, config_path, port, pid, status, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(name) DO UPDATE SET
                config_path = excluded.config_path,
                port = excluded.port,
                pid = excluded.pid,
                status = excluded.status,
                started_at = excluded.started_at",
            params![name, config_path, port as i64, pid, status, started_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn mark_stopped(&self, name: &str) -> Result<(), String> {
        let g = self.conn.lock().expect("poisoned");
        g.execute(
            "UPDATE fleet SET status = 'stopped', pid = NULL, started_at = NULL WHERE name = ?1",
            params![name],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn row_from(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        name: r.get(0)?,
        config_path: r.get(1)?,
        port: r.get::<_, i64>(2)? as u16,
        pid: r.get::<_, Option<i64>>(3)?.map(|v| v as i32),
        status: r.get(4)?,
        started_at: r.get(5)?,
    })
}

pub fn pid_alive(pid: i32) -> bool {
    // signal 0 doesn't actually send anything — it just probes the existence
    // of the target process. Returns 0 on alive, -1 on missing.
    unsafe { libc::kill(pid, 0) == 0 }
}

fn now_unix() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn child_binary() -> PathBuf {
    if let Ok(p) = std::env::var("LIBRARIAN_COLLECTION_BIN") {
        return PathBuf::from(p);
    }
    PathBuf::from("librarian-collection")
}

fn pick_port(reg: &Registry, prefer: Option<u16>) -> Result<u16, String> {
    let used: std::collections::HashSet<u16> = reg.used_ports()?.into_iter().collect();
    if let Some(p) = prefer {
        if !used.contains(&p) { return Ok(p); }
    }
    for p in PORT_BASE..PORT_BASE + PORT_RANGE {
        if !used.contains(&p) { return Ok(p); }
    }
    Err("no free ports in range".into())
}

pub fn start(reg: &Registry, name: &str, config_path: &Path) -> Result<String, String> {
    if let Some(row) = reg.get(name)? {
        if row.status == "running" {
            if let Some(pid) = row.pid {
                if pid_alive(pid) {
                    return Ok(format!("already running\tname={name}\tpid={pid}\tport={}", row.port));
                }
            }
        }
    }
    let port = pick_port(reg, reg.get(name)?.map(|r| r.port))?;
    let bin = child_binary();
    let child = Command::new(&bin)
        .arg("--config").arg(config_path)
        .arg("--port").arg(port.to_string())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn {}: {}", bin.display(), e))?;
    let pid = child.id() as i32;
    reg.upsert(name, &config_path.display().to_string(), port, Some(pid), "running", Some(now_unix()))?;
    // Detach: drop the Child without waiting. PID stays valid; supervisor
    // observes liveness via signal 0.
    std::mem::forget(child);
    Ok(format!("started\tname={name}\tpid={pid}\tport={port}"))
}

pub fn stop(reg: &Registry, name: &str) -> Result<String, String> {
    let row = match reg.get(name)? {
        None => return Ok(format!("not registered\tname={name}")),
        Some(r) => r,
    };
    if row.status == "stopped" {
        return Ok(format!("already stopped\tname={name}"));
    }
    if let Some(pid) = row.pid {
        unsafe { libc::kill(pid, libc::SIGTERM); }
        // Wait briefly for clean exit.
        for _ in 0..100 {
            if !pid_alive(pid) { break; }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if pid_alive(pid) {
            unsafe { libc::kill(pid, libc::SIGKILL); }
        }
    }
    reg.mark_stopped(name)?;
    Ok(format!("stopped\tname={name}"))
}

pub fn restart(reg: &Registry, name: &str, config_path: &Path) -> Result<String, String> {
    let _ = stop(reg, name)?;
    start(reg, name, config_path)
}

/// Reconcile registry against reality: any "running" row whose PID is gone
/// gets marked stopped. Then return the rows for printing.
pub fn fleet_status(reg: &Registry) -> Result<Vec<(Row, u64)>, String> {
    let mut rows = reg.all()?;
    let now = now_unix();
    for r in &mut rows {
        if r.status == "running" {
            let alive = r.pid.map(pid_alive).unwrap_or(false);
            if !alive {
                reg.mark_stopped(&r.name)?;
                r.status = "stopped".into();
                r.pid = None;
                r.started_at = None;
            }
        }
    }
    Ok(rows.into_iter().map(|r| {
        let uptime = match r.started_at {
            Some(s) if r.status == "running" => (now - s).max(0) as u64,
            _ => 0,
        };
        (r, uptime)
    }).collect())
}

pub fn registry_path() -> PathBuf {
    if let Ok(p) = std::env::var("LIBRARIAN_FLEET_DB") {
        return PathBuf::from(p);
    }
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/var/lib/librarian".into()))
        .join(".librarian/fleet.sqlite")
}
