//! Privacy-minimized aggregation for Codex app-server project-memory measurements.

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::project_document_index::ProjectDocumentIndex;

const MAX_EVENT_BYTES: usize = 128 * 1024;
const MAX_BENCHMARK_KEY: usize = 80;
const ABANDONED_WINDOW_AGE_MS: u64 = 24 * 60 * 60 * 1_000;
const MAX_RETAINED_WINDOWS: usize = 2_000;

pub(crate) fn start_window(
    workspace: &Path,
    benchmark_key: &str,
    measurement_window: &str,
    session_id: &str,
) -> Result<Value> {
    let benchmark_key = normalized_key(benchmark_key)?;
    let measurement_window = normalized_window(measurement_window)?;
    if session_id.trim().is_empty() {
        bail!("runtime observation session_id 不能为空");
    }
    let index = ProjectDocumentIndex::open(workspace)?;
    initialize_schema(&index)?;
    let window_id = format!("obs_{}", uuid::Uuid::new_v4().simple());
    let started_at_ms = now_millis();
    index.conn.execute(
        "UPDATE native_context_observation_windows SET status='abandoned'
         WHERE status='active' AND started_at_ms<?1",
        params![to_i64(
            started_at_ms.saturating_sub(ABANDONED_WINDOW_AGE_MS)
        )],
    )?;
    index.conn.execute(
        "DELETE FROM native_context_observation_windows WHERE window_id IN (
           SELECT window_id FROM native_context_observation_windows WHERE status!='active'
           ORDER BY completed_at_ms DESC,started_at_ms DESC LIMIT -1 OFFSET ?1
         )",
        params![MAX_RETAINED_WINDOWS as i64],
    )?;
    index.conn.execute(
        "INSERT INTO native_context_observation_windows(
           window_id,benchmark_key,measurement_window,session_fingerprint,status,
           started_at_ms,completed_at_ms,event_count,hook_count,turn_count,item_count,
           native_file_read_count,input_tokens,cached_input_tokens,output_tokens,
           selected_memory_count,returned_metadata_bytes)
         VALUES(?1,?2,?3,?4,'active',?5,0,0,0,0,0,0,0,0,0,0,0)",
        params![
            window_id,
            benchmark_key,
            measurement_window,
            fingerprint(session_id),
            to_i64(started_at_ms),
        ],
    )?;
    Ok(json!({
        "schema":"elon.project_context_runtime_observation_window.v1",
        "window_id":window_id,
        "benchmark_key":benchmark_key,
        "measurement_window":measurement_window,
        "status":"active",
        "started_at_ms":started_at_ms,
        "raw_event_payloads_stored":false,
    }))
}

pub(crate) fn ingest_event(workspace: &Path, window_id: &str, event: Value) -> Result<Value> {
    if serde_json::to_vec(&event)?.len() > MAX_EVENT_BYTES {
        bail!("runtime observation event 超过 128 KiB 上限");
    }
    let method = event
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !accepted_method(method) {
        bail!("runtime observation 不接受事件：{method}");
    }
    let index = ProjectDocumentIndex::open(workspace)?;
    initialize_schema(&index)?;
    ensure_active(&index, window_id)?;
    let hook_count = usize::from(method == "hook/completed");
    let turn_count = usize::from(method == "turn/completed");
    let item_count = usize::from(method == "item/completed");
    let native_file_read_count =
        usize::from(method == "item/completed" && item_is_native_file_read(&event));
    let input_tokens = token_value(&event, &["inputtokens", "input_tokens"]);
    let cached_input_tokens = token_value(
        &event,
        &[
            "cachedinputtokens",
            "cached_input_tokens",
            "cache_read_input_tokens",
        ],
    );
    let output_tokens = token_value(&event, &["outputtokens", "output_tokens"]);
    let changed = index.conn.execute(
        "UPDATE native_context_observation_windows SET
           event_count=event_count+1,
           hook_count=hook_count+?1,
           turn_count=turn_count+?2,
           item_count=item_count+?3,
           native_file_read_count=native_file_read_count+?4,
           input_tokens=MAX(input_tokens,?5),
           cached_input_tokens=MAX(cached_input_tokens,?6),
           output_tokens=MAX(output_tokens,?7)
         WHERE window_id=?8 AND status='active'",
        params![
            hook_count as i64,
            turn_count as i64,
            item_count as i64,
            native_file_read_count as i64,
            to_i64(input_tokens),
            to_i64(cached_input_tokens),
            to_i64(output_tokens),
            window_id,
        ],
    )?;
    if changed != 1 {
        bail!("runtime observation window 已结束或不存在");
    }
    Ok(json!({
        "status":"accepted",
        "method":method,
        "event_count_delta":1,
        "native_file_read_count_delta":native_file_read_count,
        "raw_event_payload_stored":false,
    }))
}

