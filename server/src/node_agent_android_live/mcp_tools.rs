use serde_json::{json, Value};

pub(crate) fn tool_definitions() -> Vec<Value> {
    vec![
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
            "ui_create_compose_screen_scaffold",
            "在已确认 Compose 的项目中创建不会覆盖现有文件的最小 Screen + Preview 骨架。创建后仍需按项目组件和主题补全并编译。",
            json!({
                "type":"object",
                "required":["relativeFile","packageName","screenName","screenId"],
                "properties":{
                    "relativeFile":{"type":"string","description":"模块 src/main 或 src/debug 下的 .kt 相对路径"},
                    "packageName":{"type":"string"},
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
