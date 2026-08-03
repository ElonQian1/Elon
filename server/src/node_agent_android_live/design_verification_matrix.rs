use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

use super::{broker::LiveUiSession, design_session_store::read_record};
use crate::node_agent_source_preview::{get_writeback_receipt, WritebackReceipt};

const CAPABILITIES_TOOL: &str = "ui_get_design_capabilities";
const MATRIX_TOOL: &str = "ui_get_design_verification_matrix";
const RUNTIME_SCHEMA: &str = "yilong-ui-live@1.8.0";

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            CAPABILITIES_TOOL,
            "返回当前节点实际安装的后台多端设计能力、协议版本和安全边界；调用成功本身就是节点已升级到该 schema 的证据。",
            json!({"type":"object","additionalProperties":false,"properties":{}}),
        ),
        tool(
            MATRIX_TOOL,
            "基于设计草稿、设计会话证据和写回回执生成 Web/PWA/Tauri/Android 验证矩阵；明确区分代码已具备、运行中、证据缺失和已验证。",
            json!({"type":"object","additionalProperties":false,"required":["draftId"],"properties":{"draftId":{"type":"string","pattern":"^draft_[a-f0-9]{32}$"}}}),
        ),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(name, CAPABILITIES_TOOL | MATRIX_TOOL)
}

pub(super) fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    match name {
        CAPABILITIES_TOOL => capabilities(session),
        MATRIX_TOOL => verification_matrix(session, &arguments),
        _ => bail!("未知设计验证工具: {name}"),
    }
}

fn capabilities(session: &LiveUiSession) -> Result<Value> {
    let root = canonical_root(session)?;
    let (targets, files_inspected, truncated) =
        super::design_target_discovery::discover_targets(&root)?;
    let detected = targets
        .iter()
        .map(|target| target.platform.as_str())
        .collect::<Vec<_>>();
    Ok(json!({
        "schema":"elon.ui-design-capabilities.v1",
        "runtimeSchema":RUNTIME_SCHEMA,
        "protocolRevision":"1.8",
        "installedRuntimeEvidence":{"source":"MCP_TOOL_RESPONSE","tool":CAPABILITIES_TOOL},
        "capabilityIds":[
            "PROJECT_SCOPED_DESIGN_SESSIONS",
            "STATEFUL_BROWSER_RUNTIME",
            "FIXTURE_BACKED_FORM_INTERACTION",
            "TAURI_NATIVE_WINDOW_CAPTURE",
            "TAURI_NATIVE_MENU_INSPECTION",
            "TAURI_NATIVE_DIALOG_INSPECTION",
            "TAURI_PROJECT_COMMAND_TRACE",
            "REVISIONED_DESIGN_DRAFTS",
            "DESIGN_DRAFT_LIVE_PREVIEW",
            "DESIGN_SOURCE_BINDING_CANDIDATES",
            "EVIDENCE_GATED_WRITEBACK",
            "PLATFORM_VERIFICATION_MATRIX"
        ],
        "platforms":{
            "web":{"surface":"HEADLESS_CHROMIUM","statefulInteraction":true},
            "pwa":{"surface":"HEADLESS_CHROMIUM_PWA","statefulInteraction":true},
            "tauri":{"surface":"WEBVIEW_PLUS_NATIVE_HOST","nativeBehavior":"LAYERED_EVIDENCE"},
            "android":{"surface":"ANDROID_LIVE_RUNTIME","statefulInteraction":true}
        },
        "limits":{"activeBrowserRuntimes":4,"browserIdleMinutes":15,"browserLifetimeMinutes":60,
            "browserOperations":128,"secretInputAllowed":false,"arbitraryTauriCommandAllowed":false},
        "project":{"detectedPlatforms":detected,"filesInspected":files_inspected,"scanTruncated":truncated},
        "contentEmbedded":false
    }))
}

