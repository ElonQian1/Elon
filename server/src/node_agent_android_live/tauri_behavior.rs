use std::{fs, path::PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{
    broker::LiveUiSession,
    design_session_store::{persist_record, read_record, validate_design_session_id},
    design_targets::DesignPlatform,
    tauri_behavior_windows::{capture_native_behavior, ObservedMenuItem},
    tauri_host_runtime::registered_runtime,
};

const TOOL_NAME: &str = "ui_capture_tauri_behavior";
const MAX_TRACE_BYTES: u64 = 256 * 1024;
const MAX_TRACE_EVENTS: usize = 200;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Expectations {
    #[serde(default)]
    menu_labels: Vec<String>,
    #[serde(default)]
    dialog_titles: Vec<String>,
    #[serde(default)]
    commands: Vec<CommandExpectation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CommandExpectation {
    name: String,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CommandTraceEvent {
    schema: String,
    invocation_id: String,
    command: String,
    status: String,
    timestamp: String,
    #[serde(default)]
    result_sha256: Option<String>,
}

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![tool(
        TOOL_NAME,
        "读取当前 Tauri Runtime 后代进程的原生菜单、候选系统对话框和项目插桩 Rust command trace，形成分层 JSON/SHA-256 证据；不点击菜单、不调用任意 command，也不把项目 trace 冒充操作系统证明。",
        json!({
            "type":"object","additionalProperties":false,"required":["designSessionId"],
            "properties":{
                "designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"},
                "expectations":{"type":"object","additionalProperties":false,"properties":{
                    "menuLabels":{"type":"array","maxItems":32,"items":{"type":"string","minLength":1,"maxLength":160}},
                    "dialogTitles":{"type":"array","maxItems":16,"items":{"type":"string","minLength":1,"maxLength":240}},
                    "commands":{"type":"array","maxItems":32,"items":{"type":"object","additionalProperties":false,"required":["name"],"properties":{
                        "name":{"type":"string","minLength":1,"maxLength":160},
                        "status":{"enum":["STARTED","SUCCEEDED","FAILED"]}
                    }}}
                }}
            }
        }),
        false,
    )]
}

pub(super) fn is_tool(name: &str) -> bool {
    name == TOOL_NAME
}

pub(super) async fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    if name != TOOL_NAME {
        bail!("未知 Tauri 行为证据工具: {name}");
    }
    capture(session, &arguments).await
}

async fn capture(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let id = arguments
        .get("designSessionId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("缺少 designSessionId"))?;
    validate_design_session_id(id)?;
    let root = canonical_root(session)?;
    let mut record = read_record(&root, id)?;
    if record.platform != DesignPlatform::Tauri {
        bail!("TAURI_DESIGN_SESSION_REQUIRED：行为证据只支持 Tauri 会话");
    }
    let runtime = registered_runtime(id, &root).await?;
    let launcher = runtime.launcher_process_id;
    let native = tokio::task::spawn_blocking(move || capture_native_behavior(launcher)).await??;
    let (command_events, invalid_trace_count, trace_present) =
        read_command_trace(&root, &runtime.command_trace_path)?;
    let expectations = arguments
        .get("expectations")
        .cloned()
        .map(serde_json::from_value::<Expectations>)
        .transpose()
        .context("Tauri 行为 expectations 无效")?
        .unwrap_or(Expectations {
            menu_labels: Vec::new(),
            dialog_titles: Vec::new(),
            commands: Vec::new(),
        });
    validate_expectations(&expectations)?;
    let assertions = evaluate_expectations(
        &native.menus,
        &native.dialogs,
        &command_events,
        &expectations,
    );
    let captured_at = chrono::Utc::now().to_rfc3339();
    let artifact_value = json!({
        "schema":"elon.tauri.native-behavior.v1","designSessionId":id,
        "runtime":{"runtimeId":runtime.runtime_id,"launcherProcessId":runtime.launcher_process_id,
            "startedAt":runtime.started_at},
        "native":native,"rustCommands":{"coverage":if trace_present {"PROJECT_INSTRUMENTED_TRACE"} else {"NOT_INSTRUMENTED"},
            "tracePath":runtime.command_trace_path,"events":command_events,
            "invalidEventCount":invalid_trace_count,"payloadsEmbedded":false},
        "assertions":assertions,"capturedAt":captured_at,"base64Embedded":false,
    });
    let bytes = serde_json::to_vec_pretty(&artifact_value)?;
    let sha256 = hex::encode(Sha256::digest(&bytes));
    fs::create_dir_all(&runtime.evidence_directory)?;
    let path = runtime.evidence_directory.join(format!(
        "behavior-{}-{}.json",
        chrono::Utc::now().timestamp_millis(),
        &sha256[..16]
    ));
    fs::write(&path, &bytes)?;
    let evidence = json!({
        "hostCoverage":"TAURI_NATIVE_BEHAVIOR","artifact":{"path":path,"sha256":sha256,
            "bytes":bytes.len(),"mediaType":"application/json"},
        "menuCoverage":"WIN32_NATIVE_MENU_OBSERVED","menuItemCount":menu_count(&artifact_value["native"]["menus"]),
        "dialogCoverage":"DESCENDANT_TOP_LEVEL_WINDOWS_OBSERVED",
        "dialogCount":artifact_value["native"]["dialogs"].as_array().map(Vec::len).unwrap_or(0),
        "rustCommandCoverage":if trace_present {"PROJECT_INSTRUMENTED_TRACE"} else {"NOT_INSTRUMENTED"},
        "commandEventCount":artifact_value["rustCommands"]["events"].as_array().map(Vec::len).unwrap_or(0),
        "assertionsPassed":artifact_value["assertions"]["passed"],"capturedAt":captured_at,
        "base64Embedded":false,
    });
    let mut session_evidence = record.last_evidence.take().unwrap_or_else(|| json!({}));
    session_evidence
        .as_object_mut()
        .context("designSession evidence 必须是对象")?
        .insert("nativeBehavior".into(), evidence.clone());
    record.last_evidence = Some(session_evidence);
    record.state = "TAURI_BEHAVIOR_CAPTURED".into();
    record.updated_at = chrono::Utc::now().to_rfc3339();
    persist_record(&root, &record)?;
    Ok(json!({"ok":true,"status":"CAPTURED","designSessionId":id,
        "nativeBehavior":evidence,"snapshot":artifact_value,"contentEmbedded":false}))
}

fn read_command_trace(
    root: &std::path::Path,
    path: &std::path::Path,
) -> Result<(Vec<CommandTraceEvent>, usize, bool)> {
    if !path.is_file() {
        return Ok((Vec::new(), 0, false));
    }
    let canonical = path.canonicalize()?;
    let metadata = fs::metadata(&canonical)?;
    if !canonical.starts_with(root) || metadata.len() > MAX_TRACE_BYTES {
        bail!("TAURI_COMMAND_TRACE_REJECTED：trace 越出项目或超过 256KiB");
    }
    let text = fs::read_to_string(canonical).context("Tauri command trace 不是 UTF-8")?;
    let mut events = Vec::new();
    let mut invalid = 0usize;
    for line in text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(MAX_TRACE_EVENTS)
    {
        match serde_json::from_str::<CommandTraceEvent>(line)
            .ok()
            .filter(valid_trace_event)
        {
            Some(event) => events.push(event),
            None => invalid += 1,
        }
    }
    Ok((events, invalid, true))
}

fn valid_trace_event(event: &CommandTraceEvent) -> bool {
    event.schema == "elon.tauri.command-event.v1"
        && valid_text(&event.invocation_id, 160)
        && valid_text(&event.command, 160)
        && matches!(event.status.as_str(), "STARTED" | "SUCCEEDED" | "FAILED")
        && chrono::DateTime::parse_from_rfc3339(&event.timestamp).is_ok()
        && event
            .result_sha256
            .as_ref()
            .is_none_or(|value| value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()))
}

