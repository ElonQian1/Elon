use serde_json::{json, Value};

mod fit_environment_schema;
mod fit_run_tools;
pub(crate) fn tool_definitions() -> Vec<Value> {
    let mut definitions = vec![
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
            "ui_import_desktop_task",
            "Codex 桌面端 UI 任务的第一步：把本轮草图/标注图复制进项目 UI 工作区，生成共享任务、附件哈希和项目 UI Profile，使 PC 网页端和节点读取同一工件。",
            json!({
                "type":"object",
                "required":["request"],
                "properties":{
                    "request":{"type":"string","minLength":1,"maxLength":20000},
                    "taskId":{"type":"string","maxLength":96},
                    "mode":{"enum":["AUTO","MODIFY_EXISTING","EXTEND_EXISTING","CREATE_NEW"]},
                    "attachmentIntent":{"enum":["AUTO","TARGET_DESIGN","ANNOTATED_CHANGE_REQUEST","REFERENCE_STYLE","CURRENT_SCREENSHOT"]},
                    "attachments":{
                        "type":"array","maxItems":64,
                        "items":{
                            "type":"object","required":["path"],
                            "properties":{
                                "path":{"type":"string"},
                                "displayName":{"type":"string","maxLength":240},
                                "intent":{"enum":["AUTO","TARGET_DESIGN","ANNOTATED_CHANGE_REQUEST","REFERENCE_STYLE","CURRENT_SCREENSHOT"]}
                            }
                        }
                    }
                }
            }),
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
            "分阶段准备 Debug Runtime：优先复用与当前源码一致的 APK 和已有会话，后台执行构建、设备校验、安装、启动、端口映射与 Runtime 握手。返回 IN_PROGRESS 时按 retryAfterMs 重复调用读取进度；FAILED 会包含具体阶段和 ADB 证据，修正环境后可传 restart=true 有界重试。",
            json!({
                "type":"object",
                "properties":{
                    "basePackageName":{"type":"string","description":"可选；默认使用项目 UI Profile 预提取的 applicationId"},
                    "deviceId":{"type":"string","description":"可选；不填时优先选择在线模拟器"},
                    "autoStartEmulator":{"type":"boolean","default":true,"description":"没有在线设备时自动启动 ELON_ANDROID_AVD 或排序后的首个 AVD"},
                    "fallbackToEmulator":{"type":"boolean","default":true,"description":"显式真机不在线时回退到模拟器，并在结果中标记 deviceSelection.source"},
                    "debugApplicationIdSuffix":{"type":"string","default":".uitest"},
                    "isolatedEmulatorPackage":{"type":"boolean","default":false,"description":"仅模拟器可显式启用；真机一律使用节点固定 .uituner_<指纹> 包"}, "lkgEnabled":{"type":"boolean","default":false,"description":"本次调试任务显式启用最近成功版本；默认关闭，不参与构建、安装或收尾门禁"},
                    "candidate":super::debug_integration_contract::debug_candidate_schema(),
                    "restart":{"type":"boolean","default":false,"description":"仅在上一轮 FAILED 或源码变化后显式启动新一轮；运行中重复调用不创建新会话"}
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
            "ui_create_android_screen_scaffold",
            "为全新 Android 页面按项目 UI Profile 生成不覆盖现有文件的真实工程骨架。纯 View/XML 或纯 Compose 自动选择；混合项目必须显式选择，创建后交给 Codex 补业务结构，再构建进入真实 Renderer。",
            json!({
                "type":"object",
                "required":["screenName","screenId"],
                "properties":{
                    "uiToolkit":{"enum":["COMPOSE","VIEWS"],"description":"混合项目必填；纯技术栈项目可省略"},
                    "relativeFile":{"type":"string","description":"可选；默认根据 UI Profile 的 app 模块生成"},
                    "packageName":{"type":"string","description":"Compose 可选；默认使用 Android namespace"},
                    "layoutName":{"type":"string","description":"View/XML 可选；必须是小写 Android resource name"},
                    "screenName":{"type":"string"},
                    "screenId":{"type":"string"}
                }
            }),
        ),
        tool(
            "ui_create_compose_screen_scaffold",
            "兼容旧客户端的 Compose 专用页面骨架工具；新任务优先使用 ui_create_android_screen_scaffold。",
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
            "ui_trace_window_insets_sequence",
            "在真实 Android Renderer 上重放受限页面序列，并在每个状态采集 Window Insets 与指定 UiAutomator 节点坐标；返回紧凑状态和相对首状态的差值。",
            json!({
                "type":"object",
                "required":["steps","selectors"],
                "properties":{
                    "deviceId":{"type":"string","description":"BOOTSTRAP 阶段必填；LIVE 阶段默认使用当前 Renderer 设备"},
                    "packageName":{"type":"string","description":"BOOTSTRAP 阶段必填；必须是设备上已安装的应用包名"},
                    "settleMs":{"type":"integer","minimum":100,"maximum":5000,"default":700},
                    "steps":{
                        "type":"array","minItems":1,"maxItems":16,
                        "items":{
                            "type":"object","required":["name","action"],
                            "properties":{
                                "name":{"type":"string","minLength":1,"maxLength":80},
                                "action":{
                                    "type":"object","required":["type"],
                                    "properties":{
                                        "type":{"enum":["LAUNCH","ACTIVATE_NODE","TAP","TAP_NODE","BACK","WAIT"]},
                                        "definitionId":{"type":"string","minLength":1,"maxLength":500},
                                        "instanceKey":{"type":"string","maxLength":500},
                                        "x":{"type":"integer","minimum":0},
                                        "y":{"type":"integer","minimum":0},
                                        "resourceIdSuffix":{"type":"string","maxLength":200},
                                        "text":{"type":"string","maxLength":500},
                                        "contentDescription":{"type":"string","maxLength":500},
                                        "occurrence":{"type":"integer","minimum":0,"maximum":50}
                                    }
                                }
                            }
                        }
                    },
                    "selectors":{
                        "type":"array","minItems":1,"maxItems":16,
                        "items":{
                            "type":"object","required":["label"],
                            "properties":{
                                "label":{"type":"string","minLength":1,"maxLength":80},
                                "resourceIdSuffix":{"type":"string","maxLength":200},
                                "text":{"type":"string","maxLength":500},
                                "contentDescription":{"type":"string","maxLength":500},
                                "occurrence":{"type":"integer","minimum":0,"maximum":50}
                            }
                        }
                    }
                }
            }),
        ),
        tool(
            "ui_trace_relational_layout_geometry",
            "在真实 Android Renderer 的多页面/多状态序列中稳定选择节点，计算节点、屏幕及安全内容区的边缘、中心、尺寸与间距关系，并执行带像素容差的可追溯断言。",
            json!({
                "type":"object",
                "required":["steps","selectors","assertions"],
                "properties":{
                    "deviceId":{"type":"string"},
                    "packageName":{"type":"string"},
                    "settleMs":{"type":"integer","minimum":100,"maximum":5000,"default":700},
                    "steps":{
                        "type":"array","minItems":1,"maxItems":16,
                        "items":{
                            "type":"object","required":["name","action"],
                            "properties":{
                                "name":{"type":"string","minLength":1,"maxLength":80},
                                "action":{
                                    "type":"object","required":["type"],
                                    "properties":{
                                        "type":{"enum":["LAUNCH","ACTIVATE_NODE","TAP","TAP_NODE","BACK","WAIT"]},
                                        "definitionId":{"type":"string","minLength":1,"maxLength":500},
                                        "instanceKey":{"type":"string","maxLength":500},
                                        "x":{"type":"integer","minimum":0},
                                        "y":{"type":"integer","minimum":0},
                                        "resourceIdSuffix":{"type":"string","maxLength":200},
                                        "text":{"type":"string","maxLength":500},
                                        "contentDescription":{"type":"string","maxLength":500},
                                        "occurrence":{"type":"integer","minimum":0,"maximum":50}
                                    }
                                }
                            }
                        }
                    },
                    "selectors":{
                        "type":"array","minItems":1,"maxItems":16,
                        "items":{
                            "type":"object","required":["label"],
                            "properties":{
                                "label":{"type":"string","minLength":1,"maxLength":80},
                                "resourceIdSuffix":{"type":"string","maxLength":200},
                                "text":{"type":"string","maxLength":500},
                                "contentDescription":{"type":"string","maxLength":500},
                                "occurrence":{"type":"integer","minimum":0,"maximum":50}
                            }
                        }
                    },
                    "assertions":{
                        "type":"array","minItems":1,"maxItems":32,
                        "items":{
                            "type":"object",
                            "required":["name","left","right","tolerancePx"],
                            "properties":{
                                "name":{"type":"string","minLength":1,"maxLength":120},
                                "left":geometry_operand_schema(),
                                "right":geometry_operand_schema(),
                                "expectedDeltaPx":{"type":"integer","default":0},
                                "tolerancePx":{"type":"integer","minimum":0,"maximum":10000}
                            }
                        }
                    }
                }
            }),
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
            "ui_capture_android_launcher_surface",
            "默认由已连接 Runtime 使用 LauncherApps/PackageManager 直接读取 packageName 图标并产出稳定 iconRect/iconCrop，不切 HOME、不逐页模拟；OEM_FIXED_POSITION 仅用于固定位置的 MIUI 最终呈现复验，LEGACY_BOUNDED_SEARCH 是显式降级。",
            json!({
                "type":"object","additionalProperties":false,"properties":{
                    "deviceId":{"type":"string","minLength":1,"maxLength":128},
                    "packageName":{"type":"string","minLength":1,"maxLength":180},
                    "mode":{"enum":["PACKAGE_ICON","OEM_FIXED_POSITION","LEGACY_BOUNDED_SEARCH"],"default":"PACKAGE_ICON"},
                    "appLabel":{"type":"string","minLength":1,"maxLength":180},
                    "iconRect":rect_value_schema(),
                    "iconSizePx":{"type":"integer","minimum":48,"maximum":1024,"default":512},
                    "settleMs":{"type":"integer","minimum":200,"maximum":5000,"default":900},
                    "maxPages":{"type":"integer","minimum":1,"maximum":32,"default":24}
                }
            }),
        ),
        tool(
            "ui_render_android_launcher_masks",
            "对同一 PackageManager/LauncherApps iconCrop 生成 CIRCLE、ROUNDED_SQUARE、SQUIRCLE 三种真实 PNG mask 产物，并返回各自与原图的 diff/score。",
            json!({
                "type":"object","additionalProperties":false,"required":["currentArtifact"],"properties":{
                    "currentArtifact":launcher_artifact_schema(),
                    "shapes":{"type":"array","minItems":1,"maxItems":3,"uniqueItems":true,"items":{"enum":["CIRCLE","ROUNDED_SQUARE","SQUIRCLE"]}},
                    "safeZoneInsetFraction":{"type":"number","minimum":0,"maximum":0.25,"default":0}
                }
            }),
        ),
        tool(
            "ui_get_visual_diff",
            "本地计算目标图与真机截图的颜色、边缘、几何损失；支持复用 Launcher iconCrop 及 Android adaptive icon 系统 mask/crop。",
            json!({
                "type":"object","additionalProperties":false,"properties":{
                    "currentArtifact":{"type":"object",
                        "required":["source","path","sha256"],"properties":{
                            "source":{"const":"ANDROID_LAUNCHER"},
                            "path":{"type":"string","minLength":1,"maxLength":4000},
                            "sha256":{"type":"string","pattern":"^[A-Fa-f0-9]{64}$"}
                        }
                    },
                    "targetRect":rect_value_schema(),
                    "currentRect":rect_value_schema(),
                    "projectedCurrentRect":rect_value_schema()
                    ,"mask":{"type":"object","additionalProperties":false,"properties":{
                        "excludeRects":{"type":"array","maxItems":24,"items":rect_value_schema()},
                        "adaptiveIconMask":{"type":"object","additionalProperties":false,"required":["shape"],"properties":{
                            "shape":{"enum":["CIRCLE","ROUNDED_SQUARE","SQUIRCLE"]},
                            "safeZoneInsetFraction":{"type":"number","minimum":0,"maximum":0.25,"default":0}
                        }}
                    }}
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
                    ,"visualMask":{"type":"object","properties":{"excludeRects":{"type":"array","maxItems":24,"items":rect_value_schema()}}}
                }
            }),
        ),
        tool(
            "ui_check_capabilities",
            "在编辑源码前检查当前一龙 UI 平台能否完成任务；系统会从结构化任务和项目规则推导必需能力，requiredCapabilities 只能追加而不能削弱。",
            json!({
                "type":"object",
                "properties":{
                    "taskId":{"type":"string","maxLength":128},
                    "requiredCapabilities":{
                        "type":"array","minItems":1,"maxItems":32,
                        "items":{"type":"string","maxLength":80}
                    }
                }
            }),
        ),
        tool(
            "ui_check_workflow_completion",
            "UI 任务收尾前的强制门禁：completionReady 表示平台全闭环；businessDeliveryReady 表示业务 UI 已通过并可在非阻塞平台进化分流后先行交付。",
            json!({
                "type":"object",
                "properties":{"taskId":{"type":"string","maxLength":128}}
            }),
        ),
        tool(
            "ui_write_cross_platform_verification",
            "绑定当前 Git revision 原子生成跨端验收工件。VISUAL_PARITY 必须使用独立真实 Android/Web 截图；NO_WEB_COUNTERPART 必须使用可复核的 Android 来源文件、Web 跟踪源码根和搜索词证明仓库中没有对应功能，禁止伪造 Web 截图。",
            super::cross_platform_verification::tool_input_schema(),
        ),
        tool(
            "ui_report_capability_gap",
            "确认 PC UI 平台自身缺少能力后创建分流工件。业务任务只产出 Codex Desktop Worktree handoff；独立 EVOLUTION_THREAD 才执行升级与发布。",
            json!({
                "type":"object",
                "required":["taskId","missingCapabilities","evidence","proposedChanges","resumeTarget"],
                "properties":{
                    "taskId":{"type":"string","maxLength":128},
                    "fitRunId":{"type":"string","maxLength":128},
                    "executionMode":{"enum":["BUSINESS_THREAD","EVOLUTION_THREAD"],"default":"BUSINESS_THREAD"},
                    "deliveryImpact":{"enum":["DELIVERY_BLOCKING","DELIVERY_NON_BLOCKING","EVOLUTION_ONLY"],"default":"DELIVERY_BLOCKING"},
                    "originGapId":{"type":"string","maxLength":128},
                    "originThreadId":{"type":"string","maxLength":128},
                    "missingCapabilities":{"type":"array","minItems":1,"maxItems":16,"items":{"type":"string","maxLength":80}},
                    "evidence":{"type":"array","minItems":1,"maxItems":32,"items":{"type":"string","maxLength":2000}},
                    "proposedChanges":{"type":"array","minItems":1,"maxItems":16,"items":{"type":"string","maxLength":2000}},
                    "resumeTarget":{"type":"string","minLength":1,"maxLength":2000},
                    "businessDelivery":{
                        "type":"object",
                        "required":["sourceRevision","sourceWritebackVerified","patchFreeBuildVerified","visualLoss","maxVisualLoss","sourceParityLoss","maxSourceParityLoss","reason"],
                        "properties":{
                            "sourceRevision":{"type":"string","minLength":1,"maxLength":256},
                            "sourceWritebackVerified":{"const":true},
                            "patchFreeBuildVerified":{"const":true},
                            "visualLoss":{"type":"number","minimum":0,"maximum":1},
                            "maxVisualLoss":{"type":"number","minimum":0,"maximum":1},
                            "sourceParityLoss":{"type":"number","minimum":0,"maximum":1},
                            "maxSourceParityLoss":{"type":"number","minimum":0,"maximum":1},
                            "reason":{"type":"string","minLength":1,"maxLength":2000}
                        }
                    }
                }
            }),
        ),
        tool(
            "ui_get_capability_gap",
            "读取当前项目的平台能力缺口、自动升级轮次、发布结果和原 UI 任务恢复点。",
            json!({
                "type":"object",
                "properties":{"gapId":{"type":"string","maxLength":128}}
            }),
        ),
        tool(
            "ui_control_capability_gap",
            "仅在独立 EVOLUTION_THREAD 中驱动可信本地 Git 工作区的平台升级、发布和复检；业务任务的 DEFERRED gap 不允许启动升级。",
            json!({
                "type":"object",
                "required":["gapId","action"],
                "properties":{
                    "gapId":{"type":"string","maxLength":128},
                    "action":{"enum":["START_UPGRADE","PUBLISH_COMPLETED","RECHECK_PASSED","RECHECK_FAILED","UPGRADE_FAILED","CANCEL"]},
                    "sourceRevisionBefore":{"type":"string","maxLength":256},
                    "sourceRevisionAfter":{"type":"string","maxLength":256},
                    "commitId":{"type":"string","maxLength":256},
                    "version":{"type":"string","maxLength":256},
                    "changedFiles":{"type":"array","minItems":1,"maxItems":128,"items":{"type":"string","maxLength":2000}},
                    "failureSignature":{"type":"string","maxLength":500},
                    "error":{"type":"string","maxLength":2000}
                    ,"originProjectRoot":{"type":"string","minLength":1,"maxLength":4000}
                }
            }),
        ),
        tool(
            "ui_start_capability_upgrade",
            "把已批准的平台能力缺口正式推进到 UPGRADING；必须记录升级前源码 revision。",
            json!({
                "type":"object",
                "required":["gapId","sourceRevisionBefore"],
                "properties":{
                    "gapId":{"type":"string","maxLength":128},
                    "sourceRevisionBefore":{"type":"string","maxLength":256}
                }
            }),
        ),
        tool(
            "ui_complete_capability_upgrade",
            "回报平台升级发布或复检结果，严格推进 PUBLISHED、RESUMED 或失败熔断状态。",
            json!({
                "type":"object",
                "required":["gapId","transition"],
                "properties":{
                    "gapId":{"type":"string","maxLength":128},
                    "transition":{"enum":["PUBLISH_COMPLETED","RECHECK_PASSED","RECHECK_FAILED","UPGRADE_FAILED","CANCEL"]},
                    "sourceRevisionAfter":{"type":"string","maxLength":256},
                    "commitId":{"type":"string","maxLength":256},
                    "version":{"type":"string","maxLength":256},
                    "changedFiles":{"type":"array","minItems":1,"maxItems":128,"items":{"type":"string","maxLength":2000}},
                    "failureSignature":{"type":"string","maxLength":500},
                    "error":{"type":"string","maxLength":2000}
                    ,"originProjectRoot":{"type":"string","minLength":1,"maxLength":4000}
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
            "ui_activate_preview_scenario",
            "无需重建 APK，正式激活已安装 Debug Runtime 注册的 Preview 场景；节点端由 UiRuntimePreviewRegistry.supportedScenarios 校验，并完成 PreviewHost 回页、Runtime 重连、节点树刷新与截图取证。",
            json!({
                "type":"object",
                "required":["screenId","scenario"],
                "properties":{
                    "screenId":{"type":"string","minLength":1,"maxLength":180},
                    "scenario":{"type":"string","minLength":1,"maxLength":80},
                    "theme":{"enum":["system","light","dark"],"default":"system"},
                    "fontScale":{"type":"number","minimum":0.5,"maximum":2.0,"default":1.0},
                    "locale":{"type":"string","minLength":1,"maxLength":40,"default":"zh-CN"}
                }
            }),
        ),
        tool(
            "ui_build_and_verify",
            "后台执行构建、安装、清 Patch、回页和真机验收。首次调用立即返回 operationId/IN_PROGRESS；后续只传 operationId 轮询安装、回页、节点树刷新和截图终态，客户端窗口结束不会取消后台操作。",
            json!({
                "type":"object",
                "properties":{
                    "operationId":{
                        "type":"string",
                        "pattern":"^ui_build_verify_[a-f0-9]{32}$",
                        "description":"轮询既有后台操作时只需传此字段"
                    },
                    "debugApplicationIdSuffix":{
                        "type":"string",
                        "pattern":"^\\.[A-Za-z0-9._]{1,39}$",
                        "description":"可选；仅用于并行安装 Debug 验收包，例如 .uitest"
                    },
                    "forceRerun":{"type":"boolean","default":false,"description":"默认复用 Gradle 增量和构建缓存；仅诊断缓存污染时显式启用 --rerun-tasks"},
                    "lkgEnabled":{"type":"boolean","default":false,"description":"本次构建验收显式启用最近成功版本；默认关闭"},
                    "preview":{
                        "type":"object",
                        "required":["screenId","scenario","theme","fontScale","locale"],
                        "properties":{
                            "screenId":{"type":"string"},
                            "scenario":{"type":"string","minLength":1,"maxLength":80},
                            "theme":{"enum":["system","light","dark"]},
                            "fontScale":{"type":"number","minimum":0.5,"maximum":2.0},
                            "locale":{"type":"string"}
                        }
                    }
                }
            }),
        ),
    ];
    definitions.push(tool("ui_get_debug_integration_status",
        "读取当前项目指定设备的固定调试槽、贡献提交、冲突、代次，以及最近成功版本是否显式启用。",
        json!({"type":"object","required":["deviceId"],"properties":{"deviceId":{"type":"string"},"projectId":{"type":"string"}}})));
    definitions.push(crate::node_agent_pwa_runtime::tool_definition());
    definitions.push(super::verification_workflow::tool_definition());
    definitions.extend(fit_run_tools::definitions());
    definitions
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    let read_only = matches!(
        name,
        "ui_confirm_route"
            | "ui_get_project_profile"
            | "ui_get_design_task"
            | "ui_get_runtime_status"
            | "ui_get_debug_integration_status"
            | "ui_list_render_devices"
            | "ui_get_screen_summary"
            | "ui_get_node"
            | "ui_get_subtree"
            | "ui_get_source_bundle"
            | "ui_get_target_crop"
            | "ui_get_current_crop"
            | "ui_capture_android_launcher_surface"
            | "ui_render_android_launcher_masks"
            | "ui_get_visual_diff"
            | "ui_propose_live_patch"
            | "ui_get_fit_run"
            | "ui_check_capabilities"
            | "ui_check_workflow_completion"
            | "ui_get_capability_gap"
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

fn geometry_operand_schema() -> Value {
    json!({
        "type":"object",
        "required":["step","source","anchor"],
        "properties":{
            "step":{"type":"string","minLength":1,"maxLength":80},
            "source":{"enum":["NODE","DISPLAY","SAFE_CONTENT"]},
            "selector":{"type":"string","minLength":1,"maxLength":80},
            "anchor":{"enum":["LEFT","TOP","RIGHT","BOTTOM","CENTER_X","CENTER_Y","WIDTH","HEIGHT"]}
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
            "ui_get_debug_integration_status",
            "ui_list_render_devices",
            "ui_check_workflow_completion",
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

    #[test]
    fn capability_gap_schema_exposes_nonblocking_business_handoff_evidence() {
        let tools = tool_definitions();
        let tool = tools
            .iter()
            .find(|tool| tool["name"] == "ui_report_capability_gap")
            .expect("capability gap tool");
        let schema = &tool["inputSchema"];
        assert_eq!(
            schema["properties"]["executionMode"]["enum"][1],
            "EVOLUTION_THREAD"
        );
        assert_eq!(
            schema["properties"]["deliveryImpact"]["enum"][1],
            "DELIVERY_NON_BLOCKING"
        );
        assert!(schema["properties"]["businessDelivery"]["required"]
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value == "sourceRevision")));
    }

    #[test]
    fn fit_run_control_schema_exposes_rebind_and_state_replay_attachment() {
        let tools = tool_definitions();
        let tool = tools
            .iter()
            .find(|tool| tool["name"] == "ui_control_fit_run")
            .expect("fit run control tool");
        let schema = &tool["inputSchema"];
        assert!(schema["properties"]["action"]["enum"]
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value == "REBIND_SESSION")));
        assert_eq!(schema["properties"]["newSessionId"]["minLength"], 1);
        assert!(schema["properties"]["newCurrentRect"].is_object());
        assert!(schema["properties"]["action"]["enum"]
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value == "ATTACH_STATE_REPLAY")));
        assert_eq!(
            schema["properties"]["stateReplay"]["properties"]["steps"]["minItems"],
            1
        );
        assert!(schema["allOf"][0]["then"]["required"]
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value == "projectRoot")));
    }
}

fn node_selector_schema() -> Value {
    json!({
        "type":"object",
        "oneOf":[{"required":["runtimeNodeId"]},{"required":["selector"]},{"required":["definitionId"]}],
        "properties":{
            "runtimeNodeId":{"type":"string","description":"精确但仅当前 Runtime 有效"},
            "selector":stable_selector_value_schema(),
            "definitionId":{"type":"string","description":"兼容字段；多实例时会拒绝歧义"},
            "instanceKey":{"type":"string"},
            "screenId":{"type":"string"}
        }
    })
}

fn stable_selector_value_schema() -> Value {
    json!({
        "type":"object","required":["definitionId"],
        "properties":{
            "definitionId":{"type":"string","minLength":1},
            "instanceKey":{"type":"string"},
            "screenId":{"type":"string"}
        }
    })
}

fn rect_schema() -> Value {
    json!({"type":"object","properties":{"rect":rect_value_schema()}})
}

fn launcher_artifact_schema() -> Value {
    json!({
        "type":"object","additionalProperties":false,
        "required":["source","path","sha256"],"properties":{
            "source":{"const":"ANDROID_LAUNCHER"},
            "path":{"type":"string","minLength":1,"maxLength":4000},
            "sha256":{"type":"string","pattern":"^[A-Fa-f0-9]{64}$"}
        }
    })
}

fn fit_visual_mask_schema() -> Value {
    json!({
        "type":"object",
        "description":"排除区域坐标相对 target crop；只允许动态内容或批注，合计不超过 25%。",
        "properties":{
            "regions":{
                "type":"array","maxItems":24,
                "items":{
                    "type":"object","required":["kind","rect","reason"],
                    "properties":{
                        "kind":{"enum":["DYNAMIC_CONTENT","ANNOTATION"]},
                        "rect":rect_value_schema(),
                        "reason":{"type":"string","minLength":1,"maxLength":240}
                    }
                }
            }
        }
    })
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
        assert_eq!(
            tool["inputSchema"]["properties"]["restart"]["type"],
            "boolean"
        );
        assert!(tool["description"]
            .as_str()
            .unwrap()
            .contains("IN_PROGRESS"));
    }
}
