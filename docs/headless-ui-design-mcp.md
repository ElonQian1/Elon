---
version_status: current
reviewed_at: 2026-08-05
---

# 后台多端 UI 设计 MCP

## 1. 目标

一龙把“微调画布”从只能由用户打开的 PC 页面，拆成两个可独立工作的部分：

1. 后台设计数据面：AI 通过 `yilong-ui-live` MCP 发现、打开、捕获和读取项目页面。
2. 可选可视客户端：PC 微调画布消费同一设计会话，提供中间画面、平台切换和右侧持续对话。

第一部分不依赖 PC 画布是否打开。Codex 等代理可以在绑定的项目工作区中完成“找到目标 → 恢复会话 → 加载页面 → 受限交互 → 读取 UI 树 → 引用像素工件 → 建立可撤销草稿 → 绑定源码 → 写回并记录分平台证据”的闭环。第二部分已经作为同一数据面的 PC 客户端形成代码，但不是后台能力的前置条件。

## 2. 当前实现状态

当前代码已经形成以下能力：

- 发现 Web、PWA、Tauri 和 Android 设计目标，并返回来源目录、配置文件、适配器和证据级别。
- 在项目内创建 `designSessionId`；同一 canonical Git 项目的新 MCP 会话或 PC 客户端可用 `ui_list_design_sessions` 恢复，无需保留创建时连接或打开 PC 画布。
- `ui_plan_design_intent` 把自然语言编译为项目持久的 `DesignIntentPlan`：保存意图摘要和 SHA-256，不保存完整聊天；计划明确 Web/PWA/Tauri/Android、route、state hints、会话复用/打开策略和有序工具动作。计划使用单调 `revision`；`ui_start_design_intent_plan` 绑定真实 task lease，`ui_record_design_intent_action` 保存逐动作回执，`ui_transition_design_intent_plan` 管理暂停/恢复/取消/失败/完成，`ui_replan_design_intent` 用 `replannedFrom/supersededBy` 保留替代链。规划本身不启动 Runtime、不修改源码。
- AI 任务可用 `ui_bind_design_task` 把 `taskId` 持久绑定到 designSession/draft，并获取 60–3600 秒 lease。同一 designSession 同时只允许一个未过期任务 lease；重新绑定必须提交当前 `expectedLeaseId`，运行中可续租，终态显式结算。
- `ui_list_design_events` 按 `taskId + afterCursor` 增量读取紧凑事件，支持最长 15 秒有界等待；事件只含工具、session/draft、平台/route、revision、工件哈希等摘要，项目内最多保留 1,000 条，不保存截图正文或工具完整结果。
- 每个 `consumerId + taskId` 可用 `ui_get_design_event_checkpoint` 恢复已确认 cursor，并以 `expectedRevision` 调用 `ui_commit_design_event_checkpoint` 单调提交。checkpoint 只确认已成功消费的事件，不删除事件；PC 重开后无需从头读取。
- Web、PWA 和 Tauri 前端既可单次捕获，也可按 `designSessionId` 启动持久有界 Chromium。持久模式保留同一 page、Cookie、localStorage、滚动和组件状态，最多 4 个活跃会话、空闲 15 分钟、总生命周期 60 分钟或 128 次操作。
- 安全交互支持 `click`、`waitFor`、`assertText`、`scrollIntoView`、白名单键盘键、checkbox/radio、select 和非秘密表单填入。表单值只能引用项目内已审查 `fixtureProfile.formValues` 的 key，MCP/PC 不传真实值；password、hidden、file 与疑似秘密键值失败关闭。
- 每次成功捕获同时生成 PNG 和紧凑语义 UI 树，两者都只返回绝对路径、SHA-256 和小型元数据，不返回 Base64。
- AI 可按 selector、role、label 或 tag 查询 UI 树，默认最多读取 40 个节点，单次最多 80 个。
- Android 继续复用既有 Android Live Runtime；Runtime 未连接时明确返回 `PREPARATION_REQUIRED`，不会用浏览器画面冒充 Android。
- Tauri 目标可按 `ui_prepare_tauri_runtime -> ui_capture_tauri_host -> ui_capture_tauri_behavior -> ui_stop_tauri_runtime` 管理项目发现出的开发命令，且只枚举该 Runtime 的后代进程。Windows 原生窗口通过 `PrintWindow` 保存 PNG、边界、PID 和 SHA-256；行为层另外只读原生菜单、候选系统对话框和项目显式写入的严格 command trace。
- Tauri 证据分层为 `TAURI_NATIVE_WINDOW`、`WIN32_NATIVE_MENU_OBSERVED`、`DESCENDANT_TOP_LEVEL_WINDOWS_OBSERVED` 与 `PROJECT_INSTRUMENTED_TRACE/NOT_INSTRUMENTED`。项目 trace 不含 command 参数或结果正文，不能冒充操作系统证明；数据面不提供任意菜单点击或任意 Rust command 执行。
- Web、PWA、Tauri 和 Android 共用项目级 Design Draft v2：除兼容 style patch 外，可表达 `SET_STYLE`、`SET_TEXT`、`REPLACE_ASSET`、`SET_VARIANT`、节点增删移动和 `SET_RESPONSIVE_STYLE`；每项按平台标记 `LIVE_PREVIEW`、`SOURCE_HANDOFF` 或 `UNSUPPORTED`。草稿继续使用乐观 revision、最多 50 层内部历史和单步撤销，MCP 不回传完整历史。
- `ui_preview_design_draft` 把最多 32 个白名单视觉属性临时应用到当前持久浏览器，`ui_restore_design_draft_preview` 恢复首次预览前的内联值。它们拒绝外部 CSS 资源、脚本型值和任意 JavaScript，只生成新 PNG/UI tree；响应固定声明 `previewOnly=true`、`sourceModified=false`。
- `ui_suggest_design_source_binding` 先验证会话 UI tree 的路径和 SHA-256，再结合 selector、可访问 label、role、tag 和 route，只扫描 designSession 声明的 `sourceRoots`。候选最多检查 2,000 个、单文件 512 KiB，遵守 Git ignore，跳过构建目录和秘密形态行；只返回局部 excerpt、行号、字节范围、来源 SHA-256、命中信号和 `CANDIDATE` 建议，不自动确认 `BOUND`。
- `ui_check_design_source_binding` 在写回前重新读取绑定文件，校验普通文件、2 MiB 上限、SHA-256 和 byte range，区分 `HEALTHY`、`SOURCE_CHANGED`、`RANGE_STALE`、`FILE_MISSING` 等状态。失败时可复用候选器给出有界恢复候选，但固定 `autoRebound=false`。
- `ui_plan_design_writeback` 把 DraftOperation 按目标平台编译成持久写回计划，展示框架适配器、mutation kind、确定性/AI handoff、阻塞项和 LOW/MEDIUM/HIGH 风险。`ui_decide_design_writeback_plan` 必须显式批准或带原因拒绝；批准不修改源码。`ui_begin_design_writeback` 只接受已批准且 draft revision、绑定 SHA/range 未漂移的 `writebackPlanId`，再固定 Git/source 基线。AI 修改源码并完成分平台验证后，仍由 `ui_complete_design_writeback` 持久化 changed files、source hashes、Git revision 和平台证据。
- `ui_propose_design_source_patch` 只接受已批准写回计划、已确认 binding、精确 UTF-8 byte range、片段 SHA-256 和有界 replacement。提案把完整审查内容留在项目 review artifact，API 只返回范围/摘要哈希；`ui_decide_design_source_patch` 二次批准后，`ui_apply_design_source_patch` 才能按 `APPROVED -> APPLYING -> APPLIED` journal 原子替换源码。应用后 `ui_plan_design_source_rollback` 生成精确逆向审查计划，但不自动回滚。
- `ui_create_design_regression_baseline` 只固化已通过文件哈希复核的 PNG/UI tree。重新捕获后，`ui_plan_design_regression_comparison` 固定前后证据、平台、route、viewport、允许变化 selector 和阈值；比较器通过 `ui_complete_design_regression_comparison` 提交项目内视觉/语义 diff artifact，节点复核 artifact SHA 后计算 `PASSED/FAILED`。创建契约不等于已执行比较。
- `ui_get_project_profile` 的 schema v3 包含相同的多端目标摘要，其他代理可以先读小型档案再决定调用什么工具。
- Node Admin 提供与 MCP 工具同源的项目级 HTTP 适配层；PC 不复制目标发现、会话、Tauri Runtime、草稿或证据状态机。
- `ui_get_design_capabilities` 返回节点实际安装的 `yilong-ui-live@1.11.0` schema、能力 ID、安全边界和项目已发现平台；调用成功本身才是当前节点已升级的证据。v1.11 新增意图执行生命周期、受审确定性源码补丁/回滚计划和视觉/语义回归契约。
- `ui_get_design_verification_matrix` 把草稿、当前 designSession 工件和写回回执汇总为 Web/PWA/Tauri/Android 行，明确区分 `READY`、`IN_PROGRESS`、`BLOCKED` 与 `PASSED`。只有写回回执中所有目标平台均为 `BUILD_VERIFIED + evidenceComplete=true` 才是通过。
- PC `/pc/ui-tuner` 默认进入“多端后台”：左侧平台/会话/UI 树，中间最终 UI 与语义选区，右侧常驻项目 Codex 对话。用户发送聊天时先生成或重规划 DesignIntentPlan；画布显示 plan revision、逐动作回执和暂停/恢复/取消，自动跟随 source patch、rollback、regression 事件。源码补丁必须在画布二次批准后由用户明确点击应用；回滚只生成审查计划。普通 AI 任务按 taskId/cursor 长轮询并提交 `pc-ui-tuner` checkpoint；若代理绑定另一 designSession，中间画布按事件定向恢复。