fn validate_expectations(expectations: &Expectations) -> Result<()> {
    if expectations.menu_labels.len() > 32
        || expectations.dialog_titles.len() > 16
        || expectations.commands.len() > 32
        || expectations
            .menu_labels
            .iter()
            .any(|value| !valid_text(value, 160))
        || expectations
            .dialog_titles
            .iter()
            .any(|value| !valid_text(value, 240))
        || expectations.commands.iter().any(|value| {
            !valid_text(&value.name, 160)
                || value.status.as_ref().is_some_and(|status| {
                    !matches!(status.as_str(), "STARTED" | "SUCCEEDED" | "FAILED")
                })
        })
    {
        bail!("TAURI_BEHAVIOR_EXPECTATION_INVALID：期望为空、过长或超出数量上限");
    }
    Ok(())
}

fn evaluate_expectations(
    menus: &[ObservedMenuItem],
    dialogs: &[super::tauri_behavior_windows::ObservedWindow],
    commands: &[CommandTraceEvent],
    expected: &Expectations,
) -> Value {
    let mut labels = Vec::new();
    flatten_menu_labels(menus, &mut labels);
    let missing_menus = expected
        .menu_labels
        .iter()
        .filter(|wanted| !labels.iter().any(|actual| same_text(actual, wanted)))
        .cloned()
        .collect::<Vec<_>>();
    let missing_dialogs = expected
        .dialog_titles
        .iter()
        .filter(|wanted| {
            !dialogs
                .iter()
                .any(|dialog| contains_text(&dialog.title, wanted))
        })
        .cloned()
        .collect::<Vec<_>>();
    let missing_commands = expected
        .commands
        .iter()
        .filter(|wanted| {
            !commands.iter().any(|actual| {
                same_text(&actual.command, &wanted.name)
                    && wanted
                        .status
                        .as_ref()
                        .is_none_or(|status| actual.status.as_str() == status)
            })
        })
        .map(|value| json!({"name":value.name,"status":value.status}))
        .collect::<Vec<_>>();
    json!({"passed":missing_menus.is_empty() && missing_dialogs.is_empty() && missing_commands.is_empty(),
        "missingMenus":missing_menus,"missingDialogs":missing_dialogs,
        "missingCommands":missing_commands,"expectationCount":expected.menu_labels.len()
            + expected.dialog_titles.len() + expected.commands.len()})
}

fn flatten_menu_labels(items: &[ObservedMenuItem], output: &mut Vec<String>) {
    for item in items {
        if !item.label.is_empty() {
            output.push(item.label.clone());
        }
        flatten_menu_labels(&item.children, output);
    }
}

fn menu_count(value: &Value) -> usize {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| 1 + menu_count(&item["children"]))
                .sum()
        })
        .unwrap_or(0)
}

fn valid_text(value: &str, max: usize) -> bool {
    let value = value.trim();
    !value.is_empty() && value.chars().count() <= max && !value.contains(['\r', '\0'])
}

fn same_text(left: &str, right: &str) -> bool {
    left.trim()
        .replace('&', "")
        .eq_ignore_ascii_case(&right.trim().replace('&', ""))
}

fn contains_text(actual: &str, wanted: &str) -> bool {
    actual
        .to_ascii_lowercase()
        .contains(&wanted.to_ascii_lowercase())
}

fn canonical_root(session: &LiveUiSession) -> Result<PathBuf> {
    PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("Tauri 行为证据需要绑定项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")
}

fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema,
        "annotations":{"readOnlyHint":read_only,"destructiveHint":false,
            "idempotentHint":read_only,"openWorldHint":false}})
}
