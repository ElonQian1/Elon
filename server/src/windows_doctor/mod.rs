use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

mod command;
mod memory;
mod repair;
mod sessions;
mod snapshot;

const MAX_ANALYSIS_PROBLEM_CHARS: usize = 2_000;

const DOCTOR_SYSTEM_PROMPT: &str = r#"你是一龙 Windows 电脑医生。你会根据用户问题、历史问题记忆和只读系统快照诊断 Windows 电脑问题。

规则：
1. 默认只诊断，不要求用户执行危险命令。
2. 只能建议本系统白名单中的修复动作：flush_dns、reset_winhttp_proxy、clear_user_proxy、restart_adapter。
3. 如果要改代理、DNS、注册表、网卡或服务，必须明确说明影响范围，并要求用户确认。
4. 输出中文，先给结论，再给原因和建议动作。
5. 不要编造快照中没有的事实。
6. 可以使用轻量 Markdown 方便阅读，但不要堆叠多级标题，不要为了排版使用大表格。
7. 优先用 3 到 6 条短清单；命令、注册表路径、服务名和修复 action 用行内代码标注。"#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DoctorAnalyzeRequest {
    problem: String,
    session_id: Option<String>,
    agent: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DoctorRepairRequest {
    action: String,
    confirm: Option<bool>,
    adapter_name: Option<String>,
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DoctorSessionCreateRequest {
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DoctorSnapshotQuery {
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DoctorMemorySaveRequest {
    problem: String,
    summary: String,
    fix: Option<String>,
    result: Option<String>,
    session_id: Option<String>,
}

pub(crate) async fn snapshot_handler(
    State(_runtime): State<Arc<crate::NodeRuntime>>,
    Query(query): Query<DoctorSnapshotQuery>,
) -> (StatusCode, Json<Value>) {
    let snapshot = snapshot::collect_snapshot();
    if let Some(session_id) = query.session_id.as_deref() {
        let count = snapshot
            .get("commands")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let content = format!("已完成只读体检，采集到 {count} 组系统状态。");
        append_session_tool_message(session_id, &content, "tool");
    }
    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "snapshot": snapshot,
            "session": query.session_id.as_deref().and_then(|id| sessions::read_session(id).ok().flatten()),
            "sessions": sessions::list_session_summaries().unwrap_or_default(),
        })),
    )
}

pub(crate) async fn analyze_handler(
    State(runtime): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<DoctorAnalyzeRequest>,
) -> (StatusCode, Json<Value>) {
    let problem = normalize_text(&req.problem, MAX_ANALYSIS_PROBLEM_CHARS);
    if problem.is_empty() {
        return json_status(
            StatusCode::BAD_REQUEST,
            json!({"ok": false, "error": "问题不能为空"}),
        );
    }

    let mut session = match sessions::load_or_create(req.session_id.as_deref(), &problem) {
        Ok(session) => session,
        Err(error) => {
            return json_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"ok": false, "error": format!("读取诊断会话失败: {error}")}),
            );
        }
    };
    sessions::push_message(&mut session, "user", &problem, None);
    if let Err(error) = sessions::save_session(&session) {
        return json_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"ok": false, "error": format!("保存诊断会话失败: {error}")}),
        );
    }

    let Some(token) = runtime.user_token().await else {
        return analysis_error_response(
            StatusCode::UNAUTHORIZED,
            &mut session,
            "尚未登录，无法使用远程 AI 分析".to_string(),
            None,
        );
    };

    let snapshot = snapshot::collect_snapshot();
    let memories =
        memory::relevant_memories(&problem, &memory::read_memory_items().unwrap_or_default());
    let context_messages = sessions::context_messages(&session);
    let user_prompt = build_doctor_prompt(&problem, &snapshot, &memories, &context_messages);
    let mut body = json!({
        "messages": [
            { "role": "system", "content": DOCTOR_SYSTEM_PROMPT },
            { "role": "user", "content": user_prompt }
        ]
    });
    if let Some(agent) = req
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        body["agent"] = json!(agent);
    }
    let url = format!(
        "{}/api/agent/runtime/chat",
        runtime.cloud_http_url().trim_end_matches('/')
    );

    let response = match reqwest::Client::new()
        .post(url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return analysis_error_response(
                StatusCode::BAD_GATEWAY,
                &mut session,
                format!("连接远程 AI 失败: {error}"),
                None,
            );
        }
    };

    let status = response.status();
    let value = match response.json::<Value>().await {
        Ok(value) => value,
        Err(error) => {
            return analysis_error_response(
                StatusCode::BAD_GATEWAY,
                &mut session,
                format!("远程 AI 响应解析失败: {error}"),
                None,
            );
        }
    };

    if !status.is_success() {
        return analysis_error_response(
            status,
            &mut session,
            value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("远程 AI 调用失败")
                .to_string(),
            Some(json!({"raw": value})),
        );
    }

    let analysis = extract_llm_text(&value).unwrap_or_else(|| value.to_string());
    sessions::push_message(&mut session, "assistant", &analysis, Some("ok"));
    if let Err(error) = sessions::save_session(&session) {
        return json_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"ok": false, "error": format!("保存诊断会话失败: {error}")}),
        );
    }
    json_status(
        StatusCode::OK,
        json!({
            "ok": true,
            "analysis": analysis,
            "snapshot": snapshot,
            "memories": memories,
            "allowedRepairs": repair::allowed_repairs(),
            "session": session,
            "sessions": sessions::list_session_summaries().unwrap_or_default(),
        }),
    )
}

