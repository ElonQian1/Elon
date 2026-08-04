use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde_json::{json, Value};

use super::{
    broker::LiveUiSession,
    design_intent_plan::{self, DesignIntentPlan},
    design_session_store,
};

const START_TOOL: &str = "ui_start_design_intent_plan";
const TRANSITION_TOOL: &str = "ui_transition_design_intent_plan";
const RECORD_ACTION_TOOL: &str = "ui_record_design_intent_action";
const REPLAN_TOOL: &str = "ui_replan_design_intent";

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            START_TOOL,
            "以 expectedRevision 启动 DesignIntentPlan，绑定 task lease；需要已有后台设计会话，不负责启动 Runtime。",
            start_schema(),
            false,
        ),
        tool(
            TRANSITION_TOOL,
            "以乐观并发控制暂停、恢复、取消、失败或完成 DesignIntentPlan；终态会尝试释放 task lease。",
            transition_schema(),
            false,
        ),
        tool(
            RECORD_ACTION_TOOL,
            "记录 DesignIntentPlan 单个动作的有界回执、错误码和证据引用；不嵌入证据正文。",
            action_schema(),
            false,
        ),
        tool(
            REPLAN_TOOL,
            "从非运行中的旧计划生成新 DesignIntentPlan，并以 supersededBy/replannedFrom 保存可追溯关系。",
            replan_schema(),
            false,
        ),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(
        name,
        START_TOOL | TRANSITION_TOOL | RECORD_ACTION_TOOL | REPLAN_TOOL
    )
}

pub(super) fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    let root = design_intent_plan::canonical_root(session)?;
    match name {
        START_TOOL => start(session, &root, &arguments),
        TRANSITION_TOOL => transition(session, &root, &arguments),
        RECORD_ACTION_TOOL => record_action(session, &root, &arguments),
        REPLAN_TOOL => replan(&root, &arguments),
        _ => bail!("未知设计意图执行工具: {name}"),
    }
}

fn start(session: &LiveUiSession, root: &std::path::Path, arguments: &Value) -> Result<Value> {
    let mut plan = load_expected(root, arguments)?;
    if plan.status != "PLANNED" {
        bail!("DESIGN_INTENT_STATE_CONFLICT：只有 PLANNED 计划可以启动");
    }
    if plan.needs_clarification {
        bail!("DESIGN_INTENT_CLARIFICATION_REQUIRED：计划仍有待确认问题");
    }
    let task_id = required_text(arguments, "taskId")?;
    super::design_task_binding::validate_task_id(task_id)?;
    if plan
        .task_id
        .as_deref()
        .is_some_and(|value| value != task_id)
    {
        bail!("DESIGN_INTENT_TASK_CONFLICT：taskId 与计划不一致");
    }
    let design_session_id = optional_text(arguments, "designSessionId")
        .or(plan.design_session_id.as_deref())
        .context("DESIGN_INTENT_SESSION_REQUIRED：请先打开后台设计会话")?
        .to_string();
    validate_session_matches(root, &plan, &design_session_id)?;

    let mut binding_arguments = json!({
        "taskId": task_id,
        "designSessionId": design_session_id.clone(),
    });
    for key in ["draftId", "expectedLeaseId", "leaseSeconds"] {
        if let Some(value) = arguments.get(key) {
            binding_arguments[key] = value.clone();
        }
    }
    let binding_result =
        super::design_task_binding::call(session, "ui_bind_design_task", binding_arguments)?;
    let lease_id = binding_result
        .pointer("/binding/leaseId")
        .and_then(Value::as_str)
        .context("任务绑定没有返回 leaseId")?
        .to_string();
    let binding_action = binding_result
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("BOUND");

    let now = Utc::now().to_rfc3339();
    plan.schema_version = 2;
    plan.revision += 1;
    plan.task_id = Some(task_id.to_string());
    plan.task_lease_id = Some(lease_id.clone());
    plan.design_session_id = Some(design_session_id);
    plan.status = "RUNNING".to_string();
    plan.started_at.get_or_insert_with(|| now.clone());
    plan.finished_at = None;
    plan.updated_at = now.clone();
    mark_binding_receipt(&mut plan, &now);
    if let Err(error) = design_intent_plan::persist_record(root, &plan) {
        if binding_action == "BOUND" {
            let _ = super::design_task_binding::call(
                session,
                "ui_settle_design_task_binding",
                json!({"taskId":task_id,"leaseId":lease_id,"succeeded":false}),
            );
        }
        return Err(error);
    }
    Ok(plan_result("STARTED", plan, Some(binding_result)))
}

