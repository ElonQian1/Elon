use serde_json::{json, Value};

use super::{
    fit_environment_schema, fit_visual_mask_schema, rect_value_schema,
    stable_selector_value_schema, tool,
};

pub(super) fn definitions() -> Vec<Value> {
    vec![
        tool(
            "ui_start_fit_run",
            "创建并立即启动可恢复的自动拟合任务。它会持久化每次试验、预算、最佳结果和学习案例；达到候选后再确认写回源码。",
            json!({
                "type":"object",
                "required":["targetRect","projectedTargetRect"],
                "anyOf":[{"required":["runtimeNodeId"]},{"required":["selector"]}],
                "properties":{
                    "taskId":{"type":"string","maxLength":128,"description":"可选；默认绑定当前结构化 UI 任务"},
                    "runtimeNodeId":{"type":"string"},
                    "selector":stable_selector_value_schema(),
                    "targetRect":rect_value_schema(),
                    "projectedTargetRect":rect_value_schema(),
                    "properties":{"type":"array","items":{"type":"string"},"maxItems":64},
                    "environment":fit_environment_schema::fit_environment_schema(),
                    "visualMask":fit_visual_mask_schema()
                }
            }),
        ),
        tool(
            "ui_get_fit_run",
            "读取一个持久化拟合任务；不传 runId 时列出当前项目最近任务。",
            json!({
                "type":"object",
                "properties":{"runId":{"type":"string"}}
            }),
        ),
        tool(
            "ui_control_fit_run",
            "控制持久化拟合任务。ATTACH_STATE_REPLAY 仅按调用方提交的明确步骤为既有非终态 run 原子绑定页面重放，不根据页面名称猜测动作；CANDIDATE_READY 使用 ACCEPT_BEST。",
            json!({
                "type":"object",
                "required":["runId","action"],
                "allOf":[{
                    "if":{"properties":{"action":{"const":"ATTACH_STATE_REPLAY"}}},
                    "then":{"required":["projectRoot","scenario","stateReplay","targetRuntimeNodeId","targetDefinitionId"]}
                }],
                "properties":{
                    "runId":{"type":"string"},
                    "action":{"enum":["START","PAUSE","RESUME","CANCEL","REBIND_SESSION","ATTACH_STATE_REPLAY","ACCEPT_BEST","CODEX_STARTED","CODEX_COMPLETED","CODEX_FAILED"]},
                    "projectRoot":{"type":"string","minLength":1,"maxLength":4096},
                    "scenario":{"type":"string","minLength":1,"maxLength":128},
                    "stateReplay":fit_environment_schema::state_replay_schema(),
                    "targetRuntimeNodeId":{"type":"string","minLength":1,"maxLength":500},
                    "targetDefinitionId":{"type":"string","minLength":1,"maxLength":500},
                    "targetInstanceKey":{"type":"string","maxLength":500},
                    "newSessionId":{"type":"string","minLength":1,"maxLength":256},
                    "newRuntimeNodeId":{"type":"string","minLength":1,"maxLength":256},
                    "newCurrentRect":rect_value_schema(),
                    "handoffId":{"type":"string"},
                    "taskId":{"type":"string"},
                    "sourceRevisionBefore":{"type":"string"},
                    "sourceRevisionAfter":{"type":"string"},
                    "changedFiles":{"type":"array","items":{"type":"string"}},
                    "commitId":{"type":"string"},
                    "tokenUsage":{"type":"integer","minimum":0},
                    "error":{"type":"string"}
                }
            }),
        ),
    ]
}
