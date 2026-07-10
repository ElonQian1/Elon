use serde_json::{json, Value};

pub(crate) fn tool_definitions() -> Vec<Value> {
    vec![
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
            "返回当前真机截图本地路径、哈希和指定区域。",
            rect_schema(),
        ),
        tool(
            "ui_get_visual_diff",
            "本地计算目标图与真机截图的颜色、边缘、几何损失。",
            json!({
                "type":"object","properties":{"targetRect":rect_value_schema(),"currentRect":rect_value_schema()}
            }),
        ),
        tool(
            "ui_propose_live_patch",
            "把目标几何映射为类型化 LIVE Patch，不修改真机。",
            json!({
                "type":"object","required":["runtimeNodeId","targetRect"],
                "properties":{"runtimeNodeId":{"type":"string"},"targetRect":rect_value_schema()}
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
                "type":"object","required":["runtimeNodeId","targetRect"],
                "properties":{
                    "runtimeNodeId":{"type":"string"},"targetRect":rect_value_schema(),
                    "properties":{"type":"array","items":{"type":"string"}},
                    "maxEvaluations":{"type":"integer","minimum":1,"maximum":24},
                    "initialStepDp":{"type":"number","minimum":0.25,"maximum":32}
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
    json!({ "name": name, "description": description, "inputSchema": input_schema })
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
