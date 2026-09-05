use anyhow::Result;
use rusqlite::Connection;

use crate::esk_asset::platform::{sellback::*, PlatformError};

use super::{ensure_session, history::scan_authenticated_history_on, new_id, now, policy_on};

mod cancel;
mod read;
mod records;
mod snapshot;
mod write;

pub(super) use snapshot::scan_delegated_on;

fn authenticate(conn: &Connection, user_id: &str, token: &str) -> Result<()> {
    ensure_session(conn, user_id, token, false).map_err(platform_error)
}

fn platform_error(error: anyhow::Error) -> anyhow::Error {
    match error.downcast_ref::<PlatformError>() {
        Some(PlatformError::Unauthorized) => SellbackError::Unauthorized.into(),
        Some(_) => SellbackError::Corrupt.into(),
        None => error,
    }
}

fn add(left: i64, right: i64) -> Result<i64> {
    left.checked_add(right)
        .ok_or_else(|| SellbackError::Corrupt.into())
}
