use anyhow::{bail, Result};
use serde_json::Value;

use super::broker::LiveUiSession;

pub(super) fn tool_definitions() -> Vec<Value> {
    let mut definitions = super::design_targets::tool_definitions();
    definitions.extend(super::design_browser_runtime::tool_definitions());
    definitions.extend(super::tauri_host_runtime::tool_definitions());
    definitions.extend(super::tauri_behavior::tool_definitions());
    definitions.extend(super::design_drafts::tool_definitions());
    definitions.extend(super::design_draft_preview::tool_definitions());
    definitions.extend(super::design_source_binding::tool_definitions());
    definitions.extend(super::design_verification_matrix::tool_definitions());
    definitions
}

pub(super) fn is_tool(name: &str) -> bool {
    super::design_targets::is_tool(name)
        || super::design_browser_runtime::is_tool(name)
        || super::tauri_host_runtime::is_tool(name)
        || super::tauri_behavior::is_tool(name)
        || super::design_drafts::is_tool(name)
        || super::design_draft_preview::is_tool(name)
        || super::design_source_binding::is_tool(name)
        || super::design_verification_matrix::is_tool(name)
}

pub(super) async fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    if super::design_targets::is_tool(name) {
        return super::design_targets::call(session, name, arguments).await;
    }
    if super::design_browser_runtime::is_tool(name) {
        return super::design_browser_runtime::call(session, name, arguments).await;
    }
    if super::tauri_host_runtime::is_tool(name) {
        return super::tauri_host_runtime::call(session, name, arguments).await;
    }
    if super::tauri_behavior::is_tool(name) {
        return super::tauri_behavior::call(session, name, arguments).await;
    }
    if super::design_drafts::is_tool(name) {
        return super::design_drafts::call(session, name, arguments);
    }
    if super::design_draft_preview::is_tool(name) {
        return super::design_draft_preview::call(session, name, arguments).await;
    }
    if super::design_source_binding::is_tool(name) {
        return super::design_source_binding::call(session, arguments);
    }
    if super::design_verification_matrix::is_tool(name) {
        return super::design_verification_matrix::call(session, name, arguments);
    }
    bail!("未知后台设计工具: {name}")
}