pub(crate) fn finish_window(
    workspace: &Path,
    window_id: &str,
    selected_memory_count: usize,
    returned_metadata_bytes: usize,
) -> Result<Value> {
    if selected_memory_count > 64 || returned_metadata_bytes > 4 * 1024 * 1024 {
        bail!("runtime observation 完成计数超出边界");
    }
    let index = ProjectDocumentIndex::open(workspace)?;
    initialize_schema(&index)?;
    let completed_at_ms = now_millis();
    let changed = index.conn.execute(
        "UPDATE native_context_observation_windows SET status='completed',completed_at_ms=?1,
           selected_memory_count=?2,returned_metadata_bytes=?3
         WHERE window_id=?4 AND status='active'",
        params![
            to_i64(completed_at_ms),
            selected_memory_count as i64,
            returned_metadata_bytes as i64,
            window_id,
        ],
    )?;
    if changed != 1 {
        bail!("runtime observation window 已结束或不存在");
    }
    load_window(&index, window_id)
}

pub(crate) fn overview(workspace: &Path, benchmark_key: Option<&str>) -> Result<Value> {
    let index = ProjectDocumentIndex::open(workspace)?;
    initialize_schema(&index)?;
    let benchmark_key = benchmark_key
        .filter(|value| !value.trim().is_empty())
        .map(normalized_key)
        .transpose()?;
    let filter = benchmark_key.as_deref().unwrap_or("");
    let (total, baseline, enabled) = index.conn.query_row(
        "SELECT COUNT(*),
           SUM(CASE WHEN measurement_window='baseline_without_project_memory' THEN 1 ELSE 0 END),
           SUM(CASE WHEN measurement_window='with_project_memory' THEN 1 ELSE 0 END)
         FROM native_context_observation_windows
         WHERE status='completed' AND (?1='' OR benchmark_key=?1)",
        params![filter],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
            ))
        },
    )?;
    let matched_benchmark_count = if benchmark_key.is_some() {
        if baseline > 0 && enabled > 0 {
            1
        } else {
            0
        }
    } else {
        index.conn.query_row(
            "SELECT COUNT(*) FROM (
               SELECT benchmark_key FROM native_context_observation_windows
               WHERE status='completed' GROUP BY benchmark_key
               HAVING SUM(CASE WHEN measurement_window='baseline_without_project_memory' THEN 1 ELSE 0 END)>0
                  AND SUM(CASE WHEN measurement_window='with_project_memory' THEN 1 ELSE 0 END)>0
             )",
            [],
            |row| row.get::<_, i64>(0),
        )?
    };
    let status = if matched_benchmark_count > 0 {
        "matched_windows_available"
    } else if total > 0 {
        "partial_measurement"
    } else {
        "no_measurements"
    };
    let comparison = if status == "matched_windows_available" && benchmark_key.is_some() {
        matched_comparison(&index, filter)?.unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    Ok(json!({
        "schema":"elon.project_context_runtime_observation_summary.v1",
        "adapter_status":"ingest_adapter_available",
        "measurement_status":status,
        "benchmark_key":benchmark_key,
        "completed_window_count":total.max(0),
        "baseline_window_count":baseline.max(0),
        "enabled_window_count":enabled.max(0),
        "matched_benchmark_count":matched_benchmark_count.max(0),
        "comparison":comparison,
        "raw_event_payloads_stored":false,
        "not_vendor_billing":true,
        "not_total_task_tokens":true,
    }))
}