2026-08-05 已完成隔离 Windows 候选节点和正式安装节点的两阶段真实验收：Rust `--locked` check、目标契约测试、PC TypeScript 构建、ESLint、微调画布测试和 MCP bridge 测试通过；正式 7799 节点实际激活并回读 `release_identity=0.3.69+e30818e6abdd770c0f2b3fcf4affe1c0e1a294af`、`build_git_sha=e30818e6abdd770c0f2b3fcf4affe1c0e1a294af`、`yilong-ui-live@1.11.0` 与 24 个 capability ID。7799 listener 的进程路径复核为 `LocalAppData\ElonNode\一龙开发平台.exe`；激活回执从旧 `067ec391…` 精确推进到 `e30818e6…`，不会再因 7800 候选节点回报目标身份而误标正式安装成功。

代理未打开 PC 画布或可见浏览器，即在正式节点完成 Web/Tauri 目标发现、Web 会话打开、PNG 与 5 节点语义树捕获、按钮点击与文本断言、回归基线、可逆样式草稿预览/恢复、写回安全阻断及六步 DesignIntentPlan 回执闭环；计划最后进入 `COMPLETED`，task lease 进入 `SETTLED`。Web 语义查询读取到 `#status` 的“正式节点后台交互成功”；Tauri 前端也执行相同 2 步后台交互并准确返回 `TAURI_FRONTEND_WEBVIEW_ONLY`、`nativeHostVerified=false`。候选节点首次冷浏览器捕获约 33 秒，后续捕获/交互约 3–8 秒；正式节点本轮小型控制调用约 1.1–2.1 秒，页面捕获约 3.3–3.7 秒。响应只返回路径、哈希和紧凑节点，不嵌入 Base64。复验后已重新打开桌面壳，只有一个安装目录 `elon-desktop.exe`，正式 7799 节点仍保持目标 release identity 且云连接正常。

