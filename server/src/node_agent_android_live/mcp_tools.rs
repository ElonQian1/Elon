use serde_json::{json, Value};

pub(crate) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "ui_confirm_route",
            "模糊任务的第一步：确认本轮是 UI_DESIGN 还是 NON_UI。只提交判断和理由，不读取源码、不修改文件。",
            json!({
                "type":"object",
                "required":["route","reason"],
                "properties":{
                    "route":{"enum":["UI_DESIGN","NON_UI"]},
                    "reason":{"type":"string","maxLength":500},
                    "confidence":{"type":"number","minimum":0,"maximum":1}
                }
            }),
        ),
        tool(
            "ui_get_project_profile",
            "读取节点预生成的项目 UI 技术栈、主题、组件、导航和 Preview 候选；全新页面任务首先调用。",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "ui_get_design_task",
            "读取结构化设计任务、本地附件清单和标注；不需要重新解析用户长提示。",
            json!({"type":"object","properties":{"taskId":{"type":"string"}}}),
        ),
        tool(
            "ui_get_runtime_status",
            "查看当前处于无 Runtime 的 BOOTSTRAP 阶段还是已连接真实 Android Renderer 的 LIVE 阶段。",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "ui_list_render_devices",
            "列出远程 PC 节点当前可用于真实 Android Renderer 的设备和模拟器，并给出推荐设备。",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "ui_prepare_debug_runtime",
            "首次页面骨架编译完成后，构建并安装带 Debug Runtime 的 APK，自动优先选择模拟器并把项目 MCP 升级到 LIVE。",
            json!({
                "type":"object",
                "properties":{
                    "basePackageName":{"type":"string","description":"可选；默认使用项目 UI Profile 预提取的 applicationId"},
                    "deviceId":{"type":"string","description":"可选；不填时优先选择在线模拟器"},
                    "debugApplicationIdSuffix":{"type":"string","default":".uitest"}
                }
            }),
        ),
        tool(
            "ui_bind_target_design",
            "把已校验的 TARGET_DESIGN 手机附件绑定为视觉 Diff/FitRun 目标。标注修改图和风格参考图会被拒绝。",
            json!({
                "type":"object",
                "properties":{
                    "taskId":{"type":"string"},
                    "attachmentId":{"type":"string"}
                }
            }),
        ),
        tool(
            "ui_map_annotations_to_nodes",
            "把手机标注框的归一化坐标映射到当前 Runtime 节点，返回前三候选与置信度；不会把标注层当成目标像素。",
            json!({
                "type":"object",
                "properties":{"taskId":{"type":"string"}}
            }),
        ),
        tool(
            "ui_create_compose_screen_scaffold",
            "在已确认 Compose 的项目中创建不会覆盖现有文件的最小 Screen + Preview 骨架。创建后仍需按项目组件和主题补全并编译。",
            json!({
                "type":"object",
                "required":["screenName","screenId"],
                "properties":{
                    "relativeFile":{"type":"string","description":"可选；默认根据 UI Profile 的 app 模块和 namespace 生成"},
                    "packageName":{"type":"string","description":"可选；默认使用 UI Profile 的 Android namespace"},
                    "screenName":{"type":"string"},
                    "screenId":{"type":"string"}
                }
            }),
        ),
        tool(
            "ui_get_screen_summary",
            "读取紧凑页面摘要；每个任务应先调用。",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "ui_get_node",
            "按 runtimeNodeId 或 definitionId 读取一个节点。",
            node_selector_schema(),
        ),
        tool(
            "ui_get_subtree",
            "读取一个节点及其后代，不返回整屏无关节点。",
            node_selector_schema(),
        ),
        tool(
            "ui_get_source_bundle",
            "读取节点相关的少量源码片段与 source candidates。",
            node_selector_schema(),
        ),
        tool(
            "ui_get_target_crop",
            "返回目标设计图本地路径、哈希和指定区域。",
            rect_schema(),
        ),
        tool(
            "ui_get_current_crop",
            "立即 ADB 捕获最新真机画面，真实裁剪后返回本地路径、哈希和尺寸。",
            rect_schema(),
        ),
        tool(
            "ui_get_visual_diff",
            "本地计算目标图与真机截图的颜色、边缘、几何损失。",
            json!({
                "type":"object","properties":{
                    "targetRect":rect_value_schema(),
                    "currentRect":rect_value_schema(),
                    "projectedCurrentRect":rect_value_schema()
                }
            }),
        ),
        tool(
            "ui_propose_live_patch",
            "把校准后的设备目标几何映射为类型化 LIVE Patch，不修改真机。",
            json!({
                "type":"object","required":["runtimeNodeId","projectedCurrentRect"],
                "properties":{"runtimeNodeId":{"type":"string"},"projectedCurrentRect":rect_value_schema()}
            }),
        ),
        tool(
            "ui_apply_live_patch",
            "校验并应用类型化 LIVE Patch。",
            json!({
                "type":"object","required":["patch"],"properties":{"patch":{"type":"object"}}
            }),
        ),
        tool(
            "ui_run_visual_solver",
            "在本机进行有限次 Patch→截图→比较，自动保留更优参数，不消耗模型 Token。",
            json!({
                "type":"object","required":["runtimeNodeId","targetRect","projectedCurrentRect"],
                "properties":{
                    "runtimeNodeId":{"type":"string"},"targetRect":rect_value_schema(),
                    "projectedCurrentRect":rect_value_schema(),
                    "properties":{"type":"array","items":{"type":"string"}},
                    "maxEvaluations":{"type":"integer","minimum":1,"maximum":24},
                    "initialStepDp":{"type":"number","minimum":0.25,"maximum":32}
                }
            }),
        ),
        tool(
            "ui_start_fit_run",
            "创建并立即启动可恢复的自动拟合任务。它会持久化每次试验、预算、最佳结果和学习案例；达到候选后再确认写回源码。",
            json!({
                "type":"object",
                "required":["runtimeNodeId","targetRect","projectedTargetRect"],
                "properties":{
                    "runtimeNodeId":{"type":"string"},
                    "targetRect":rect_value_schema(),
                    "projectedTargetRect":rect_value_schema(),
                    "properties":{"type":"array","items":{"type":"string"},"maxItems":64},
                    "environment":{"type":"object"}
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
            "控制持久化拟合任务。CANDIDATE_READY 使用 ACCEPT_BEST；AWAITING_CODEX 时按 handoff 完成小范围源码修改后报告 CODEX_COMPLETED。",
            json!({
                "type":"object",
                "required":["runId","action"],
                "properties":{
                    "runId":{"type":"string"},
                    "action":{"enum":["START","PAUSE","RESUME","CANCEL","ACCEPT_BEST","CODEX_STARTED","CODEX_COMPLETED","CODEX_FAILED"]},
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
        tool(
            "ui_get_commit_plan",
            "分析当前 LIVE 修改的确定性写回与 Codex 回退项。",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "ui_commit_bound_styles",
            "按 sourceRevision 确定性写回绑定 XML/资源。",
            json!({
                "type":"object","required":["sourceRevision"],"properties":{"sourceRevision":{"type":"string"}}
            }),
        ),
        tool(
            "ui_build_and_verify",
            "请求构建、安装、清 Patch、回页和真机验收。",
            json!({
                "type":"object",
                "properties":{
                    "debugApplicationIdSuffix":{
                        "type":"string",
                        "pattern":"^\\.[A-Za-z0-9._]{1,39}$",
                        "description":"可选；仅用于并行安装 Debug 验收包，例如 .uitest"
                    },
                    "preview":{
                        "type":"object",
                        "required":["screenId","scenario","theme","fontScale","locale"],
                        "properties":{
                            "screenId":{"type":"string"},
                            "scenario":{"enum":["normal","loading","empty","error"]},
                            "theme":{"enum":["system","light","dark"]},
                            "fontScale":{"type":"number","minimum":0.5,"maximum":2.0},
                            "locale":{"type":"string"}
                        }
                    }
                }
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    let read_only = matches!(
        name,
        "ui_confirm_route"
            | "ui_get_project_profile"
            | "ui_get_design_task"
            | "ui_get_runtime_status"
            | "ui_list_render_devices"
            | "ui_get_screen_summary"
            | "ui_get_node"
            | "ui_get_subtree"
            | "ui_get_source_bundle"
            | "ui_get_target_crop"
            | "ui_get_current_crop"
            | "ui_get_visual_diff"
            | "ui_propose_live_patch"
            | "ui_get_fit_run"
            | "ui_get_commit_plan"
    );
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
        "annotations": {
            "readOnlyHint": read_only,
            "destructiveHint": false,
            "idempotentHint": read_only,
            "openWorldHint": false
        }
    })
}

#[cfg(test)]
mod annotation_tests {
    use super::tool_definitions;

    #[test]
    fn status_and_route_tools_are_declared_read_only() {
        let tools = tool_definitions();
        for name in [
            "ui_confirm_route",
            "ui_get_design_task",
            "ui_get_project_profile",
            "ui_get_runtime_status",
            "ui_list_render_devices",
        ] {
            let tool = tools
                .iter()
                .find(|tool| tool["name"] == name)
                .expect("read-only UI tool");
            assert_eq!(tool["annotations"]["readOnlyHint"], true);
        }
        let apply = tools
            .iter()
            .find(|tool| tool["name"] == "ui_apply_live_patch")
            .expect("live patch tool");
        assert_eq!(apply["annotations"]["readOnlyHint"], false);
    }
}

fn node_selector_schema() -> Value {
    json!({
        "type":"object",
        "properties":{"runtimeNodeId":{"type":"string"},"definitionId":{"type":"string"}}
    })
}

fn rect_schema() -> Value {
    json!({"type":"object","properties":{"rect":rect_value_schema()}})
}

fn rect_value_schema() -> Value {
    json!({
        "type":"object","required":["left","top","right","bottom"],
        "properties":{
            "left":{"type":"integer"},"top":{"type":"integer"},
            "right":{"type":"integer"},"bottom":{"type":"integer"}
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_runtime_uses_profile_application_id_by_default() {
        let tool = tool_definitions()
            .into_iter()
            .find(|tool| tool["name"] == "ui_prepare_debug_runtime")
            .expect("tool should exist");
        assert!(tool["inputSchema"].get("required").is_none());
        assert!(
            tool["inputSchema"]["properties"]["basePackageName"]["description"]
                .as_str()
                .unwrap()
                .contains("UI Profile")
        );
    }
}
