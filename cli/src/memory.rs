use anyhow::Context;
use rusqlite::Connection;
use std::path::PathBuf;

#[derive(Debug)]
pub struct SqliteConnection {
    path: PathBuf,
    connection: rusqlite::Connection,
}

impl SqliteConnection {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let connection = rusqlite::Connection::open(&path)
            .with_context(|| format!("failed to connect to sqlite database at {:?}", path))?;
        Self::connection_init(&connection)?;
        Ok(SqliteConnection { path, connection })
    }

    fn connection_init(connection: &Connection) -> anyhow::Result<()> {
        Self::init_trace_table(connection)?;
        Ok(())
    }

    fn init_trace_table(connection: &Connection) -> anyhow::Result<()> {
        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS traces (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    trace TEXT NOT NULL
                )",
                [],
            )
            .with_context(|| "failed to create traces table")?;
        Ok(())
    }

    pub fn trace(&self, message: &str) -> anyhow::Result<()> {
        self.connection
            .execute("INSERT INTO traces (trace) VALUES (?1)", &[message])
            .with_context(|| "failed to insert trace into database")?;
        Ok(())
    }
}

pub struct Memory {}
