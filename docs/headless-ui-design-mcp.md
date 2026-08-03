---
version_status: current
reviewed_at: 2026-08-03
---

# 后台多端 UI 设计 MCP

## 1. 目标

一龙把“微调画布”从只能由用户打开的 PC 页面，拆成两个可独立工作的部分：

1. 后台设计数据面：AI 通过 `yilong-ui-live` MCP 发现、打开、捕获和读取项目页面。
2. 可选可视客户端：PC 微调画布消费同一设计会话，提供中间画面、平台切换和右侧持续对话。

第一部分不依赖 PC 画布是否打开。Codex 等代理可以在绑定的项目工作区中完成“找到目标 → 加载页面 → 受限交互 → 读取 UI 树 → 引用像素工件”的闭环。第二部分是后续阶段的交互增强，不是后台能力的前置条件。

## 2. 当前实现状态

当前代码已经形成以下第一阶段能力：

- 发现 Web、PWA、Tauri 和 Android 设计目标，并返回来源目录、配置文件、适配器和证据级别。
- 在项目内创建与当前 MCP 会话绑定的 `designSessionId`，无需打开 PC 画布。
- Web、PWA 和 Tauri 前端复用受控无头 Chromium，执行稳定 selector 的点击、等待和文本断言。
- 每次成功捕获同时生成 PNG 和紧凑语义 UI 树，两者都只返回绝对路径、SHA-256 和小型元数据，不返回 Base64。
- AI 可按 selector、role、label 或 tag 查询 UI 树，默认最多读取 40 个节点，单次最多 80 个。
- Android 继续复用既有 Android Live Runtime；Runtime 未连接时明确返回 `PREPARATION_REQUIRED`，不会用浏览器画面冒充 Android。
- `ui_get_project_profile` 的 schema v3 包含相同的多端目标摘要，其他代理可以先读小型档案再决定调用什么工具。

本阶段尚未实现或尚未验证：

- Tauri 原生宿主窗口、系统菜单、原生对话框和 Rust command 的运行时证据。
- Web/PWA/Tauri 共用的持久浏览器进程与跨捕获会话状态；当前每次捕获仍使用独立临时浏览器 profile。
- 通用多端设计草稿、撤销栈、源码绑定和写回回执；现有 Android Live Patch 与 PWA/APK 源码闭环仍保持各自契约。
- PC 画布的平台切换器、中央会话画面和默认常驻右侧聊天对新后台会话的消费。
- 真机、模拟器、Tauri 原生窗口、人工视觉、完整 E2E 或发布验收。

因此，当前 Tauri 证据必须标记为 `TAURI_FRONTEND_WEBVIEW_ONLY`，`nativeHostVerified=false`；文档和 UI 都不得把它表述为 Tauri 原生宿主已经通过。

## 3. 代理的标准调用顺序

```text
ui_list_design_targets
  -> ui_open_design_target(platform, route, url?, viewport?)
  -> ui_capture_design_surface(designSessionId, capture)
  -> ui_get_design_surface(designSessionId, query?, limit?)
```

调用规则：

1. MCP 会话必须绑定项目 `EDIT_ROOT`。
2. 先枚举目标，不凭目录名称猜测平台。
3. `ui_open_design_target` 只打开后台会话，不启动 PC 画布。
4. Web/PWA/Tauri 捕获参数复用 `ui_capture_pwa_runtime` 的 URL、认证、fixture、viewport 和受限 steps 契约。
5. Android 返回准备要求时，继续走 `ui_get_runtime_status`、`ui_prepare_debug_runtime`、`ui_get_screen_summary` 和 `ui_get_current_crop`。
6. 默认先读取语义 UI 树；只有布局、颜色、间距或像素差需要视觉判断时，再按路径读取 PNG。
7. 任何设计修改都仍须由源码或现有 Live Patch 工具产生；PNG 和 UI 树是证据，不是源码真源。

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
| Tauri | `TAURI_WEBVIEW_FRONTEND` | 仅前端 WebView |
| Android | `ANDROID_LIVE_RUNTIME` | Android Runtime |

目标发现最多检查 4,000 个文件，遵守 Git ignore，并跳过 `.git`、`.elon`、`target`、`build`、`.gradle`、`node_modules` 和 `dist`。返回值报告 `filesInspected`、`truncated` 和 `contentEmbedded=false`，不会把文件正文塞进项目档案。

## 5. 后台设计会话

会话记录保存在：

```text
.elon/ui-tuner/headless-design/sessions/design_<uuid>.json
```

记录包含平台、目标、route、脱敏 URL、viewport、状态、最近证据引用和时间戳。它必须同时满足：

- `designSessionId` 使用固定格式并通过路径校验。
- 记录只能由创建它的 MCP session 读取或继续操作。
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
- 截图、UI 树、manifest 和 session 文件都不能作为“源码已经修改”的证明。

平台覆盖必须单独声明。浏览器证据不能证明 Android Runtime，Tauri 前端证据不能证明原生宿主，模拟器不能冒充用户要求的真机。

## 8. 与 PC 微调画布的目标关系

PC 端后续应成为后台会话的客户端，而不是第二套状态机：

```text
用户自然语言
  -> AI 选择 platform / route
  -> 后台 design session
  -> 中央 UI 画面与选区
  -> 右侧连续对话
  -> 修改源码或应用可撤销草稿
  -> 同一 session 重新捕获证据
```

建议界面保持三个稳定区域：左侧项目/页面/平台导航，中间实际页面与选区，右侧默认打开的 AI 对话。用户说“修改 Web 登录页”或“看 Tauri 设置页”时，代理先切换后台目标和 route，再让画布订阅该 session；用户不打开画布时，同一工具链仍可完全后台运行。

PC 接入时必须复用 `designSessionId`、平台覆盖和证据引用，不在浏览器 local state 里重新定义“当前页面”。对话记录引用 selector、route、源码绑定和工件哈希，不内嵌整张 PNG。

## 9. 下一阶段顺序

1. Tauri Runtime Adapter：管理 dev server/Tauri 生命周期，区分 WebView DOM、窗口和原生宿主证据。
2. Stateful Interaction：在受控权限下维持浏览器会话，支持导航、表单非秘密值和更细的可访问性操作。
3. Design Draft：统一 selector/node/source binding、可撤销 patch、平台覆盖和 writeback receipt。
4. PC Canvas Client：平台切换、中央实时画面、选区同步、默认右侧聊天与会话恢复。
5. 验证矩阵：分别执行 Web/PWA 浏览器、Tauri 原生窗口、Android 隔离模拟器，以及用户明确要求后的真机复核。

每一阶段都先扩展相同 MCP 契约；不得通过让代理操控 Windows 桌面来绕过缺失的数据面。

## 10. 实现入口与核验

主要实现引用：

- `file:server/src/node_agent_android_live/design_target_discovery.rs`
- `file:server/src/node_agent_android_live/design_targets.rs`
- `file:server/src/node_agent_pwa_runtime/semantic_tree.rs`
- `file:server/src/node_agent_pwa_runtime/artifact.rs`
- `file:server/src/node_agent_ui_design_workspace.rs`
- `symbol:node_agent_android_live::design_targets::tool_definitions`
- `symbol:node_agent_android_live::design_targets::call`
- `symbol:node_agent_ui_design_workspace::build_project_ui_profile`
- `test:server/src/node_agent_android_live/design_targets_tests.rs`

代码形成不等于平台验收。判断当前事实时还需同时读取 `AI_CURRENT.md`；判断 PWA 网络、认证、工件和浏览器生命周期时继续读取 `docs/system-architecture.md` 的“PWA Runtime 像素证据”。
