//! Local-admin-only producer binding and durable private snapshot outbox.
use crate::{
    node_agent_config::Credentials,
    private_read_projection::{
        self as contract,
        storage::{self, Binding},
    },
    NodeRuntime,
};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use rusqlite::{params, Connection, TransactionBehavior};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};

mod sync;
pub(crate) use sync::spawn;

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/private-read-projections", post(submit))
        .route("/api/private-read-projections/binding", get(binding))
        .route("/api/private-read-projections/status", get(status))
        .layer(DefaultBodyLimit::max(contract::MAX_BYTES))
}
fn database() -> anyhow::Result<Connection> {
    let path: PathBuf =
        crate::node_agent_config::state_path().with_file_name("private-read-projections-v1.sqlite");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(3))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
    storage::migrate(&conn)?;
    Ok(conn)
}
fn identity<'a>(creds: &'a Credentials, install: &'a str, hash: &'a str) -> Binding<'a> {
    Binding {
        owner: &creds.owner_user_id,
        node: &creds.agent_id,
        install,
        credential_hash: hash,
    }
}
fn binding_id(creds: &Credentials, install: &str, epoch: u64) -> String {
    contract::digest(
        serde_json::to_string(&json!([
            creds.owner_user_id,
            creds.agent_id,
            install,
            contract::digest(creds.agent_secret.as_bytes()),
            epoch
        ]))
        .unwrap()
        .as_bytes(),
    )
}
fn bound(creds: &Credentials, install: &str) -> bool {
    !creds.owner_user_id.is_empty()
        && creds.owner_user_id != "local-owner"
        && !creds.agent_id.is_empty()
        && !creds.agent_secret.is_empty()
        && !install.is_empty()
}
fn response(status: StatusCode, value: serde_json::Value) -> Response {
    (status, [("cache-control", "no-store")], Json(value)).into_response()
}
fn error(status: StatusCode, code: &'static str) -> Response {
    response(status, json!({"ok":false,"code":code}))
}
async fn binding(State(runtime): State<Arc<NodeRuntime>>) -> Response {
    if runtime.endpoint_credentials.endpoint_required().await {
        return error(
            StatusCode::CONFLICT,
            "projection_endpoint_authority_required",
        );
    }
    let (creds, epoch) = runtime.credential_session().await;
    let Some(creds) = creds.filter(|c| bound(c, &runtime.install_id)) else {
        return error(StatusCode::UNAUTHORIZED, "projection_node_unbound");
    };
    response(
        StatusCode::OK,
        json!({"schema":"yilong.private_read_projection.binding.v1",
        "binding_id":binding_id(&creds,&runtime.install_id,epoch),
        "research_owner_hash":contract::digest(creds.owner_user_id.as_bytes())}),
    )
}
async fn submit(
    State(runtime): State<Arc<NodeRuntime>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if runtime.endpoint_credentials.endpoint_required().await {
        return error(
            StatusCode::CONFLICT,
            "projection_endpoint_authority_required",
        );
    }
    let at = contract::now_ms();
    let value = match contract::parse(&body, at) {
        Ok(v) => v,
        Err(code) => return error(StatusCode::BAD_REQUEST, code),
    };
    let (creds, epoch) = runtime.credential_session().await;
    let Some(creds) = creds.filter(|c| bound(c, &runtime.install_id)) else {
        return error(StatusCode::UNAUTHORIZED, "projection_node_unbound");
    };
    let expected = binding_id(&creds, &runtime.install_id, epoch);
    if headers.get_all("x-elon-projection-binding").iter().count() != 1
        || headers
            .get("x-elon-projection-binding")
            .and_then(|v| v.to_str().ok())
            != Some(expected.as_str())
    {
        return error(StatusCode::CONFLICT, "projection_binding_changed");
    }
    let stored = runtime
        .with_current_credential_session(epoch, &creds, || -> anyhow::Result<bool> {
            let mut conn = database()?;
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let hash = contract::digest(creds.agent_secret.as_bytes());
            let changed = storage::put(
                &tx,
                &identity(&creds, &runtime.install_id, &hash),
                &value,
                at,
            )?;
            tx.commit()?;
            Ok(changed)
        })
        .await;
    match stored {
        Ok(Ok(changed)) => response(
            StatusCode::ACCEPTED,
            json!({"schema":"yilong.private_read_projection.receipt.v1",
          "revision":value.revision,"connection_id":value.connection_id,"status":if changed{"queued"}else{"unchanged"}}),
        ),
        Ok(Err(cause)) => error(StatusCode::CONFLICT, storage_error(&cause)),
        Err(_) => error(StatusCode::CONFLICT, "projection_binding_changed"),
    }
}
fn storage_error(cause: &anyhow::Error) -> &'static str {
    match cause.to_string().as_str() {
        "projection_revision_conflict" => "projection_revision_conflict",
        "projection_out_of_order" => "projection_out_of_order",
        "projection_freshness_conflict" => "projection_freshness_conflict",
        "projection_source_limit" => "projection_source_limit",
        "projection_total_limit" => "projection_total_limit",
        _ => "projection_storage_unavailable",
    }
}
async fn status(State(runtime): State<Arc<NodeRuntime>>) -> Response {
    if runtime.endpoint_credentials.endpoint_required().await {
        return error(
            StatusCode::CONFLICT,
            "projection_endpoint_authority_required",
        );
    }
    let (creds, epoch) = runtime.credential_session().await;
    let Some(creds) = creds.filter(|c| bound(c, &runtime.install_id)) else {
        return error(StatusCode::UNAUTHORIZED, "projection_node_unbound");
    };
    let result=runtime.with_current_credential_session(epoch,&creds,|| -> anyhow::Result<Vec<serde_json::Value>> {
        let conn=database()?; let hash=contract::digest(creds.agent_secret.as_bytes());
        let mut stmt=conn.prepare("SELECT connection_id,revision,synced_at_ms,rejected,last_error_code,last_success_at_ms FROM private_read_projection_heads
          WHERE owner_id=?1 AND node_id=?2 AND install_id=?3 AND credential_hash=?4 ORDER BY connection_id LIMIT 32")?;
        let rows=stmt.query_map(params![creds.owner_user_id,creds.agent_id,runtime.install_id,hash],|r| {
          let success:Option<u64>=r.get(2)?; let rejected:bool=r.get(3)?;
          Ok(json!({"connection_id":r.get::<_,String>(0)?,"current_revision":r.get::<_,String>(1)?,"pending_revision":if success.is_none()&&!rejected{Some(r.get::<_,String>(1)?)}else{None},
            "last_success_at_ms":r.get::<_,Option<u64>>(5)?,"last_error_code":r.get::<_,Option<String>>(4)?,
            "status":if rejected{"rejected"}else if success.is_some(){"synced"}else{"pending"}}))
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }).await;
    match result {
        Ok(Ok(items)) => response(
            StatusCode::OK,
            json!({"schema":"yilong.private_read_projection.status.v1","items":items}),
        ),
        _ => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "projection_status_unavailable",
        ),
    }
}
