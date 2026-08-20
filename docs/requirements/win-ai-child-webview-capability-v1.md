---
version_status: current
requirement_status: accepted
reviewed_at: 2026-08-20
---

# Win 官方 AI 子 WebView 权限绑定 V1

## 目标

Win 在主工作台窗口内后台或内嵌创建 ChatGPT、Google AI 官方子 WebView 时，
语义适配器必须能够通过既有白名单命令上报可见页面状态，让原生聊天及时获得
`adapterReady`、`composerReady` 与上下文快照。

## 必须实现

- Tauri capability 必须按官方子 WebView 的稳定标签匹配，不能按其当前宿主窗口匹配。
- ChatGPT 与 Google AI 继续只获得 `publish_local_ai_web_event`，不得继承主窗口会话控制权限。
- 子 WebView 在主窗口、弹出宿主和重新挂载之间切换时使用同一权限边界。
- 现有 Provider Adapter、Profile、缓存、官方页回退和原生 UI 不建立第二套实现。

## 非目标

- 不读取 Cookie、Token、请求正文或厂商私有接口。
- 不改变 Google/ChatGPT 官方页面的登录、地区或风控规则。
- 不修改 PWA、Android 或 React 聊天 UI。
- 不发送真实聊天消息，也不清除用户现有 WebView2 Profile。

## 验收标准

1. 两个官方 AI capability 均使用 `webviews` 标签范围，不使用宿主 `windows` 范围。
2. Google 与 ChatGPT 子 WebView 只获得语义事件上报权限。
3. 静态合同测试覆盖多 WebView 宿主权限回归并通过。
4. Tauri 定向测试与生产构建通过。
5. Win 发布后，Google 现场状态能从 `adapter_connected=false` 推进为适配器实时状态；真实发送仍由用户执行。

## 实现范围

- `desktop-shell/src-tauri/capabilities/local-ai-google-web.json`
- `desktop-shell/src-tauri/capabilities/local-ai-web.json`
- `pc-frontend/scripts/test-local-ai-browser-contract.cjs`
- `docs/user-browser-module-integration.md`