pub(crate) async fn sessions_list_handler(
    State(_runtime): State<Arc<crate::NodeRuntime>>,
) -> (StatusCode, Json<Value>) {
    match sessions::list_session_summaries() {
        Ok(items) => json_status(
            StatusCode::OK,
            json!({"ok": true, "items": items, "path": sessions::sessions_path().display().to_string()}),
        ),
        Err(error) => json_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"ok": false, "error": format!("读取诊断会话失败: {error}")}),
        ),
    }
}

pub(crate) async fn session_create_handler(
    State(_runtime): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<DoctorSessionCreateRequest>,
) -> (StatusCode, Json<Value>) {
    match sessions::create_session(req.title.as_deref()) {
        Ok(session) => json_status(
            StatusCode::OK,
            json!({
                "ok": true,
                "session": session,
                "sessions": sessions::list_session_summaries().unwrap_or_default(),
            }),
        ),
        Err(error) => json_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"ok": false, "error": format!("创建诊断会话失败: {error}")}),
        ),
    }
}

pub(crate) async fn session_get_handler(
    State(_runtime): State<Arc<crate::NodeRuntime>>,
    Path(session_id): Path<String>,
) -> (StatusCode, Json<Value>) {
    match sessions::read_session(&session_id) {
        Ok(Some(session)) => json_status(StatusCode::OK, json!({"ok": true, "session": session})),
        Ok(None) => json_status(
            StatusCode::NOT_FOUND,
            json!({"ok": false, "error": "诊断会话不存在"}),
        ),
        Err(error) => json_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"ok": false, "error": format!("读取诊断会话失败: {error}")}),
        ),
    }
}

pub(crate) async fn session_delete_handler(
    State(_runtime): State<Arc<crate::NodeRuntime>>,
    Path(session_id): Path<String>,
) -> (StatusCode, Json<Value>) {
    match sessions::delete_session(&session_id) {
        Ok(deleted) => json_status(
            StatusCode::OK,
            json!({
                "ok": true,
                "deleted": deleted,
                "sessions": sessions::list_session_summaries().unwrap_or_default(),
            }),
        ),
        Err(error) => json_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"ok": false, "error": format!("删除诊断会话失败: {error}")}),
        ),
    }
}

pub(crate) async fn memory_list_handler(
    State(_runtime): State<Arc<crate::NodeRuntime>>,
) -> (StatusCode, Json<Value>) {
    match memory::read_memory_items() {
        Ok(items) => json_status(
            StatusCode::OK,
            json!({"ok": true, "items": items, "path": memory::memory_path().display().to_string()}),
        ),
        Err(error) => json_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"ok": false, "error": format!("读取电脑问题记忆失败: {error}")}),
        ),
    }
}

pub(crate) async fn memory_save_handler(
    State(_runtime): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<DoctorMemorySaveRequest>,
) -> (StatusCode, Json<Value>) {
    let problem = normalize_text(&req.problem, MAX_ANALYSIS_PROBLEM_CHARS);
    let summary = normalize_text(&req.summary, 4_000);
    if problem.is_empty() || summary.is_empty() {
        return json_status(
            StatusCode::BAD_REQUEST,
            json!({"ok": false, "error": "问题和总结不能为空"}),
        );
    }

    match memory::save_memory_item(&problem, &summary, req.fix, req.result) {
        Ok(item) => {
            if let Some(session_id) = req.session_id.as_deref() {
                append_session_tool_message(
                    session_id,
                    "已把本次诊断保存为常见问题记忆，后续相似问题会优先复用。",
                    "tool",
                );
            }
            json_status(
                StatusCode::OK,
                json!({
                    "ok": true,
                    "item": item,
                    "session": req.session_id.as_deref().and_then(|id| sessions::read_session(id).ok().flatten()),
                    "sessions": sessions::list_session_summaries().unwrap_or_default(),
                }),
            )
        }
        Err(error) => json_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"ok": false, "error": format!("保存电脑问题记忆失败: {error}")}),
        ),
    }
}

