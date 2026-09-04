use anyhow::Result;

use super::model::{PlatformEntry, PlatformError};

#[derive(Debug, Clone)]
pub(crate) struct PlatformHistoryPage {
    pub snapshot_digest: String,
    pub total_base_units: i64,
    pub entry_count: i64,
    pub range_start: i64,
    pub range_end: i64,
    pub updated_at: Option<String>,
    pub entries: Vec<PlatformEntry>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

/// A position hint, never a user identity or authorization credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlatformHistoryCursor {
    pub snapshot_digest: String,
    pub after_entry_id: String,
}

impl PlatformHistoryCursor {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        if value.len() != 114 {
            return Err(PlatformError::InvalidInput.into());
        }
        let mut parts = value.split('.');
        let prefix = parts.next();
        let digest = parts.next().unwrap_or_default();
        let entry_id = parts.next().unwrap_or_default();
        if prefix != Some("ephp1")
            || !lower_hex(digest, 64)
            || !valid_history_entry_id(entry_id)
            || parts.next().is_some()
        {
            return Err(PlatformError::InvalidInput.into());
        }
        Ok(Self {
            snapshot_digest: digest.to_owned(),
            after_entry_id: entry_id.to_owned(),
        })
    }
}

pub(crate) fn valid_history_entry_id(value: &str) -> bool {
    value
        .strip_prefix("eskp_entry_")
        .is_some_and(|suffix| lower_hex(suffix, 32))
}

fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
