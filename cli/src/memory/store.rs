use rusqlite::Connection;

#[derive(Debug)]
pub struct MemoryStore {
    pub(super) connection: Connection,
}
