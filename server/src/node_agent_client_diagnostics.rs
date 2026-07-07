// server/src/node_agent_client_diagnostics.rs

use axum::{http::StatusCode, Json};
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
use std::process::{Command, Stdio};

const MAX_LATEST_TASKS: usize = 20;

pub(crate) async fn export_handler() -> (StatusCode, Json<Value>) {
    match export_diagnostics() {
        Ok(export) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "path": path_to_string(&export.path),
                "opened": export.opened,
                "message": "已生成客户端诊断信息。"
            })),
        ),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": error })),
        ),
    }
}

#[cfg(windows)]
pub(crate) fn export_diagnostics_file() -> Result<(PathBuf, bool), String> {
    export_diagnostics().map(|export| (export.path, export.opened))
}


#[path = "node_agent_client_diagnostics_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