本轮仍未验证 Tauri 原生窗口、菜单、对话框或 command trace，未执行 Android Runtime、模拟器、真机、人工视觉和视觉/语义比较器回执；写回计划因 fixture 未确认源码绑定而按设计对 Web/Tauri 返回 `BLOCKED_BINDING`，没有修改源码。正式发布与升级回读已经通过，但这不代表所有平台均已验收。

Tauri 前端截图仍只能证明 WebView；只有 `ui_capture_tauri_host` 返回带 SHA-256 的原生工件时才能声明原生窗口证据。菜单、对话框和 command trace 是额外分层证据，不会单独把 `nativeHostVerified` 变为 true。

## 3. 代理的标准调用顺序

```text
ui_get_design_capabilities
  -> ui_plan_design_intent(intent, platform?, route?, state?, designSessionId?)
  -> ui_list_design_targets
  -> ui_list_design_sessions(limit?)
  -> 恢复已有 designSessionId 或 ui_open_design_target(platform, route, url?, viewport?)
  -> ui_start_design_intent_plan(planId, expectedRevision, taskId, designSessionId)
  -> 消费者读取 ui_get_design_event_checkpoint(consumerId, taskId)
  -> 并行 ui_list_design_events(taskId, afterCursor=resumeAfterCursor?, waitMs?)
  -> 每批处理成功后 ui_commit_design_event_checkpoint(consumerId, taskId, cursor, expectedRevision)
  -> 单次：ui_capture_design_surface(designSessionId, capture)
     或持久：ui_prepare_design_browser(designSessionId, capture?)
             -> ui_interact_design_browser(designSessionId, capture/navigateTo?)
             -> ui_stop_design_browser(designSessionId)
  -> ui_get_design_surface(designSessionId, query?, limit?)
  -> ui_create_design_regression_baseline(designSessionId, draftId?)
  -> ui_create_design_draft(designSessionId, operations?, patches?, ...)
  -> 可选 ui_preview_design_draft(draftId) -> ui_restore_design_draft_preview(draftId)
  -> ui_suggest_design_source_binding(draftId, limit?)
  -> ui_update_design_draft(... sourceBinding.status=CANDIDATE)
  -> 核对来源哈希/范围后 ui_update_design_draft(... sourceBinding.status=BOUND)
  -> ui_check_design_source_binding(draftId)
  -> ui_plan_design_writeback(draftId)
  -> ui_decide_design_writeback_plan(planId, expectedPlanRevision, APPROVE)
  -> ui_begin_design_writeback(draftId, expectedRevision, writebackPlanId=planId)
  -> ui_propose_design_source_patch(writebackPlanId, draftId, exact ranges + SHA)
  -> 审查 reviewArtifactPath
  -> ui_decide_design_source_patch(proposalId, expectedRevision, APPROVE)
  -> ui_apply_design_source_patch(proposalId, expectedRevision)
  -> ui_plan_design_source_rollback(proposalId, expectedRevision)
  -> 按目标平台重新捕获
  -> ui_plan_design_regression_comparison(baselineId, afterDesignSessionId)
  -> 比较器执行后 ui_complete_design_regression_comparison(... diff artifacts)
  -> ui_complete_design_writeback(draftId, expectedRevision, receiptId, changedFiles, evidence)
  -> ui_get_design_verification_matrix(draftId)
  -> ui_record_design_intent_action(planId, expectedRevision, actionOrder, status, evidenceRefs?)
  -> ui_transition_design_intent_plan(planId, expectedRevision, COMPLETE)
  -> ui_settle_design_task_binding(taskId, leaseId, succeeded?)
```

