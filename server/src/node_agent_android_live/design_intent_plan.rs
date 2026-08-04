use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{
    broker::LiveUiSession,
    design_session_store::{list_records, read_record, validate_design_session_id},
};

const PLAN_TOOL: &str = "ui_plan_design_intent";
const GET_TOOL: &str = "ui_get_design_intent_plan";
const MAX_RECORD_BYTES: u64 = 96 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesignIntentPlan {
    schema_version: u32,
    plan_id: String,
    task_id: Option<String>,
    intent_sha256: String,
    intent_summary: String,
    requested_platforms: Vec<String>,
    primary_platform: Option<String>,
    route: String,
    state_hints: Vec<String>,
    target_id: Option<String>,
    design_session_id: Option<String>,
    session_action: String,
    actions: Vec<IntentAction>,
    needs_clarification: bool,
    clarifications: Vec<String>,
    status: String,
    created_at: String,
    updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IntentAction {
    order: u32,
    action: String,
    tool: String,
    reason: String,
    requires_approval: bool,
}

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            PLAN_TOOL,
            "把用户自然语言和显式平台/路由提示编译为可持久审查的多端设计执行计划；只规划，不启动 Runtime、不修改源码。",
            json!({"type":"object","additionalProperties":false,"required":["intent"],"properties":{
                "intent":{"type":"string","minLength":1,"maxLength":4000},
                "taskId":{"type":"string","minLength":1,"maxLength":160,"pattern":"^[A-Za-z0-9._:-]+$"},
                "platform":{"enum":["web","pwa","tauri","android"]},
                "route":{"type":"string","minLength":1,"maxLength":2048},
                "state":{"type":"string","maxLength":240},
                "designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"}
            }}),
            false,
        ),
        tool(
            GET_TOOL,
            "读取项目内已持久化的 DesignIntentPlan；返回摘要、目标和推荐工具顺序，不返回完整用户输入。",
            json!({"type":"object","additionalProperties":false,"required":["planId"],"properties":{
                "planId":{"type":"string","pattern":"^intent_[a-f0-9]{32}$"}
            }}),
            true,
        ),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(name, PLAN_TOOL | GET_TOOL)
}

pub(super) fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    let root = canonical_root(session)?;
    match name {
        PLAN_TOOL => create_plan(&root, &arguments),
        GET_TOOL => get_plan(&root, required_text(&arguments, "planId")?),
        _ => bail!("未知设计意图计划工具: {name}"),
    }
}