fn transition(session: &LiveUiSession, root: &std::path::Path, arguments: &Value) -> Result<Value> {
    let mut plan = load_expected(root, arguments)?;
    let transition = required_text(arguments, "transition")?.to_ascii_uppercase();
    let reason = optional_text(arguments, "reason").map(|value| clean(value, 500));
    let next = match (plan.status.as_str(), transition.as_str()) {
        ("RUNNING", "PAUSE") => "PAUSED",
        ("PAUSED" | "FAILED", "RESUME") => "RUNNING",
        ("PLANNED" | "RUNNING" | "PAUSED" | "FAILED", "CANCEL") => "CANCELED",
        ("RUNNING", "FAIL") => "FAILED",
        ("RUNNING", "COMPLETE") if all_actions_settled(&plan) => "COMPLETED",
        ("RUNNING", "COMPLETE") => {
            bail!("DESIGN_INTENT_ACTIONS_INCOMPLETE：仍有动作没有成功或跳过")
        }
        _ => bail!(
            "DESIGN_INTENT_STATE_CONFLICT：{} 不允许 {}",
            plan.status,
            transition
        ),
    };
    if matches!(transition.as_str(), "FAIL" | "CANCEL") && reason.is_none() {
        bail!("reason 是失败或取消计划的必填说明");
    }
    let now = Utc::now().to_rfc3339();
    plan.revision += 1;
    plan.status = next.to_string();
    plan.execution_summary = reason;
    plan.updated_at = now.clone();
    if is_terminal(next) {
        plan.finished_at = Some(now);
    } else {
        plan.finished_at = None;
    }
    design_intent_plan::persist_record(root, &plan)?;
    let settlement = is_terminal(next).then(|| settle_binding(session, &plan));
    Ok(plan_result(transition.as_str(), plan, settlement))
}

fn record_action(
    session: &LiveUiSession,
    root: &std::path::Path,
    arguments: &Value,
) -> Result<Value> {
    let mut plan = load_expected(root, arguments)?;
    if plan.status != "RUNNING" {
        bail!("DESIGN_INTENT_STATE_CONFLICT：只有 RUNNING 计划可以写入动作回执");
    }
    let order = arguments
        .get("actionOrder")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .context("缺少或无效的 actionOrder")?;
    if !plan.actions.iter().any(|action| action.order == order) {
        bail!("DESIGN_INTENT_ACTION_UNKNOWN：actionOrder 不属于计划");
    }
    let status = required_text(arguments, "status")?.to_ascii_uppercase();
    let receipt = plan
        .action_receipts
        .iter_mut()
        .find(|receipt| receipt.order == order)
        .context("动作回执槽位不存在")?;
    validate_receipt_transition(&receipt.status, &status)?;
    let now = Utc::now().to_rfc3339();
    if status == "RUNNING" || receipt.attempt == 0 {
        receipt.attempt += 1;
    }
    receipt.status = status.clone();
    receipt.summary = optional_text(arguments, "summary").map(|value| clean(value, 500));
    receipt.error_code = optional_text(arguments, "errorCode").map(|value| clean(value, 80));
    receipt.evidence_refs = evidence_refs(arguments)?;
    receipt.updated_at = now.clone();
    plan.revision += 1;
    plan.updated_at = now.clone();
    let mut action = "ACTION_RECORDED";
    if status == "FAILED" {
        plan.status = "FAILED".to_string();
        plan.finished_at = Some(now);
        plan.execution_summary = receipt.summary.clone();
        action = "PLAN_FAILED";
    } else if all_actions_settled(&plan) {
        plan.status = "COMPLETED".to_string();
        plan.finished_at = Some(now);
        action = "PLAN_COMPLETED";
    }
    design_intent_plan::persist_record(root, &plan)?;
    let settlement = (plan.status == "COMPLETED").then(|| settle_binding(session, &plan));
    Ok(plan_result(action, plan, settlement))
}