Tauri 需要原生窗口证据时，在前端捕获之外执行：

```text
ui_prepare_tauri_runtime(designSessionId)
  -> 轮询至 READY
  -> ui_capture_tauri_host(designSessionId)
  -> ui_capture_tauri_behavior(designSessionId, expectations?)
  -> ui_stop_tauri_runtime(designSessionId)
```

调用规则：

1. MCP 会话必须绑定项目 `EDIT_ROOT`。
2. 先枚举目标，不凭目录名称猜测平台。
3. 先尝试恢复同项目最近会话；目标、route 或 URL 改变时再 `ui_open_design_target`。它只打开后台会话，不启动 PC 画布。
4. 先调用能力清单。工具不存在或 `runtimeSchema` 低于任务要求时，报告节点待升级，不能把仓库源码冒充已安装能力。
5. Web/PWA/Tauri 捕获参数复用 `ui_capture_pwa_runtime` 的 URL、认证、fixture、viewport 和受限 steps 契约；需要连续页面状态时使用持久浏览器，结束后 stop。
6. Tauri 的 prepare 只能使用目标发现得到的模块目录与受支持包管理器/Cargo 命令，不接受任意命令；原生窗口与行为证据完成后及时 stop。
7. Android 返回准备要求时，继续走 `ui_get_runtime_status`、`ui_prepare_debug_runtime`、`ui_get_screen_summary` 和 `ui_get_current_crop`。
8. 默认先读取语义 UI 树；只有布局、颜色、间距或像素差需要视觉判断时，再按路径读取 PNG。
9. Design Draft 只是意图与撤销边界。开始写回前必须具备健康的 `BOUND` 源码绑定、当前 draft revision 的写回计划和显式批准；完成后必须读验证矩阵，不按 UI 上“已有截图”推断全平台通过。
10. 草稿预览只适用于 Web/PWA/Tauri 前端持久浏览器；Android 继续使用 Live Runtime。预览和恢复都不写源码，源码绑定候选也不会自动升级为 `BOUND`。
11. 一个 designSession 同时只允许一个有效 AI task lease。事件消费者必须在实际处理成功后提交自己的 checkpoint；断线重连从 `resumeAfterCursor` 继续，任务终态必须 settle。
12. `SET_STYLE` 仅在 Web/PWA/Tauri 标记为 `LIVE_PREVIEW`；Android 样式、文字/资源/variant/结构操作和响应式写回均按 capability entry 交给源码适配器，不能因已有机器契约就宣称运行完成。

