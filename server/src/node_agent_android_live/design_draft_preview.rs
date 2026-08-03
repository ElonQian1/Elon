use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

use super::broker::LiveUiSession;

const PREVIEW_TOOL: &str = "ui_preview_design_draft";
const RESTORE_TOOL: &str = "ui_restore_design_draft_preview";

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            PREVIEW_TOOL,
            "把设计草稿的白名单视觉 patch 临时应用到持久浏览器并重新捕获；只改变当前页面内联样式，不写源码、不产生完成证据。",
        ),
        tool(
            RESTORE_TOOL,
            "恢复该草稿 selector 在当前持久浏览器中的预览前内联样式并重新捕获；不修改草稿或源码。",
        ),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(name, PREVIEW_TOOL | RESTORE_TOOL)
}

pub(super) async fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    let draft_id = required_text(&arguments, "draftId")?;
    let draft_result =
        super::design_drafts::call(session, "ui_get_design_draft", json!({"draftId":draft_id}))?;
    let draft = draft_result
        .get("draft")
        .context("设计草稿响应缺少 draft")?;
    let design_session_id = required_text(draft, "designSessionId")?;
    let selector = required_text(draft, "selector")?;
    let revision = draft
        .get("revision")
        .and_then(Value::as_u64)
        .context("设计草稿缺少 revision")?;
    let step = match name {
        PREVIEW_TOOL => preview_step(draft, selector)?,
        RESTORE_TOOL => json!({"action":"restoreStyle","selector":selector}),
        _ => bail!("未知设计草稿预览工具: {name}"),
    };
    let browser_arguments = json!({
        "designSessionId":design_session_id,
        "capture":{"steps":[step]}
    });
    let mut capture = super::design_browser_runtime::call(
        session,
        "ui_interact_design_browser",
        browser_arguments.clone(),
    )
    .await?;
    let mut prepared_now = false;
    if diagnostic_code(&capture) == Some("BROWSER_SESSION_NOT_PREPARED") {
        capture = super::design_browser_runtime::call(
            session,
            "ui_prepare_design_browser",
            browser_arguments,
        )
        .await?;
        prepared_now = true;
    }
    Ok(json!({
        "schema":"elon.ui-design-draft-preview.v1",
        "draftId":draft_id,
        "designSessionId":design_session_id,
        "revision":revision,
        "action":if name == PREVIEW_TOOL {"PREVIEW"} else {"RESTORE"},
        "previewOnly":true,
        "sourceModified":false,
        "completionEvidence":false,
        "browserPreparedNow":prepared_now,
        "capture":capture,
        "contentEmbedded":false
    }))
}

fn preview_step(draft: &Value, selector: &str) -> Result<Value> {
    let patches = draft
        .get("patches")
        .and_then(Value::as_array)
        .context("设计草稿缺少 patches")?;
    if patches.is_empty() || patches.len() > 32 {
        bail!("DESIGN_DRAFT_PREVIEW_PATCH_LIMIT：预览需要 1..32 个样式 patch");
    }
    let patches = patches
        .iter()
        .map(|patch| {
            Ok(json!({
                "property":required_text(patch, "property")?,
                "value":required_text(patch, "after")?
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"action":"previewStyle","selector":selector,"patches":patches}))
}

fn diagnostic_code(value: &Value) -> Option<&str> {
    value.pointer("/diagnostic/code").and_then(Value::as_str)
}

fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("缺少 {key}"))
}

fn tool(name: &str, description: &str) -> Value {
    json!({
        "name":name,
        "description":description,
        "inputSchema":{"type":"object","additionalProperties":false,"required":["draftId"],
            "properties":{"draftId":{"type":"string","pattern":"^draft_[a-f0-9]{32}$"}}},
        "annotations":{"readOnlyHint":false,"destructiveHint":false,
            "idempotentHint":name == RESTORE_TOOL,"openWorldHint":false}
    })
}