fn create_plan(root: &Path, arguments: &Value) -> Result<Value> {
    let intent = required_text(arguments, "intent")?;
    if intent.chars().count() > 4_000 || intent.contains('\0') {
        bail!("intent 过长或包含 NUL");
    }
    let task_id = optional_text(arguments, "taskId");
    if let Some(task_id) = task_id {
        super::design_task_binding::validate_task_id(task_id)?;
    }
    let explicit_platform = optional_text(arguments, "platform");
    let design_session_id = optional_text(arguments, "designSessionId");
    let bound_session = design_session_id
        .map(|id| {
            validate_design_session_id(id)?;
            read_record(root, id)
        })
        .transpose()?;
    let requested_platforms = infer_platforms(
        intent,
        explicit_platform,
        bound_session
            .as_ref()
            .map(|record| record.platform.as_str()),
    )?;
    let primary_platform = requested_platforms.first().cloned();
    let inferred_route = infer_route(intent);
    let route = normalize_route(
        optional_text(arguments, "route")
            .or(inferred_route.as_deref())
            .or_else(|| bound_session.as_ref().map(|record| record.route.as_str()))
            .unwrap_or("/"),
    )?;
    let mut state_hints = infer_states(intent);
    if let Some(state) = optional_text(arguments, "state") {
        push_unique(&mut state_hints, clean_summary(state, 240));
    }
    let (targets, _, _) = super::design_target_discovery::discover_targets(root)?;
    let target = primary_platform.as_deref().and_then(|platform| {
        targets
            .iter()
            .find(|target| target.platform.as_str() == platform)
    });
    let sessions = list_records(root, 50)?.0;
    let reusable = if let Some(record) = bound_session.as_ref() {
        primary_platform
            .as_deref()
            .is_none_or(|platform| record.platform.as_str() == platform)
            .then_some(record.design_session_id.clone())
    } else {
        sessions
            .iter()
            .find(|record| {
                primary_platform.as_deref() == Some(record.platform.as_str())
                    && record.route == route
            })
            .map(|record| record.design_session_id.clone())
    };
    let mut clarifications = Vec::new();
    if requested_platforms.is_empty() {
        clarifications.push("需要明确 Web、PWA、Tauri 或 Android 目标端".to_string());
    }
    if requested_platforms.len() > 1 && explicit_platform.is_none() {
        clarifications.push("检测到多个平台；执行前需要选择 primary platform".to_string());
    }
    if primary_platform.is_some() && target.is_none() {
        clarifications.push("项目目标发现器没有找到所选平台配置".to_string());
    }
    let session_action = if reusable.is_some() {
        "REUSE_SESSION"
    } else {
        "OPEN_SESSION"
    };
    let actions = build_actions(session_action);
    let now = chrono::Utc::now().to_rfc3339();
    let plan = DesignIntentPlan {
        schema_version: 1,
        plan_id: format!("intent_{}", uuid::Uuid::new_v4().simple()),
        task_id: task_id.map(str::to_string),
        intent_sha256: hex::encode(Sha256::digest(intent.as_bytes())),
        intent_summary: clean_summary(intent, 240),
        requested_platforms,
        primary_platform,
        route,
        state_hints,
        target_id: target.map(|target| target.id.clone()),
        design_session_id: reusable,
        session_action: session_action.to_string(),
        actions,
        needs_clarification: !clarifications.is_empty(),
        clarifications,
        status: "PLANNED".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };
    persist(root, &plan)?;
    Ok(
        json!({"schema":"elon.ui-design-intent-plan.v1","plan":plan,"sourceModified":false,"runtimeStarted":false}),
    )
}

fn get_plan(root: &Path, plan_id: &str) -> Result<Value> {
    validate_plan_id(plan_id)?;
    let path = plan_directory(root, false)?
        .context("设计意图计划目录不存在")?
        .join(format!("{plan_id}.json"));
    let metadata = fs::metadata(&path).context("DesignIntentPlan 不存在")?;
    if !metadata.is_file() || metadata.len() > MAX_RECORD_BYTES {
        bail!("DesignIntentPlan 无效或过大");
    }
    let plan: DesignIntentPlan =
        serde_json::from_slice(&fs::read(path)?).context("DesignIntentPlan JSON 无效")?;
    Ok(json!({"schema":"elon.ui-design-intent-plan.v1","plan":plan,"contentEmbedded":false}))
}

fn persist(root: &Path, plan: &DesignIntentPlan) -> Result<()> {
    let directory = plan_directory(root, true)?.context("无法创建设计意图计划目录")?;
    fs::write(
        directory.join(format!("{}.json", plan.plan_id)),
        serde_json::to_vec_pretty(plan)?,
    )?;
    prune(&directory)?;
    Ok(())
}

fn build_actions(session_action: &str) -> Vec<IntentAction> {
    let mut specs = vec![
        (
            session_action,
            if session_action == "REUSE_SESSION" {
                "ui_get_design_surface"
            } else {
                "ui_open_design_target"
            },
            "恢复或打开匹配平台和 route 的后台会话",
            false,
        ),
        (
            "CAPTURE_SURFACE",
            "ui_capture_design_surface",
            "生成当前像素和紧凑语义 UI tree 证据",
            false,
        ),
    ];
    specs.push((
        "BIND_TASK",
        "ui_bind_design_task",
        "获得 taskId 后让 PC 和后台消费者跟随同一 AI task",
        false,
    ));
    specs.extend([
        (
            "SELECT_NODE",
            "ui_get_design_surface",
            "按 selector、role 或 label 缩小目标节点",
            false,
        ),
        (
            "CREATE_OR_UPDATE_DRAFT",
            "ui_create_design_draft",
            "把修改表达为可撤销 DraftOperation",
            false,
        ),
        (
            "REVIEW_WRITEBACK",
            "ui_plan_design_writeback",
            "写回前检查绑定健康和分平台影响",
            true,
        ),
    ]);
    specs
        .into_iter()
        .enumerate()
        .map(|(index, (action, tool, reason, approval))| IntentAction {
            order: index as u32 + 1,
            action: action.to_string(),
            tool: tool.to_string(),
            reason: reason.to_string(),
            requires_approval: approval,
        })
        .collect()
}