### 3.1 Node Admin 同源路由

PC 使用本机 Admin token 调用与 MCP 相同的状态机，关键新增路由是：

- `POST /api/android-live/design/intents/:plan_id/start|transition|replan` 与 `.../actions/:action_order`。
- `POST /api/android-live/design/source-patches/propose`，以及 `.../:proposal_id/decision|apply|rollback/plan`。
- `POST /api/android-live/design/regressions/baselines`、`.../comparisons/plan` 与 `.../:comparison_id/complete`。

路由只负责把路径 ID 注入同名 MCP 参数并调用 `design_tools`；审批、revision、项目 canonical path、SHA 和状态迁移仍由同一 Rust 契约校验。PC 不实现第二套业务逻辑。

## 4. 目标模型

每个设计目标都包含：

| 字段 | 含义 |
|---|---|
| `id` / `platform` | `web`、`pwa`、`tauri` 或 `android` |
| `adapter` | 后台运行时适配器 |
| `evidenceLevel` | 当前证据能覆盖到的边界 |
| `sourceRoots` | 可能的 UI 源码根目录，只返回路径 |
| `configFiles` | 用于发现目标的配置证据 |
| `capabilities` | 导航、捕获、UI 树或 Live Patch 等能力 |
| `nativeHostVerified` | 是否已覆盖该平台原生宿主 |

当前适配关系：

| 平台 | 适配器 | 当前证据 |
|---|---|---|
| Web | `HEADLESS_CHROMIUM` | 浏览器运行时 |
| PWA | `HEADLESS_CHROMIUM_PWA` | PWA 浏览器运行时 |
| Tauri | `TAURI_FRONTEND_PLUS_NATIVE_HOST` | WebView 前端；原生窗口按需捕获 |
| Android | `ANDROID_LIVE_RUNTIME` | Android Runtime |

目标发现最多检查 4,000 个文件，遵守 Git ignore，并跳过 `.git`、`.elon`、`target`、`build`、`.gradle`、`node_modules` 和 `dist`。返回值报告 `filesInspected`、`truncated` 和 `contentEmbedded=false`，不会把文件正文塞进项目档案。

## 5. 后台设计会话

会话记录保存在：

```text
.elon/ui-tuner/headless-design/sessions/design_<uuid>.json
```

草稿和写回回执分别保存在：

```text
.elon/ui-tuner/headless-design/drafts/draft_<uuid>.json
.elon/ui-tuner/writeback-receipts/<receipt>.json
```

任务绑定和增量事件分别保存在：

