use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

mod command;
mod memory;
mod repair;
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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DoctorRepairRequest {
    action: String,
    confirm: Option<bool>,
    adapter_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DoctorMemorySaveRequest {
    problem: String,
    summary: String,
    fix: Option<String>,
    result: Option<String>,
}

pub(crate) async fn snapshot_handler(
    State(_runtime): State<Arc<crate::NodeRuntime>>,
) -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "snapshot": snapshot::collect_snapshot(),
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

    let Some(token) = runtime.user_token().await else {
        return json_status(
            StatusCode::UNAUTHORIZED,
            json!({"ok": false, "error": "尚未登录，无法使用远程 AI 分析"}),
        );
    };

    let snapshot = snapshot::collect_snapshot();
    let memories =
        memory::relevant_memories(&problem, &memory::read_memory_items().unwrap_or_default());
    let user_prompt = build_doctor_prompt(&problem, &snapshot, &memories);
    let body = json!({
        "messages": [
            { "role": "system", "content": DOCTOR_SYSTEM_PROMPT },
            { "role": "user", "content": user_prompt }
        ]
    });
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
            return json_status(
                StatusCode::BAD_GATEWAY,
                json!({"ok": false, "error": format!("连接远程 AI 失败: {error}")}),
            )
        }
    };

    let status = response.status();
    let value = match response.json::<Value>().await {
        Ok(value) => value,
        Err(error) => {
            return json_status(
                StatusCode::BAD_GATEWAY,
                json!({"ok": false, "error": format!("远程 AI 响应解析失败: {error}")}),
            )
        }
    };

    if !status.is_success() {
        return json_status(
            status,
            json!({"ok": false, "error": value.get("error").and_then(Value::as_str).unwrap_or("远程 AI 调用失败"), "raw": value}),
        );
    }

    let analysis = extract_llm_text(&value).unwrap_or_else(|| value.to_string());
    json_status(
        StatusCode::OK,
        json!({
            "ok": true,
            "analysis": analysis,
            "snapshot": snapshot,
            "memories": memories,
            "allowedRepairs": repair::allowed_repairs(),
        }),
    )
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
        Ok(item) => json_status(StatusCode::OK, json!({"ok": true, "item": item})),
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
) -> String {
    let memories_text = serde_json::to_string_pretty(memories).unwrap_or_else(|_| "[]".to_string());
    let snapshot_text = serde_json::to_string_pretty(snapshot).unwrap_or_else(|_| "{}".to_string());
    format!(
        "用户问题：\n{problem}\n\n历史电脑问题记忆（可能为空）：\n{memories_text}\n\n本机只读快照：\n{snapshot_text}\n\n请给出诊断结论、证据、建议动作。若建议执行修复，只能使用 allowedRepairs 中的 action，并说明为什么需要用户确认。输出格式使用轻量 Markdown：少量短清单即可，避免大表格和多层标题。"
    )
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
