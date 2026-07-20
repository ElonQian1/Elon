// server/src/node_agent_client_maintenance.rs

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(windows)]
use std::process::{Command, Stdio};

#[derive(Deserialize)]
pub(crate) struct OpenTargetRequest {
    target: String,
}

#[derive(Deserialize)]
pub(crate) struct AutostartSetRequest {
    enabled: bool,
}

pub(crate) async fn status_handler() -> Json<Value> {
    Json(status_payload())
}

pub(crate) async fn autostart_status_handler() -> Json<Value> {
    Json(autostart_status_payload())
}

pub(crate) async fn autostart_set_handler(
    Json(req): Json<AutostartSetRequest>,
) -> (StatusCode, Json<Value>) {
    match set_autostart(req.enabled) {
        Ok(()) => {
            record_maintenance_event(
                "autostart",
                true,
                if req.enabled { "enabled" } else { "disabled" },
            );
            let mut payload = autostart_status_payload();
            if let Some(object) = payload.as_object_mut() {
                object.insert("ok".to_string(), Value::Bool(true));
                object.insert(
                    "message".to_string(),
                    Value::String(
                        if req.enabled {
                            "已开启开机自动守护。"
                        } else {
                            "已关闭开机自动守护。"
                        }
                        .to_string(),
                    ),
                );
            }
            (StatusCode::OK, Json(payload))
        }
        Err(error) => {
            record_maintenance_event("autostart", false, &error);
            error_response(StatusCode::BAD_REQUEST, error)
        }
    }
}

pub(crate) async fn open_target_handler(
    Json(req): Json<OpenTargetRequest>,
) -> (StatusCode, Json<Value>) {
    match open_target(&req.target) {
        Ok(path) => {
            record_maintenance_event("open_target", true, &req.target);
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "opened": path_to_string(&path),
                })),
            )
        }
        Err(error) => {
            record_maintenance_event("open_target", false, &format!("{}: {}", req.target, error));
            error_response(StatusCode::BAD_REQUEST, error)
        }
    }
}

pub(crate) async fn update_handler(
    State(runtime): State<Arc<crate::NodeRuntime>>,
) -> (StatusCode, Json<Value>) {
    match crate::node_agent_restart_drain::schedule_update(runtime, "local_admin", None, None).await
    {
        Ok(payload) => {
            record_maintenance_event("update", true, "scheduled");
            (StatusCode::OK, Json(payload))
        }
        Err(error) => {
            record_maintenance_event("update", false, &error);
            error_response(StatusCode::BAD_REQUEST, error)
        }
    }
}

pub(crate) async fn repair_handler() -> (StatusCode, Json<Value>) {
    match spawn_client_action(ClientAction::Repair) {
        Ok(()) => {
            record_maintenance_event("repair", true, "scheduled");
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "message": "已开始后台修复客户端入口；会重新创建主程序、卸载程序、开始菜单和网页唤起协议。已开启的开机守护会保留并迁移为当前用户计划任务。"
                })),
            )
        }
        Err(error) => {
            record_maintenance_event("repair", false, &error);
            error_response(StatusCode::BAD_REQUEST, error)
        }
    }
}

pub(crate) async fn uninstall_handler() -> (StatusCode, Json<Value>) {
    match spawn_client_action(ClientAction::Uninstall) {
        Ok(()) => {
            record_maintenance_event("uninstall", true, "scheduled");
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "message": "已安排卸载；本机节点会退出并清理安装目录。"
                })),
            )
        }
        Err(error) => {
            record_maintenance_event("uninstall", false, &error);
            error_response(StatusCode::BAD_REQUEST, error)
        }
    }
}

mod payload_helpers;
use self::payload_helpers::*;

mod actions;
use self::actions::*;

pub(crate) mod maintenance_ops;
#[cfg(test)]
mod maintenance_test;
pub(crate) use self::maintenance_ops::*;