```text
.elon/ui-tuner/headless-design/task-bindings/<sha256(taskId)>.json
.elon/ui-tuner/headless-design/events/<cursor>.json
```

意图计划、消费者断点和写回计划分别保存在：

```text
.elon/ui-tuner/headless-design/intent-plans/intent_<uuid>.json
.elon/ui-tuner/headless-design/event-checkpoints/<sha256(consumerId + taskId)>.json
.elon/ui-tuner/headless-design/writeback-plans/writeplan_<digest>.json
```

受审源码补丁、review artifact、逆向计划和回归契约保存在：

```text
.elon/ui-tuner/headless-design/source-patches/sourcepatch_<digest>.json
.elon/ui-tuner/headless-design/source-patches/<id>.review.patch
.elon/ui-tuner/headless-design/source-patches/rollback_<digest>.json
.elon/ui-tuner/headless-design/regressions/baseline_<digest>.json
.elon/ui-tuner/headless-design/regressions/compare_<digest>.json
```

记录包含平台、目标、route、脱敏 URL、viewport、状态、最近证据引用和时间戳。它必须同时满足：

- `designSessionId` 使用固定格式并通过路径校验。
- 记录只能由绑定同一 canonical Git 项目的 MCP/Node Admin 会话读取或继续操作；`mcpSessionId` 只保留创建者审计，不是长期所有权边界。
- `designSessionId` 是不可猜测 UUID，所有读写仍需项目级本机 Admin/MCP 访问权；仅知道 ID 不获得跨项目读取权。
- 会话目录 canonical path 必须位于项目根内。
- URL 只允许无用户名和密码的 http(s)，query 疑似含 token、secret、password、authorization、signature 或 api_key 时失败关闭。
- 项目工作区是会话状态与工件的唯一落点；不创建供应商私有真源。

## 6. 语义 UI 树与像素证据

Web/PWA/Tauri 前端捕获会在 PNG 旁生成 `.ui.json`。语义树单个工件最多 512 KiB、最多 400 个节点，记录：

- 稳定 selector、HTML tag、role 和可访问 label。
- 是否可交互，以及 disabled、checked、selected、input type 等状态。
- 节点边界和关键样式值。
- 页面 title、route、viewport、节点数、交互节点数和截断状态。

输入值不进入 UI 树，密码输入尤其不得读取；label 只从可访问名称、title 或 placeholder 等展示元数据派生。

`ui_get_design_surface` 读取工件前会再次验证：

1. 路径位于绑定项目根目录内。
2. 文件没有超过大小上限。
3. 实际 SHA-256 与会话证据一致。
4. JSON 可以解析。

返回的 `pixels` 与 `uiTree` 都是工件引用，`base64Embedded=false`。推荐读取顺序是 `uiTree` 后 `pixels`，这样代理不必每轮消费整张截图 token。

## 7. 安全与信任边界

后台设计沿用 PWA Runtime 的网络和交互限制：

