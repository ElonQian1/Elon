//! HTTP binding and canonical body hashing for ordinary local task POSTs.

use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{json_error, CreateLocalTaskRequest};
use crate::{node_agent_local_task_store::idempotency::IdempotencyClaim, NodeRuntime};

pub(crate) const IDEMPOTENCY_HEADER: &str = "x-elon-idempotency-key";
const METHOD: &str = "POST";
const PATH: &str = "/api/local-tasks";
const MAX_KEY_CHARS: usize = 200;

pub(crate) struct Binding {
    pub(crate) key: String,
    pub(crate) task_id: String,
    pub(crate) recover_existing: bool,
}

pub(crate) enum Begin {
    Unbound { task_id: String },
    Bound(Binding),
    Response(Response),
}

pub(crate) fn begin(
    runtime: &NodeRuntime,
    owner_user_id: &str,
    headers: &HeaderMap,
    request: &CreateLocalTaskRequest,
) -> Begin {
    let proposed_task_id = format!("local-{}", uuid::Uuid::new_v4());
    let Some(raw_key) = headers.get(IDEMPOTENCY_HEADER) else {
        return Begin::Unbound {
            task_id: proposed_task_id,
        };
    };
    let Ok(key) = raw_key.to_str() else {
        return Begin::Response(json_error(
            StatusCode::BAD_REQUEST,
            "idempotency key 必须是可见 ASCII 文本。",
        ));
    };
    let key = key.trim();
    if key.is_empty()
        || key.chars().count() > MAX_KEY_CHARS
        || !key.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Begin::Response(json_error(
            StatusCode::BAD_REQUEST,
            "idempotency key 必须为 1-200 个可见 ASCII 字符。",
        ));
    }
    let digest = match canonical_digest(request) {
        Ok(digest) => digest,
        Err(error) => {
            return Begin::Response(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            ))
        }
    };
    match runtime.local_tasks.claim_local_post(
        owner_user_id,
        key,
        METHOD,
        PATH,
        &digest,
        &proposed_task_id,
    ) {
        Ok(IdempotencyClaim::Claimed { task_id }) => {
            let recover_existing = runtime
                .local_tasks
                .get_for_owner(owner_user_id, &task_id)
                .ok()
                .flatten()
                .is_some();
            Begin::Bound(Binding {
                key: key.to_string(),
                task_id,
                recover_existing,
            })
        }
        Ok(IdempotencyClaim::Completed { status, body, .. }) => Begin::Response(
            (
                StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(body),
            )
                .into_response(),
        ),
        Ok(IdempotencyClaim::Conflict) => Begin::Response(json_error(
            StatusCode::CONFLICT,
            "IDEMPOTENCY_KEY_REUSED: key 已绑定到不同的 owner、method、path 或请求体。",
        )),
        Ok(IdempotencyClaim::InFlight { task_id }) => {
            match runtime.local_tasks.get_for_owner(owner_user_id, &task_id) {
                Ok(Some(record))
                    if runtime
                        .task_journal
                        .snapshot(&task_id, 0, 1)
                        .ok()
                        .and_then(|snapshot| snapshot.record)
                        .is_some()
                        || runtime
                            .cli_sidecars
                            .session_for_task(&task_id)
                            .ok()
                            .flatten()
                            .is_some() =>
                {
                    let body = serde_json::json!({
                        "ok": true, "task_id": task_id, "status": record.status,
                        "sync_state": record.sync_state, "record": record,
                        "idempotent_recovery": true,
                    });
                    match runtime.local_tasks.complete_local_post(
                        owner_user_id,
                        key,
                        &task_id,
                        StatusCode::ACCEPTED.as_u16(),
                        &body,
                    ) {
                        Ok(true) => Begin::Response((StatusCode::ACCEPTED, Json(body)).into_response()),
                        Ok(false) => Begin::Response(json_error(StatusCode::CONFLICT, "幂等绑定已改变。")),
                        Err(error) => Begin::Response(json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())),
                    }
                }
                Ok(Some(_)) | Ok(None) => Begin::Response(json_error(
                    StatusCode::TOO_EARLY,
                    "IDEMPOTENCY_REQUEST_IN_FLIGHT: 原请求尚未完成任务持久化，请使用同一 key 重试。",
                )),
                Err(error) => Begin::Response(json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())),
            }
        }
        Err(error) => Begin::Response(json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )),
    }
}

pub(crate) fn complete(
    runtime: &NodeRuntime,
    owner_user_id: &str,
    binding: Option<&Binding>,
    status: StatusCode,
    body: &serde_json::Value,
) -> Result<(), Response> {
    let Some(binding) = binding else {
        return Ok(());
    };
    match runtime.local_tasks.complete_local_post(
        owner_user_id,
        &binding.key,
        &binding.task_id,
        status.as_u16(),
        body,
    ) {
        Ok(true) => Ok(()),
        Ok(false) => Err(json_error(StatusCode::CONFLICT, "幂等绑定已改变。")),
        Err(error) => Err(json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )),
    }
}

pub(super) fn canonical_digest(value: &impl Serialize) -> anyhow::Result<String> {
    let value = serde_json::to_value(value)?;
    let mut bytes = Vec::new();
    write_canonical(&value, &mut bytes)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn write_canonical(value: &serde_json::Value, output: &mut Vec<u8>) -> anyhow::Result<()> {
    match value {
        serde_json::Value::Object(object) => {
            output.push(b'{');
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                output.extend(serde_json::to_vec(key)?);
                output.push(b':');
                write_canonical(&object[key], output)?;
            }
            output.push(b'}');
        }
        serde_json::Value::Array(array) => {
            output.push(b'[');
            for (index, item) in array.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                write_canonical(item, output)?;
            }
            output.push(b']');
        }
        scalar => output.extend(serde_json::to_vec(scalar)?),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_digest_is_key_order_independent() {
        let left = serde_json::json!({"b": [2, 1], "a": {"z": true, "x": "fake"}});
        let right = serde_json::json!({"a": {"x": "fake", "z": true}, "b": [2, 1]});
        assert_eq!(
            canonical_digest(&left).unwrap(),
            canonical_digest(&right).unwrap()
        );
    }
}