fn matched_comparison(index: &ProjectDocumentIndex, benchmark_key: &str) -> Result<Option<Value>> {
    let baseline = latest_window(index, benchmark_key, "baseline_without_project_memory")?;
    let enabled = latest_window(index, benchmark_key, "with_project_memory")?;
    Ok(match (baseline, enabled) {
        (Some(baseline), Some(enabled)) => Some(json!({
            "baseline":baseline,
            "enabled":enabled,
            "input_token_delta":signed_delta(&enabled, &baseline, "input_tokens"),
            "elapsed_ms_delta":signed_delta(&enabled, &baseline, "elapsed_ms"),
            "native_file_read_delta":signed_delta(&enabled, &baseline, "native_file_read_count"),
            "claim_rule":"A negative delta is a measured reduction only for this matched benchmark key; it is not a universal savings claim."
        })),
        _ => None,
    })
}

fn latest_window(
    index: &ProjectDocumentIndex,
    benchmark_key: &str,
    measurement_window: &str,
) -> Result<Option<Value>> {
    index
        .conn
        .query_row(
            "SELECT window_id,started_at_ms,completed_at_ms,event_count,hook_count,turn_count,
               item_count,native_file_read_count,input_tokens,cached_input_tokens,output_tokens,
               selected_memory_count,returned_metadata_bytes
             FROM native_context_observation_windows
             WHERE benchmark_key=?1 AND measurement_window=?2 AND status='completed'
             ORDER BY completed_at_ms DESC LIMIT 1",
            params![benchmark_key, measurement_window],
            row_value,
        )
        .optional()
        .map_err(Into::into)
}

fn load_window(index: &ProjectDocumentIndex, window_id: &str) -> Result<Value> {
    index
        .conn
        .query_row(
            "SELECT window_id,started_at_ms,completed_at_ms,event_count,hook_count,turn_count,
               item_count,native_file_read_count,input_tokens,cached_input_tokens,output_tokens,
               selected_memory_count,returned_metadata_bytes
             FROM native_context_observation_windows WHERE window_id=?1",
            params![window_id],
            row_value,
        )
        .optional()?
        .ok_or_else(|| anyhow!("runtime observation window 不存在"))
}

fn row_value(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let started = row.get::<_, i64>(1)?.max(0) as u64;
    let completed = row.get::<_, i64>(2)?.max(0) as u64;
    Ok(json!({
        "window_id":row.get::<_, String>(0)?,
        "started_at_ms":started,
        "completed_at_ms":completed,
        "elapsed_ms":completed.saturating_sub(started),
        "event_count":row.get::<_, i64>(3)?.max(0),
        "hook_count":row.get::<_, i64>(4)?.max(0),
        "turn_count":row.get::<_, i64>(5)?.max(0),
        "item_count":row.get::<_, i64>(6)?.max(0),
        "native_file_read_count":row.get::<_, i64>(7)?.max(0),
        "input_tokens":row.get::<_, i64>(8)?.max(0),
        "cached_input_tokens":row.get::<_, i64>(9)?.max(0),
        "output_tokens":row.get::<_, i64>(10)?.max(0),
        "selected_memory_count":row.get::<_, i64>(11)?.max(0),
        "returned_metadata_bytes":row.get::<_, i64>(12)?.max(0),
    }))
}

fn ensure_active(index: &ProjectDocumentIndex, window_id: &str) -> Result<()> {
    let active = index
        .conn
        .query_row(
            "SELECT 1 FROM native_context_observation_windows WHERE window_id=?1 AND status='active'",
            params![window_id],
            |_| Ok(()),
        )
        .optional()?;
    active.ok_or_else(|| anyhow!("runtime observation window 已结束或不存在"))
}