fn infer_platforms(
    intent: &str,
    explicit: Option<&str>,
    session: Option<&str>,
) -> Result<Vec<String>> {
    if let Some(platform) = explicit {
        if !matches!(platform, "web" | "pwa" | "tauri" | "android") {
            bail!("platform 无效");
        }
        return Ok(vec![platform.to_string()]);
    }
    let lower = intent.to_ascii_lowercase();
    let mut values = Vec::new();
    for (platform, markers) in [
        ("tauri", &["tauri", "桌面客户端", "桌面端应用"] as &[&str]),
        ("android", &["android", "安卓", "apk"]),
        ("pwa", &["pwa", "移动网页", "手机网页"]),
        ("web", &["web", "网页端", "浏览器端"]),
    ] {
        if markers.iter().any(|marker| lower.contains(marker)) {
            values.push(platform.to_string());
        }
    }
    if values.is_empty() {
        if let Some(platform) = session {
            values.push(platform.to_string());
        }
    }
    Ok(values)
}

fn infer_route(intent: &str) -> Option<String> {
    intent.split_whitespace().find_map(|token| {
        let token =
            token.trim_matches(|ch: char| matches!(ch, '，' | '。' | ',' | ';' | ')' | ']' | '}'));
        (token.starts_with('/') && token.len() <= 2_048).then(|| token.to_string())
    })
}

fn infer_states(intent: &str) -> Vec<String> {
    let lower = intent.to_ascii_lowercase();
    let mut values = Vec::new();
    for (state, markers) in [
        (
            "AUTHENTICATED",
            &["已登录", "authenticated", "signed in"] as &[&str],
        ),
        ("ANONYMOUS", &["未登录", "anonymous", "signed out"]),
        ("LOADING", &["加载状态", "loading"]),
        ("EMPTY", &["空状态", "empty state"]),
        ("ERROR", &["错误状态", "error state"]),
        ("DARK_THEME", &["暗色", "深色", "dark mode"]),
    ] {
        if markers.iter().any(|marker| lower.contains(marker)) {
            values.push(state.to_string());
        }
    }
    values
}

fn normalize_route(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 2_048
        || value.chars().any(|ch| matches!(ch, '\0' | '\r' | '\n'))
        || !value.starts_with('/')
    {
        bail!("route 必须是以 / 开头的安全路径");
    }
    Ok(value.to_string())
}

fn prune(directory: &Path) -> Result<()> {
    let mut entries = fs::read_dir(directory)?
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    let remove_count = entries.len().saturating_sub(200);
    for entry in entries.into_iter().take(remove_count) {
        let _ = fs::remove_file(entry.path());
    }
    Ok(())
}

fn plan_directory(root: &Path, create: bool) -> Result<Option<PathBuf>> {
    let directory = root.join(".elon/ui-tuner/headless-design/intent-plans");
    if create {
        fs::create_dir_all(&directory)?;
    }
    if !directory.is_dir() {
        return Ok(None);
    }
    let canonical = directory.canonicalize()?;
    if !canonical.starts_with(root) {
        bail!("设计意图计划目录越出项目");
    }
    Ok(Some(canonical))
}

fn validate_plan_id(value: &str) -> Result<()> {
    if value.len() != 39
        || !value.starts_with("intent_")
        || !value[7..].chars().all(|ch| ch.is_ascii_hexdigit())
    {
        bail!("planId 无效");
    }
    Ok(())
}

fn clean_summary(value: &str, max: usize) -> String {
    value.trim().chars().take(max).collect()
}
fn push_unique(values: &mut Vec<String>, value: String) {
    if !value.is_empty() && !values.contains(&value) {
        values.push(value);
    }
}
fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    optional_text(value, key).ok_or_else(|| anyhow::anyhow!("缺少 {key}"))
}
fn optional_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
fn canonical_root(session: &LiveUiSession) -> Result<PathBuf> {
    PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("设计意图计划需要项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")
}
fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema,"annotations":{"readOnlyHint":read_only,"destructiveHint":false,"idempotentHint":read_only,"openWorldHint":false}})
}