- 默认只允许 localhost 和 loopback；额外 origin 必须由项目显式登记。
- 不接受任意 JavaScript。
- `previewStyle` 只接受视觉属性白名单和值上限；拒绝 `url()`、`image-set()`、`@import`、`expression()`、`javascript:`、分号和控制字符。首次预览保存原内联值，恢复后删除页面内临时状态。
- steps 只允许稳定 selector 的有限动作，不接受脚本；键盘只允许 Enter/Escape/Tab/方向键/Space/Home/End。
- fixture 只允许非秘密项目数据，疑似凭据的键和值失败关闭；MCP 输入不接受真实表单值。
- 单次浏览器捕获结束即回收；持久浏览器严格绑定项目、origin、认证、fixture 和 viewport，并受会话数、空闲、寿命和操作数上限约束。
- Tauri 只能启动项目目标发现形成的命令，只跟踪和停止已登记 Runtime 的进程树；原生截图不读取整张桌面，行为工具只读证据。
- 草稿更新要求 `expectedRevision`，源码绑定路径必须位于项目内且绑定范围有效；候选扫描只读已声明源码根并给出 source SHA-256。写回规划和批准都会检查绑定健康，开始写回时再检查批准计划、draft revision、文件摘要和范围，完成时仍验证源码与平台证据。
- taskId 仅允许有界 ASCII 标识；lease 冲突、过期和不匹配均失败关闭。事件目录 canonical path 必须位于项目内，单条事件最多 64 KiB，超过 1,000 条时按 cursor 删除最旧机器事件。
- checkpoint 的 consumerId/taskId、cursor 所属任务、乐观 revision 和 cursor 单调性全部验证；不得提交其他任务的 cursor 或倒退 checkpoint。
- DesignIntentPlan 的启动、状态转换、动作回执和重规划都要求 `expectedRevision`；终态只在动作回执全部成功/跳过或明确失败/取消时形成，计划替代链不可覆盖。
- source patch 只能修改草稿已确认绑定的单个项目内 UTF-8 文件；range 不重叠、片段 SHA、整文件前后 SHA、写回审批和 draft revision 任一漂移即拒绝。`APPLYING` journal 只接受应用前或应用后两个已知 SHA 以恢复。
- regression baseline 会重新读取 PNG/UI tree 并验证磁盘 SHA；比较回执的视觉/语义 artifact 必须是项目内普通文件且哈希匹配。没有比较器回执时只能是 `READY_TO_COMPARE`，不得宣称回归通过。
- 截图、UI 树、manifest 和 session 文件都不能作为“源码已经修改”的证明。

平台覆盖必须单独声明。浏览器证据不能证明 Android Runtime，Tauri 前端证据不能证明原生宿主，模拟器不能冒充用户要求的真机。写回回执的最低证据为：Web 需要 `browserCaptured` 与 `routeRevision`；PWA 需要 `runtimeReloaded` 与 `routeRevision`；Tauri 需要 `frontendCaptured`、`nativeHostVerified` 与 64 位 `nativeArtifactSha256`；Android/APK 需要 `runtimeConnected` 与 `apkPath`。任一目标平台缺证据时返回 `EVIDENCE_MISSING`，不得标记完成。

## 8. 与 PC 微调画布的关系

PC 端现在是后台会话的可选客户端，而不是第二套状态机：

```text
用户自然语言
  -> DesignIntentPlan 选择 platform / route / state / session action
  -> 后台 design session
  -> 中央 UI 画面与选区
  -> 右侧连续对话
  -> 可撤销草稿 -> 绑定健康 -> 分平台写回审批 -> 修改源码
  -> 修改前基线 -> 同目标重新捕获 -> 视觉/语义比较任务
```

当前界面保持三个稳定区域：左侧平台、最近会话和紧凑 UI 树，中间实际像素证据与语义选区，右侧默认打开的 AI 对话。用户说“修改 Web 登录页”或“看 Tauri 设置页”时，聊天发送前先保存计划并切换后台目标、route 或可复用 session；中间计划审查条显示绑定 SHA/range 健康、适配器、风险和审批。用户不打开画布时，代理调用相同 MCP/HTTP 状态机仍可完全后台运行。

PC 只在 localStorage 保存工作区显示模式，不把“当前页面”作为设计真源。项目 intent plan、session、draft、task binding、event/checkpoint、writeback plan/receipt、source patch/rollback、regression baseline/comparison、平台覆盖和证据引用来自 Node 数据面；context pack 只引用紧凑状态、路径和哈希，不内嵌完整聊天、补丁正文、整张 PNG、fixture 值或撤销历史。任务活动回调携带真实 taskId；已有 session 时 PC 先启动 plan，再让 task follower 复用同一 lease；尚未打开 session 时由后台代理打开后启动。事件游标推进会刷新 plan、patch 和 regression 状态。

## 9. 后续验收与增强顺序