fn initialize_schema(index: &ProjectDocumentIndex) -> Result<()> {
    index.conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS native_context_observation_windows(
           window_id TEXT PRIMARY KEY,benchmark_key TEXT NOT NULL,measurement_window TEXT NOT NULL,
           session_fingerprint TEXT NOT NULL,status TEXT NOT NULL,started_at_ms INTEGER NOT NULL,
           completed_at_ms INTEGER NOT NULL,event_count INTEGER NOT NULL,hook_count INTEGER NOT NULL,
           turn_count INTEGER NOT NULL,item_count INTEGER NOT NULL,native_file_read_count INTEGER NOT NULL,
           input_tokens INTEGER NOT NULL,cached_input_tokens INTEGER NOT NULL,output_tokens INTEGER NOT NULL,
           selected_memory_count INTEGER NOT NULL,returned_metadata_bytes INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS native_context_observation_lookup
           ON native_context_observation_windows(benchmark_key,measurement_window,status,completed_at_ms);",
    )?;
    Ok(())
}

fn accepted_method(method: &str) -> bool {
    matches!(
        method,
        "hook/started"
            | "hook/completed"
            | "thread/tokenUsage/updated"
            | "turn/started"
            | "turn/completed"
            | "item/started"
            | "item/completed"
    )
}

fn item_is_native_file_read(event: &Value) -> bool {
    let item = event.pointer("/params/item").unwrap_or(&Value::Null);
    let kind = text_value(item, &["type", "kind"]);
    let tool = text_value(item, &["toolName", "tool_name", "name"]);
    let combined = format!("{kind} {tool}").to_ascii_lowercase();
    combined.contains("fileread")
        || combined.contains("read_file")
        || combined.contains("readfile")
        || combined.contains("view_image")
}

fn text_value(value: &Value, keys: &[&str]) -> String {
    let Some(object) = value.as_object() else {
        return String::new();
    };
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .unwrap_or_default()
        .chars()
        .take(120)
        .collect()
}

fn token_value(value: &Value, keys: &[&str]) -> u64 {
    token_value_at_depth(value, keys, 0)
}

fn token_value_at_depth(value: &Value, keys: &[&str], depth: usize) -> u64 {
    if depth >= 8 {
        return 0;
    }
    match value {
        Value::Object(object) => object
            .iter()
            .map(|(key, value)| {
                let normalized = normalized_token_key(key);
                let direct = keys
                    .iter()
                    .any(|candidate| normalized_token_key(candidate) == normalized)
                    .then(|| value.as_u64().unwrap_or_default())
                    .unwrap_or_default();
                direct.max(token_value_at_depth(value, keys, depth + 1))
            })
            .max()
            .unwrap_or_default(),
        Value::Array(values) => values
            .iter()
            .map(|value| token_value_at_depth(value, keys, depth + 1))
            .max()
            .unwrap_or_default(),
        _ => 0,
    }
}

fn normalized_token_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !matches!(ch, '_' | '-' | '.'))
        .flat_map(char::to_lowercase)
        .collect()
}

fn signed_delta(enabled: &Value, baseline: &Value, field: &str) -> i64 {
    let value = |source: &Value| {
        source
            .get(field)
            .and_then(Value::as_i64)
            .unwrap_or_default()
    };
    value(enabled).saturating_sub(value(baseline))
}

fn normalized_key(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_BENCHMARK_KEY
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        bail!("benchmark_key 只能包含 1–80 个字母、数字、点、下划线或连字符");
    }
    Ok(value.to_string())
}

fn normalized_window(value: &str) -> Result<&'static str> {
    match value.trim() {
        "baseline_without_project_memory" => Ok("baseline_without_project_memory"),
        "with_project_memory" => Ok("with_project_memory"),
        _ => bail!(
            "measurement_window 仅支持 baseline_without_project_memory 或 with_project_memory"
        ),
    }
}

fn fingerprint(value: &str) -> String {
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    format!(
        "{:x}",
        Sha256::digest(format!("{nonce}\0{value}").as_bytes())
    )
    .chars()
    .take(32)
    .collect()
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

fn to_i64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}
