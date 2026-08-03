---
version_status: current
reviewed_at: 2026-08-03
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
- Web、PWA 和 Tauri 前端复用受控无头 Chromium，执行稳定 selector 的点击、等待和文本断言。
- 每次成功捕获同时生成 PNG 和紧凑语义 UI 树，两者都只返回绝对路径、SHA-256 和小型元数据，不返回 Base64。
- AI 可按 selector、role、label 或 tag 查询 UI 树，默认最多读取 40 个节点，单次最多 80 个。
- Android 继续复用既有 Android Live Runtime；Runtime 未连接时明确返回 `PREPARATION_REQUIRED`，不会用浏览器画面冒充 Android。
- Tauri 目标可按 `ui_prepare_tauri_runtime -> ui_capture_tauri_host -> ui_stop_tauri_runtime` 管理项目发现出的开发命令，且只枚举该 Runtime 的后代进程。Windows 原生捕获通过 `PrintWindow` 保存窗口 PNG、标题、边界、PID 和 SHA-256；只有成功捕获后才返回 `nativeHostVerified=true`。
- Web、PWA、Tauri 和 Android 共用项目级 Design Draft：支持 selector/scope/style patch、目标平台、源码绑定、乐观 revision、最多 50 层内部历史和单步撤销；MCP 只返回紧凑当前状态与 `historyDepth`，不回传完整历史。
- 写回前通过 `ui_begin_design_writeback` 固定 Git/source 基线；AI 修改真实源码并完成分平台验证后，通过 `ui_complete_design_writeback` 持久化 changed files、source hashes、Git revision 和平台证据。草稿及截图不能冒充源码写回完成。
- `ui_get_project_profile` 的 schema v3 包含相同的多端目标摘要，其他代理可以先读小型档案再决定调用什么工具。
- Node Admin 提供与 MCP 工具同源的项目级 HTTP 适配层；PC 不复制目标发现、会话、Tauri Runtime、草稿或证据状态机。
- PC `/pc/ui-tuner` 默认进入“多端后台”：左侧平台/会话/UI 树，中间 PNG、Tauri WebView/原生窗口证据与语义选区，右侧常驻项目 Codex 对话。用户可选择节点、重放稳定 selector 的后台点击、保存/撤销草稿并发起写回；AI 消息携带紧凑草稿、route、selector、工件路径与哈希，不嵌入 Base64 或完整历史。

本阶段尚未实现或尚未验证：

- Web/PWA/Tauri 共用的持久浏览器进程与跨捕获会话状态；当前每次捕获仍使用独立临时浏览器 profile。
- 浏览器表单非秘密值、键盘输入和比 click/waitFor/assertText 更完整的可访问性操作。
- Tauri 系统菜单、原生对话框和 Rust command 的行为级证据；当前原生证据只覆盖项目 Runtime 后代进程中的可见窗口像素。
- 真机、模拟器、Tauri 原生窗口实际启动、浏览器实际启动、人工视觉、完整 E2E 或发布验收。
- 当前安装节点的 MCP schema v1.6 发布与升级；仓库代码形成不代表现有节点已经具备这些工具。

因此，Tauri 前端截图仍只能证明 WebView；只有 `ui_capture_tauri_host` 返回带 SHA-256 的原生工件时，当前设计会话才可声明 `nativeHostVerified=true`。代码已通过目标 Rust `cargo check` 和 PC TypeScript/ESLint 门禁，但尚未执行上述平台运行验收。

## 3. 代理的标准调用顺序

```text
ui_list_design_targets
  -> ui_list_design_sessions(limit?)
  -> 恢复已有 designSessionId 或 ui_open_design_target(platform, route, url?, viewport?)
  -> ui_capture_design_surface(designSessionId, capture)
  -> ui_get_design_surface(designSessionId, query?, limit?)
  -> ui_create_design_draft(designSessionId, ...)
  -> ui_update_design_draft(draftId, expectedRevision, sourceBinding?, patches?, targetPlatforms?)
  -> ui_begin_design_writeback(draftId, expectedRevision)
  -> AI 修改绑定的真实源码
  -> 按目标平台重新捕获或验证
  -> ui_complete_design_writeback(draftId, expectedRevision, receiptId, changedFiles, evidence)
```

Tauri 需要原生窗口证据时，在前端捕获之外执行：

```text
ui_prepare_tauri_runtime(designSessionId)
  -> 轮询至 READY
  -> ui_capture_tauri_host(designSessionId)
  -> ui_stop_tauri_runtime(designSessionId)
```

