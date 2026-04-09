use anyhow::{Context, bail};
use rusqlite::Connection;
use std::path::Path;

const SCHEMA: &str = include_str!("../../db/migrations/0001_memory.sql");

const LEGACY_TABLES: &[&str] = &[
    "memory_steps",
    "memory_verifications",
    "memory_transitions",
    "memory_recoveries",
];

pub fn open_memory_connection(path: &Path) -> anyhow::Result<Connection> {
    let connection = Connection::open(path)
        .with_context(|| format!("failed to connect to sqlite database at {:?}", path))?;
    reject_legacy_schema(&connection, path)?;
    connection
        .execute_batch(SCHEMA)
        .with_context(|| "failed to initialize memory schema")?;
    Ok(connection)
}

fn reject_legacy_schema(conn: &Connection, path: &Path) -> anyhow::Result<()> {
    for table in LEGACY_TABLES {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [table],
            |row| row.get(0),
        )?;
        if exists {
            bail!(
                "database at {} contains legacy table '{table}'. \
                 The memory schema has changed (see CHANGELOG.md BREAKING). \
                 Delete or rename the old database file and restart.",
                path.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
pub fn open_in_memory_connection() -> anyhow::Result<Connection> {
    let connection =
        Connection::open_in_memory().with_context(|| "failed to create in-memory sqlite")?;
    connection
        .execute_batch(SCHEMA)
        .with_context(|| "failed to initialize memory schema")?;
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{OptionalExtension, params};

    const TABLES: &[&str] = &[
        "session_state",
        "events",
        "transitions",
        "recoveries",
        "notes",
        "artifacts",
    ];

    #[test]
    fn creates_all_tables() {
        let conn = open_in_memory_connection().expect("open");
        for table in TABLES {
            let exists = conn
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .expect("query");
            assert_eq!(exists, Some(1), "missing table {table}");
        }
    }

    #[test]
    fn idempotent_on_double_init() {
        let conn = open_in_memory_connection().expect("open");
        conn.execute_batch(SCHEMA)
            .expect("second init should be idempotent");
    }

    #[test]
    fn rejects_legacy_schema() {
        let conn = Connection::open_in_memory().expect("open");
        conn.execute_batch("CREATE TABLE memory_steps (id INTEGER PRIMARY KEY)")
            .expect("create legacy table");
        let path = Path::new("test.db");
        let err = reject_legacy_schema(&conn, path).unwrap_err();
        assert!(
            err.to_string().contains("legacy table"),
            "expected legacy table error, got: {err}"
        );
    }
}
