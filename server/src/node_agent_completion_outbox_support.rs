use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Result};
use homecli_proto::{CliCompletionEnvelope, CliCompletionProducerIdentity};
use rusqlite::Connection;

use super::LOCAL_OFFLINE_ORIGIN;

pub(super) fn validate_completion(completion: &CliCompletionEnvelope) -> Result<()> {
    required_id(&completion.event_id, "event_id")?;
    required_id(&completion.req_id, "req_id")?;
    required_id(&completion.cli, "cli")?;
    let origin = required_id(&completion.origin, "origin")?;
    let producer = completion
        .producer_identity
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("CLI completion 缺少冻结的生产者身份"))?;
    validate_producer_identity(producer)?;
    if completion.created_at_ms == 0 {
        bail!("CLI completion created_at_ms 必须大于 0");
    }
    if completion.prompt.is_some() && origin != LOCAL_OFFLINE_ORIGIN {
        bail!("只有 local_offline completion 可以持久化 prompt");
    }
    Ok(())
}

pub(super) fn validate_producer_identity(producer: &CliCompletionProducerIdentity) -> Result<()> {
    for (field, value) in [
        ("producer.owner_user_id", producer.owner_user_id.as_str()),
        ("producer.agent_id", producer.agent_id.as_str()),
        ("producer.install_id", producer.install_id.as_str()),
    ] {
        let value = required_id(value, field)?;
        if value.chars().count() > 200 || value.chars().any(char::is_control) {
            bail!("CLI completion {field} 无效");
        }
    }
    Ok(())
}

pub(super) fn ensure_identity_column(conn: &Connection, column: &str) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(cli_completion_outbox)")?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if !columns.iter().any(|value| value == column) {
        conn.execute_batch(&format!(
            "ALTER TABLE cli_completion_outbox ADD COLUMN {column} TEXT"
        ))?;
    }
    Ok(())
}

pub(super) fn required_id<'a>(value: &'a str, field: &str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        bail!("CLI completion {field} 不能为空");
    }
    Ok(value)
}

pub(super) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

pub(super) fn sqlite_ms(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

pub(super) fn nonnegative_u64(value: i64) -> u64 {
    value.max(0) as u64
}

pub(super) fn truncate_optional(value: Option<&str>, max_chars: usize) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(max_chars).collect())
}