调用规则：

1. MCP 会话必须绑定项目 `EDIT_ROOT`。
2. 先枚举目标，不凭目录名称猜测平台。
3. 先尝试恢复同项目最近会话；目标、route 或 URL 改变时再 `ui_open_design_target`。它只打开后台会话，不启动 PC 画布。
4. Web/PWA/Tauri 捕获参数复用 `ui_capture_pwa_runtime` 的 URL、认证、fixture、viewport 和受限 steps 契约。
5. Tauri 的 prepare 只能使用目标发现得到的模块目录与受支持包管理器/Cargo 命令，不接受任意命令；原生窗口捕获完成后及时 stop。
6. Android 返回准备要求时，继续走 `ui_get_runtime_status`、`ui_prepare_debug_runtime`、`ui_get_screen_summary` 和 `ui_get_current_crop`。
7. 默认先读取语义 UI 树；只有布局、颜色、间距或像素差需要视觉判断时，再按路径读取 PNG。
8. Design Draft 只是意图与撤销边界。开始写回前必须具备 `BOUND` 源码绑定；开始后由代理修改真实源码，再提交平台证据完成回执。

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
- steps 只允许稳定 selector 的 `click`、`waitFor`、`assertText`。
- fixture 只允许非秘密项目数据，疑似凭据的键失败关闭。
- 浏览器 profile、CDP 端口和进程在单次捕获结束时回收。
- Tauri 只能启动项目目标发现形成的命令，只跟踪和停止已登记 Runtime 的进程树；原生截图不读取整张桌面。
- 草稿更新要求 `expectedRevision`，源码绑定路径必须位于项目内且绑定范围有效；写回完成前会重新验证源码文件、摘要和平台证据。
- 截图、UI 树、manifest 和 session 文件都不能作为“源码已经修改”的证明。

平台覆盖必须单独声明。浏览器证据不能证明 Android Runtime，Tauri 前端证据不能证明原生宿主，模拟器不能冒充用户要求的真机。写回回执的最低证据为：Web 需要 `browserCaptured` 与 `routeRevision`；PWA 需要 `runtimeReloaded` 与 `routeRevision`；Tauri 需要 `frontendCaptured`、`nativeHostVerified` 与 64 位 `nativeArtifactSha256`；Android/APK 需要 `runtimeConnected` 与 `apkPath`。任一目标平台缺证据时返回 `EVIDENCE_MISSING`，不得标记完成。

## 8. 与 PC 微调画布的关系

PC 端现在是后台会话的可选客户端，而不是第二套状态机：

```text
用户自然语言
  -> AI 选择 platform / route
  -> 后台 design session
  -> 中央 UI 画面与选区
  -> 右侧连续对话
  -> 修改源码或应用可撤销草稿
  -> 同一 session 重新捕获证据
```

当前界面保持三个稳定区域：左侧平台、最近会话和紧凑 UI 树，中间实际像素证据与语义选区，右侧默认打开的 AI 对话。用户说“修改 Web 登录页”或“看 Tauri 设置页”时，客户端或代理先切换后台目标和 route，再读取/捕获同一 session；用户不打开画布时，同一工具链仍可完全后台运行。

PC 只在 localStorage 保存工作区显示模式，不把“当前页面”作为设计真源。项目 session、draft、writeback receipt、平台覆盖和证据引用来自 Node 数据面；对话 context pack 引用 selector、route、紧凑草稿、源码绑定和工件哈希，不内嵌整张 PNG 或完整撤销历史。中间语义框单击用于选择；“后台点击”会在新的隔离浏览器中重放稳定 selector 并重新捕获，不声称维持长期登录态。Tauri 页签明确区分 WebView 与原生窗口，未取得原生工件时保持未验证状态。

## 9. 后续验收与增强顺序

1. Stateful Interaction：在受控权限下维持浏览器会话，支持导航、表单非秘密值和更细的可访问性操作。
2. Tauri 深层证据：补系统菜单、原生对话框和 Rust command 的行为级适配器，继续与窗口像素证据分层。
3. 验证矩阵：分别执行 Web/PWA 浏览器、Tauri 原生窗口和 Android 隔离模拟器；只有用户明确要求时才做真机复核。
4. 节点发布：在平台验收后发布包含 MCP schema v1.6 的 Windows 节点，并验证旧节点升级兼容。

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
