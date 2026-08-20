---
version_status: current
reviewed_at: 2026-08-20
---

# Windows 后台 AI 官方 WebView 随窗口缩放重新停放

## 目标

修复一龙 Win Tauri 工作台在原生 AI 聊天模式下最大化、还原或跨 DPI 缩放后，后台 Google AI Mode / ChatGPT 官方子 WebView 从右下角露出并覆盖原生输入区的问题。

## 当前失败

后台官方 WebView 为保持游客 Cookie、DOM、适配器和完整响应式视口，会保留完整尺寸并移动到主窗口右下边界外。该坐标只在进入后台时计算一次；主窗口从普通尺寸放大后，旧边界坐标进入新的可视区域，官方页因而从右下角重新出现并遮挡、抢占原生输入。

## 范围

- Windows Tauri 主窗口的 resize、maximize、restore 和 DPI 变化。
- 仅重新停放当前为后台状态、且仍挂载在主窗口内的 AI 官方 WebView。
- 保留现有官方页完整视口、游客 Profile、Cookie、聊天上下文、原生语义同步和官方页显示能力。
- 增加状态选择与窗口事件合同回归。
- 复用 Android 已有 ChatGPT 适配器资源，补齐 Win 引导清单中缺失的 skin 模块，保持两端清单合同通过。
- 发布 Windows 节点并在本机最大化窗口实测。

## 非目标

- 不修改 PWA、Android 或移动端。
- 不重做聊天 UI，不删除现有会话、缓存或 Profile。
- 不把后台 WebView 缩成极小视口，不销毁或永久隐藏官方页。
- 不改变第三方登录、网页内容提取或消息渲染合同。

## 验收标准

1. 主窗口最大化、还原、手动缩放或 DPI 变化后，后台 Google AI Mode / ChatGPT 官方页不会出现在原生聊天界面内。
2. 正在前台展示的官方页不会被 resize 处理错误移走，并继续使用现有嵌入区域。
3. 最大化状态点击“新对话”后，原生输入框可立即聚焦、输入和发送，不被官方 WebView 覆盖或抢焦点。
4. 后台官方页保持完整视口与同一 WebView2 Profile；游客 Cookie、页面 DOM、适配器和上下文同步不因停放而丢失。
5. Rust 定向测试、格式检查、生产发布构建和本机真实最大化工作流通过。

## 预计实现范围

- `desktop-shell/src-tauri/src/main.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/embedded_view.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/chatgpt_adapter_bootstrap.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/state.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/state/tests.rs`
- `docs/user-browser-module-integration.md`