1. 已完成：源码大小与文档门禁、Rust/Cargo `--locked` check、TypeScript 构建、ESLint、画布测试和 MCP bridge 测试。
2. 已完成：隔离 Windows 候选节点与正式安装节点均回读 schema v1.11 和全部 24 个 capability ID；正式 release identity、7799 listener 安装路径、激活回执和桌面壳重启后健康均已复核。
3. 已完成：正式节点上的 Web 真实浏览器 fixture 无界面发现、捕获、受限交互、断言、基线、草稿预览/恢复和 DesignIntentPlan 回执；Tauri 已验证前端 WebView 分层证据。
4. 已完成：本机发布、自动激活和能力回读；激活器只接受正式安装 listener，7800 候选节点不能推进正式发布状态。旧节点 CLI/Exec 的独立兼容矩阵仍可继续扩展。
5. 待补：source patch 漂移/`APPLYING` 恢复、rollback offset、比较器 artifact 与阈值结算的运行验收。
6. 待补：Tauri 原生窗口/菜单/对话框/插桩 trace、Android 隔离模拟器、PWA 独立目标和四端验证矩阵回执；只有用户明确要求或反馈视觉不正确时再做真机复核。

每一阶段都先扩展相同 MCP 契约；不得通过让代理操控 Windows 桌面来绕过缺失的数据面。

## 10. 实现入口与核验

主要实现引用：

- `file:server/src/node_agent_android_live/design_target_discovery.rs`
- `file:server/src/node_agent_android_live/design_targets.rs`
- `file:server/src/node_agent_android_live/design_session_store.rs`
- `file:server/src/node_agent_android_live/design_http.rs`
- `file:server/src/node_agent_android_live/tauri_host_runtime.rs`
- `file:server/src/node_agent_android_live/tauri_host_windows.rs`
- `file:server/src/node_agent_android_live/design_drafts.rs`
- `file:server/src/node_agent_android_live/design_draft_operations.rs`
- `file:server/src/node_agent_android_live/design_task_binding.rs`
- `file:server/src/node_agent_android_live/design_event_stream.rs`
- `file:server/src/node_agent_android_live/design_event_checkpoint.rs`
- `file:server/src/node_agent_android_live/design_intent_plan.rs`
- `file:server/src/node_agent_android_live/design_intent_execution.rs`
- `file:server/src/node_agent_android_live/design_draft_preview.rs`
- `file:server/src/node_agent_android_live/design_source_binding.rs`
- `file:server/src/node_agent_android_live/design_binding_health.rs`
- `file:server/src/node_agent_android_live/design_writeback_plan.rs`
- `file:server/src/node_agent_android_live/design_source_patch.rs`
- `file:server/src/node_agent_android_live/design_source_patch_store.rs`
- `file:server/src/node_agent_android_live/design_regression_contract.rs`
- `file:server/src/node_agent_android_live/design_regression_store.rs`
- `file:server/src/node_agent_android_live/design_browser_runtime.rs`
- `file:server/src/node_agent_android_live/design_verification_matrix.rs`
- `file:server/src/node_agent_android_live/tauri_behavior.rs`
- `file:server/src/node_agent_android_live/tauri_behavior_windows.rs`
- `file:server/src/node_agent_pwa_runtime/stateful.rs`
- `file:server/src/node_agent_pwa_runtime/interaction.rs`
- `file:server/src/node_agent_pwa_runtime/style_preview.rs`
- `file:server/src/node_agent_pwa_runtime/semantic_tree.rs`
- `file:server/src/node_agent_pwa_runtime/artifact.rs`
- `file:server/src/node_agent_source_preview/writeback_receipt.rs`
- `file:server/src/node_agent_source_preview/writeback_receipt_workspace.rs`
- `file:server/src/node_agent_ui_design_workspace.rs`
- `file:pc-frontend/src/features/ui-tuner/headless-design/`
- `file:pc-frontend/src/features/ui-tuner/UiTunerConversationDrawer.tsx`
- `symbol:node_agent_android_live::design_targets::tool_definitions`
- `symbol:node_agent_android_live::design_targets::call`
- `symbol:node_agent_ui_design_workspace::build_project_ui_profile`
- `test:server/src/node_agent_android_live/design_targets_tests.rs`

代码形成不等于平台验收。判断当前事实时还需同时读取 `AI_CURRENT.md`；判断 PWA 网络、认证、工件和浏览器生命周期时继续读取 `docs/system-architecture.md` 的“PWA Runtime 像素证据”。