fn replan(root: &std::path::Path, arguments: &Value) -> Result<Value> {
    let mut previous = load_expected(root, arguments)?;
    if previous.status == "RUNNING" {
        bail!("DESIGN_INTENT_STATE_CONFLICT：运行中的计划必须先暂停再重规划");
    }
    if previous.status == "SUPERSEDED" || previous.superseded_by.is_some() {
        bail!("DESIGN_INTENT_ALREADY_SUPERSEDED：旧计划已经被替代");
    }
    let mut next_arguments = arguments.clone();
    for key in ["planId", "expectedRevision"] {
        next_arguments
            .as_object_mut()
            .map(|object| object.remove(key));
    }
    inherit_text(&mut next_arguments, "taskId", previous.task_id.as_deref());
    inherit_text(
        &mut next_arguments,
        "designSessionId",
        previous.design_session_id.as_deref(),
    );
    inherit_text(
        &mut next_arguments,
        "platform",
        previous.primary_platform.as_deref(),
    );
    inherit_text(&mut next_arguments, "route", Some(&previous.route));
    let next =
        design_intent_plan::create_record(root, &next_arguments, Some(previous.plan_id.clone()))?;
    let now = Utc::now().to_rfc3339();
    previous.revision += 1;
    previous.status = "SUPERSEDED".to_string();
    previous.superseded_by = Some(next.plan_id.clone());
    previous.finished_at = Some(now.clone());
    previous.updated_at = now;
    if let Err(error) = design_intent_plan::persist_record(root, &previous) {
        let _ = design_intent_plan::remove_record(root, &next.plan_id);
        return Err(error);
    }
    Ok(json!({
        "schema":"elon.ui-design-intent-plan.v1",
        "action":"REPLANNED",
        "previousPlan":previous,
        "plan":next,
        "sourceModified":false,
        "runtimeStarted":false,
    }))
}

fn load_expected(root: &std::path::Path, arguments: &Value) -> Result<DesignIntentPlan> {
    let plan_id = required_text(arguments, "planId")?;
    let expected = arguments
        .get("expectedRevision")
        .and_then(Value::as_u64)
        .context("缺少 expectedRevision")?;
    let plan = design_intent_plan::read_plan(root, plan_id)?;
    if plan.revision != expected {
        bail!(
            "DESIGN_INTENT_REVISION_CONFLICT：expected={expected} actual={}",
            plan.revision
        );
    }
    Ok(plan)
}

fn validate_session_matches(
    root: &std::path::Path,
    plan: &DesignIntentPlan,
    design_session_id: &str,
) -> Result<()> {
    design_session_store::validate_design_session_id(design_session_id)?;
    let record = design_session_store::read_record(root, design_session_id)
        .context("DESIGN_INTENT_SESSION_MISSING：后台设计会话不存在")?;
    if plan.primary_platform.as_deref() != Some(record.platform.as_str()) {
        bail!("DESIGN_INTENT_PLATFORM_MISMATCH：会话平台与计划不一致");
    }
    if record.route != plan.route {
        bail!("DESIGN_INTENT_ROUTE_MISMATCH：会话 route 与计划不一致");
    }
    Ok(())
}

fn mark_binding_receipt(plan: &mut DesignIntentPlan, now: &str) {
    let Some(order) = plan
        .actions
        .iter()
        .find(|action| action.tool == "ui_bind_design_task")
        .map(|action| action.order)
    else {
        return;
    };
    if let Some(receipt) = plan
        .action_receipts
        .iter_mut()
        .find(|receipt| receipt.order == order)
    {
        receipt.status = "SUCCEEDED".to_string();
        receipt.attempt = receipt.attempt.max(1);
        receipt.summary = Some("task lease 已绑定到设计会话".to_string());
        receipt.updated_at = now.to_string();
    }
}

fn validate_receipt_transition(current: &str, next: &str) -> Result<()> {
    if matches!(current, "SUCCEEDED" | "SKIPPED") {
        bail!("DESIGN_INTENT_ACTION_SETTLED：已完成动作不能覆盖");
    }
    let allowed = matches!(next, "RUNNING" | "SUCCEEDED" | "FAILED" | "SKIPPED");
    if !allowed || (current == "PENDING" && next == "FAILED") {
        bail!("DESIGN_INTENT_ACTION_STATE_CONFLICT：{current} 不允许 {next}");
    }
    Ok(())
}

fn all_actions_settled(plan: &DesignIntentPlan) -> bool {
    !plan.action_receipts.is_empty()
        && plan
            .action_receipts
            .iter()
            .all(|receipt| matches!(receipt.status.as_str(), "SUCCEEDED" | "SKIPPED"))
}

fn is_terminal(status: &str) -> bool {
    matches!(status, "COMPLETED" | "CANCELED" | "SUPERSEDED")
}

fn settle_binding(session: &LiveUiSession, plan: &DesignIntentPlan) -> Value {
    let (Some(task_id), Some(lease_id)) = (&plan.task_id, &plan.task_lease_id) else {
        return json!({"status":"NOT_BOUND"});
    };
    let succeeded = plan.status == "COMPLETED";
    match super::design_task_binding::call(
        session,
        "ui_settle_design_task_binding",
        json!({"taskId":task_id,"leaseId":lease_id,"succeeded":succeeded}),
    ) {
        Ok(result) => json!({"status":"SETTLED","result":result}),
        Err(error) => json!({"status":"DEFERRED","reason":clean(&error.to_string(), 240)}),
    }
}