fn verification_matrix(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let draft_id = arguments
        .get("draftId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("缺少 draftId"))?;
    let root = canonical_root(session)?;
    let draft_result =
        super::design_drafts::call(session, "ui_get_design_draft", json!({"draftId":draft_id}))?;
    let draft = draft_result
        .get("draft")
        .context("设计草稿响应缺少 draft")?;
    let design_session_id = required_text(draft, "designSessionId")?;
    let record = read_record(&root, design_session_id)?;
    let target_platforms = draft
        .get("targetPlatforms")
        .and_then(Value::as_array)
        .context("设计草稿缺少 targetPlatforms")?;
    let receipt = draft
        .get("writebackReceiptId")
        .and_then(Value::as_str)
        .map(|id| get_writeback_receipt(&root.to_string_lossy(), id))
        .transpose()?;
    let binding_ready = draft
        .pointer("/sourceBinding/status")
        .and_then(Value::as_str)
        == Some("BOUND");
    let patches_ready = draft
        .get("patches")
        .and_then(Value::as_array)
        .is_some_and(|patches| !patches.is_empty());
    let draft_ready = binding_ready && patches_ready;
    let rows = target_platforms
        .iter()
        .filter_map(Value::as_str)
        .map(|platform| platform_row(platform, &record, receipt.as_ref(), draft_ready))
        .collect::<Vec<_>>();
    let overall = overall_status(&rows, draft_ready, receipt.is_some());
    Ok(json!({
        "schema":"elon.ui-design-verification-matrix.v1",
        "runtimeSchema":RUNTIME_SCHEMA,
        "draft":{"draftId":draft_id,"revision":draft.get("revision"),"status":draft.get("status"),
            "bindingReady":binding_ready,"patchesReady":patches_ready},
        "designSession":{"designSessionId":design_session_id,"platform":record.platform,
            "state":record.state,"hasEvidence":record.last_evidence.is_some()},
        "receipt":receipt.as_ref().map(receipt_summary),
        "platforms":rows,"overallStatus":overall,
        "completionRule":"所有目标平台必须由 writeback receipt 提供 BUILD_VERIFIED 且 evidenceComplete=true",
        "runtimeTestsExecuted":false,"contentEmbedded":false
    }))
}

fn platform_row(
    platform: &str,
    record: &super::design_session_store::DesignSessionRecord,
    receipt: Option<&WritebackReceipt>,
    draft_ready: bool,
) -> Value {
    let result = receipt.and_then(|receipt| receipt.platform_results.get(platform));
    let receipt_status = result
        .map(|value| value.status.as_str())
        .unwrap_or("NOT_STARTED");
    let status = if result
        .is_some_and(|value| value.evidence_complete && value.status == "BUILD_VERIFIED")
    {
        "PASSED"
    } else if matches!(receipt_status, "FAILED" | "EVIDENCE_MISSING") {
        "BLOCKED"
    } else if receipt_status != "NOT_STARTED" && receipt_status != "PREVIEW" {
        "IN_PROGRESS"
    } else if draft_ready {
        "READY"
    } else {
        "NEEDS_DRAFT_OR_BINDING"
    };
    let current_surface = current_surface_evidence(platform, record);
    json!({
        "platform":platform,"status":status,
        "requirements":requirements(platform),
        "writeback":{"status":receipt_status,"method":result.map(|value| value.method.as_str()),
            "evidenceComplete":result.is_some_and(|value| value.evidence_complete),
            "changedFilesCount":result.map(|value| value.changed_files.len()).unwrap_or(0),
            "error":result.and_then(|value| value.error.as_deref())},
        "currentDesignSessionEvidence":current_surface,
        "codeCapabilityAvailable":true,"runtimeVerified":status == "PASSED"
    })
}

fn current_surface_evidence(
    platform: &str,
    record: &super::design_session_store::DesignSessionRecord,
) -> Value {
    let matches_session = record.platform.as_str() == platform;
    let evidence = matches_session
        .then_some(record.last_evidence.as_ref())
        .flatten();
    json!({
        "matchesDraftPlatform":matches_session,
        "pixels":evidence.and_then(|value| value.get("artifact")).is_some(),
        "uiTree":evidence.and_then(|value| value.get("uiTree")).is_some(),
        "nativeHost":evidence.and_then(|value| value.get("nativeHost")).is_some(),
        "nativeBehavior":evidence.and_then(|value| value.get("nativeBehavior")).map(|value| json!({
            "menuCoverage":value.get("menuCoverage"),"dialogCoverage":value.get("dialogCoverage"),
            "rustCommandCoverage":value.get("rustCommandCoverage"),"assertionsPassed":value.get("assertionsPassed")
        }))
    })
}

fn requirements(platform: &str) -> Vec<&'static str> {
    match platform {
        "web" => vec!["SOURCE_WRITEBACK", "BROWSER_CAPTURE", "ROUTE_REVISION"],
        "pwa" => vec!["SOURCE_WRITEBACK", "RUNTIME_RELOAD", "ROUTE_REVISION"],
        "tauri" => vec![
            "SOURCE_WRITEBACK",
            "FRONTEND_CAPTURE",
            "NATIVE_WINDOW_SHA256",
        ],
        "android" => vec!["SOURCE_WRITEBACK", "ANDROID_RUNTIME_CONNECTED", "APK_PATH"],
        _ => vec!["UNSUPPORTED_PLATFORM"],
    }
}

fn receipt_summary(receipt: &WritebackReceipt) -> Value {
    json!({"receiptId":receipt.receipt_id,"status":receipt.status,"complete":receipt.complete,
        "evidenceComplete":receipt.evidence_complete,"sourceRevision":receipt.source_revision,
        "updatedAt":receipt.updated_at})
}

fn overall_status(rows: &[Value], draft_ready: bool, has_receipt: bool) -> &'static str {
    if !rows.is_empty()
        && rows
            .iter()
            .all(|row| row.get("status").and_then(Value::as_str) == Some("PASSED"))
    {
        "PASSED"
    } else if rows
        .iter()
        .any(|row| row.get("status").and_then(Value::as_str) == Some("BLOCKED"))
    {
        "BLOCKED"
    } else if has_receipt {
        "IN_PROGRESS"
    } else if draft_ready {
        "READY"
    } else {
        "NEEDS_DRAFT_OR_BINDING"
    }
}

fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("设计草稿缺少 {key}"))
}

fn canonical_root(session: &LiveUiSession) -> Result<PathBuf> {
    PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("设计验证需要绑定项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema,
        "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
}
