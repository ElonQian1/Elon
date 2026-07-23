use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

use super::fit_run::FitCommand;

pub(super) fn fit_command(action: &str, arguments: &Value) -> Result<FitCommand> {
    let command_id = format!("mcp_{}", uuid::Uuid::new_v4().simple());
    let required = |key: &str| -> Result<String> {
        arguments
            .get(key)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow!("{action} 缺少 {key}"))
    };
    let value = match action.trim().to_ascii_uppercase().as_str() {
        "START" | "PAUSE" | "RESUME" | "CANCEL" | "ACCEPT_BEST" => json!({
            "type": action.trim().to_ascii_uppercase(),
            "commandId": command_id,
        }),
        "REBIND_SESSION" => json!({
            "type":"REBIND_SESSION", "commandId":command_id,
            "newSessionId":required("newSessionId")?,
            "newRuntimeNodeId":arguments.get("newRuntimeNodeId").cloned(),
            "newCurrentRect":arguments.get("newCurrentRect").cloned(),
        }),
        "CODEX_STARTED" => json!({
            "type":"CODEX_STARTED", "commandId":command_id,
            "handoffId":required("handoffId")?, "taskId":required("taskId")?,
        }),
        "CODEX_COMPLETED" => json!({
            "type":"CODEX_COMPLETED", "commandId":command_id,
            "handoffId":required("handoffId")?,
            "taskId":arguments.get("taskId").cloned(),
            "sourceRevisionBefore":arguments.get("sourceRevisionBefore").cloned(),
            "sourceRevisionAfter":required("sourceRevisionAfter")?,
            "changedFiles":arguments.get("changedFiles").cloned().unwrap_or_else(|| json!([])),
            "commitId":arguments.get("commitId").cloned(),
            "tokenUsage":arguments.get("tokenUsage").cloned(),
        }),
        "CODEX_FAILED" => json!({
            "type":"CODEX_FAILED", "commandId":command_id,
            "handoffId":required("handoffId")?, "error":required("error")?,
        }),
        _ => bail!("不支持 FitRun action: {action}"),
    };
    serde_json::from_value(value).context("FitRun 控制命令无效")
}