pub(crate) async fn repair_handler(
    State(_runtime): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<DoctorRepairRequest>,
) -> (StatusCode, Json<Value>) {
    if !cfg!(windows) {
        return json_status(
            StatusCode::BAD_REQUEST,
            json!({"ok": false, "error": "电脑医生修复动作当前只支持 Windows 节点"}),
        );
    }

    let Some(plan) = repair::repair_plan(&req.action, req.adapter_name.as_deref()) else {
        return json_status(
            StatusCode::BAD_REQUEST,
            json!({"ok": false, "error": "未知或参数不足的修复动作", "allowedRepairs": repair::allowed_repairs()}),
        );
    };

    if req.confirm != Some(true) {
        return json_status(
            StatusCode::CONFLICT,
            json!({
                "ok": false,
                "requiresConfirm": true,
                "action": plan.action,
                "title": plan.title,
                "risk": plan.risk,
                "impact": plan.impact,
            }),
        );
    }

    let outcome = repair::execute_repair(&plan);
    let ok = outcome
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if let Some(session_id) = req.session_id.as_deref() {
        if let Ok(Some(mut session)) = sessions::read_session(session_id) {
            let detail = outcome
                .get("error")
                .or_else(|| outcome.get("stderr"))
                .or_else(|| outcome.get("stdout"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let content = if detail.is_empty() {
                format!("已执行白名单修复：{}。", plan.title)
            } else {
                format!("已执行白名单修复：{}。\n\n{}", plan.title, detail)
            };
            sessions::push_message(
                &mut session,
                "assistant",
                &content,
                Some(if ok { "ok" } else { "err" }),
            );
            let _ = sessions::save_session(&session);
        }
    }
    json_status(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::BAD_GATEWAY
        },
        json!({
            "ok": ok,
            "action": plan.action,
            "title": plan.title,
            "impact": plan.impact,
            "outcome": outcome,
        }),
    )
}

fn build_doctor_prompt(
    problem: &str,
    snapshot: &Value,
    memories: &[memory::DoctorMemoryItem],
    context_messages: &[sessions::DoctorSessionMessage],
) -> String {
    let memories_text = serde_json::to_string_pretty(memories).unwrap_or_else(|_| "[]".to_string());
    let context_text =
        serde_json::to_string_pretty(context_messages).unwrap_or_else(|_| "[]".to_string());
    let snapshot_text = serde_json::to_string_pretty(snapshot).unwrap_or_else(|_| "{}".to_string());
    format!(
        "本轮用户问题：\n{problem}\n\n当前诊断会话最近消息（用于理解追问上下文，可能包含本轮问题）：\n{context_text}\n\n历史电脑问题记忆（跨会话复用，可能为空）：\n{memories_text}\n\n本机只读快照：\n{snapshot_text}\n\n请基于同一会话上下文给出诊断结论、证据、建议动作。若建议执行修复，只能使用 allowedRepairs 中的 action，并说明为什么需要用户确认。输出格式使用轻量 Markdown：少量短清单即可，避免大表格和多层标题。"
    )
}

fn analysis_error_response(
    status: StatusCode,
    session: &mut sessions::DoctorSession,
    error: String,
    extra: Option<Value>,
) -> (StatusCode, Json<Value>) {
    sessions::push_message(session, "assistant", &error, Some("err"));
    let save_error = sessions::save_session(session)
        .err()
        .map(|err| err.to_string());
    let mut value = json!({
        "ok": false,
        "error": error,
        "session": session,
        "sessions": sessions::list_session_summaries().unwrap_or_default(),
    });
    if let Some(extra) = extra {
        merge_json_object(&mut value, extra);
    }
    if let Some(save_error) = save_error {
        value["sessionSaveError"] = json!(save_error);
    }
    json_status(status, value)
}

fn append_session_tool_message(session_id: &str, content: &str, kind: &str) {
    if let Ok(Some(mut session)) = sessions::read_session(session_id) {
        sessions::push_message(&mut session, "assistant", content, Some(kind));
        let _ = sessions::save_session(&session);
    }
}

fn merge_json_object(target: &mut Value, extra: Value) {
    let Some(target) = target.as_object_mut() else {
        return;
    };
    if let Some(extra) = extra.as_object() {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
}

fn extract_llm_text(value: &Value) -> Option<String> {
    value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            value
                .get("content")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn normalize_text(value: &str, max_chars: usize) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn json_status(status: StatusCode, value: Value) -> (StatusCode, Json<Value>) {
    (status, Json(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_text_trims_and_limits() {
        assert_eq!(normalize_text("  abc\u{0}def  ", 4), "abcd");
    }
}