fn evidence_refs(arguments: &Value) -> Result<Vec<String>> {
    let values = arguments
        .get("evidenceRefs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if values.len() > 8 {
        bail!("evidenceRefs 最多 8 项");
    }
    values
        .into_iter()
        .map(|value| {
            let value = value
                .as_str()
                .ok_or_else(|| anyhow!("evidenceRefs 只能包含字符串"))?
                .trim();
            if value.is_empty() || value.len() > 512 || value.contains('\0') {
                bail!("evidenceRefs 包含空值、NUL 或超过 512 字节");
            }
            Ok(value.to_string())
        })
        .collect()
}

fn inherit_text(arguments: &mut Value, key: &str, fallback: Option<&str>) {
    if arguments.get(key).and_then(Value::as_str).is_none() {
        if let Some(value) = fallback {
            arguments[key] = json!(value);
        }
    }
}

fn plan_result(action: &str, plan: DesignIntentPlan, binding: Option<Value>) -> Value {
    json!({
        "schema":"elon.ui-design-intent-plan.v1",
        "action":action,
        "plan":plan,
        "taskBinding":binding,
        "sourceModified":false,
        "runtimeStarted":false,
    })
}

fn clean(value: &str, max: usize) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| *ch != '\0')
        .take(max)
        .collect()
}

fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    optional_text(value, key).ok_or_else(|| anyhow!("缺少 {key}"))
}

fn optional_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema,"annotations":{
        "readOnlyHint":read_only,"destructiveHint":false,"idempotentHint":false,"openWorldHint":false}})
}

fn plan_reference_schema() -> Value {
    json!({
        "planId":{"type":"string","pattern":"^intent_[a-f0-9]{32}$"},
        "expectedRevision":{"type":"integer","minimum":1}
    })
}

fn start_schema() -> Value {
    let mut properties = plan_reference_schema();
    properties["taskId"] =
        json!({"type":"string","minLength":1,"maxLength":160,"pattern":"^[A-Za-z0-9._:-]+$"});
    properties["designSessionId"] = json!({"type":"string","pattern":"^design_[a-f0-9]{32}$"});
    properties["draftId"] = json!({"type":"string","pattern":"^draft_[a-f0-9]{32}$"});
    properties["expectedLeaseId"] = json!({"type":"string","pattern":"^lease_[a-f0-9]{32}$"});
    properties["leaseSeconds"] = json!({"type":"integer","minimum":60,"maximum":3600});
    json!({"type":"object","additionalProperties":false,"required":["planId","expectedRevision","taskId"],"properties":properties})
}

fn transition_schema() -> Value {
    let mut properties = plan_reference_schema();
    properties["transition"] = json!({"enum":["PAUSE","RESUME","CANCEL","FAIL","COMPLETE"]});
    properties["reason"] = json!({"type":"string","maxLength":500});
    json!({"type":"object","additionalProperties":false,"required":["planId","expectedRevision","transition"],"properties":properties})
}

fn action_schema() -> Value {
    let mut properties = plan_reference_schema();
    properties["actionOrder"] = json!({"type":"integer","minimum":1,"maximum":64});
    properties["status"] = json!({"enum":["RUNNING","SUCCEEDED","FAILED","SKIPPED"]});
    properties["summary"] = json!({"type":"string","maxLength":500});
    properties["errorCode"] = json!({"type":"string","maxLength":80});
    properties["evidenceRefs"] = json!({"type":"array","maxItems":8,"items":{"type":"string","minLength":1,"maxLength":512}});
    json!({"type":"object","additionalProperties":false,"required":["planId","expectedRevision","actionOrder","status"],"properties":properties})
}

fn replan_schema() -> Value {
    let mut properties = plan_reference_schema();
    properties["intent"] = json!({"type":"string","minLength":1,"maxLength":4000});
    properties["taskId"] =
        json!({"type":"string","minLength":1,"maxLength":160,"pattern":"^[A-Za-z0-9._:-]+$"});
    properties["platform"] = json!({"enum":["web","pwa","tauri","android"]});
    properties["route"] = json!({"type":"string","minLength":1,"maxLength":2048});
    properties["state"] = json!({"type":"string","maxLength":240});
    properties["designSessionId"] = json!({"type":"string","pattern":"^design_[a-f0-9]{32}$"});
    json!({"type":"object","additionalProperties":false,"required":["planId","expectedRevision","intent"],"properties":properties})
}
