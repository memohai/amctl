use anyhow::Context;
use rusqlite::{Connection, params};
use std::path::PathBuf;

#[derive(Debug)]
pub struct TraceStore {
    connection: Connection,
}

#[derive(Debug, Clone)]
pub struct TraceRecord {
    pub created_at: String,
    pub session: String,
    pub trace_id: String,
    pub command: String,
    pub status: String,
    pub output_json: String,
    pub duration_ms: u128,
}

impl TraceStore {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let connection = Connection::open(&path)
            .with_context(|| format!("failed to connect to sqlite database at {:?}", path))?;
        init_trace_table(&connection)?;
        Ok(Self { connection })
    }

    pub fn record(&self, record: &TraceRecord) -> anyhow::Result<()> {
        self.connection
            .execute(
                "INSERT INTO traces (
                    created_at, session, trace_id, command, status, output_json, duration_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    record.created_at,
                    record.session,
                    record.trace_id,
                    record.command,
                    record.status,
                    record.output_json,
                    record.duration_ms as i64
                ],
            )
            .with_context(|| "failed to insert trace record")?;
        Ok(())
    }
}

fn init_trace_table(connection: &Connection) -> anyhow::Result<()> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS traces (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT NOT NULL,
                session TEXT NOT NULL,
                trace_id TEXT NOT NULL,
                command TEXT NOT NULL,
                status TEXT NOT NULL,
                output_json TEXT NOT NULL,
                duration_ms INTEGER NOT NULL
            )",
            [],
        )
        .with_context(|| "failed to create traces table")?;
    Ok(())
}
