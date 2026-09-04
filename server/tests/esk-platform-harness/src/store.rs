use anyhow::Result;
use rusqlite::Connection;
use std::{path::PathBuf, time::Duration};

#[derive(Clone)]
pub(crate) struct Store {
    pub(crate) path: PathBuf,
}

impl Store {
    pub(crate) fn conn(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)?;
        conn.busy_timeout(Duration::from_secs(15))?;
        conn.pragma_update(None, "foreign_keys", true)?;
        Ok(conn)
    }
}

#[path = "common.rs"]
pub(crate) mod common;
