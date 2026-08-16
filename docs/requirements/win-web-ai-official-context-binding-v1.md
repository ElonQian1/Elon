# Win 网页 AI 官方上下文绑定 V1

## 目标

修复 Windows 生产首页中“本地缓存看似连续、底层 ChatGPT 或 Google AI 官方网页实际已经进入新会话”的问题。原生 UI 必须只在当前消息快照、官方页面和厂商会话属于同一个上下文时允许发送，并在厂商切换或应用重启后恢复正确的官方会话。

## 范围

- 区分缓存回显、上下文恢复中、已绑定和绑定冲突状态。
- ChatGPT 与 Google AI 的发送命令在前端和 Tauri 宿主双重检查上下文绑定。
- 兼容官方网页的单页应用路由；WebView2 未上报顶层导航时，允许经过清洗的语义页面键推进当前会话。
- Google AI 只恢复明确属于 AI Mode 的官方搜索地址，并将完整恢复地址限制在本机 DPAPI 缓存内。
- 发送回执先于新页面快照到达时保持发送暂停，直到本轮用户消息所在快照完成绑定。
- 生产首页继续即时显示 owner/provider 隔离的只读缓存。

## 非目标

- 不读取或上传 Cookie、Token、请求头、网页私有接口或网络正文。
- 不伪造 ChatGPT 或 Google 官方会话、项目和账号能力。
- 不在没有用户即时授权时向第三方 AI 发送真实验收消息。
- 本轮不要求完整安装包构建、发布和真实账号现场验收；这些在 Goal 最终统一验证阶段执行。

## 验收标准

1. DPAPI 缓存存在但官方页尚未恢复时，`contextReady=false` 且发送失败关闭。
2. ChatGPT/Google 消息快照只有在页面键和会话键一致时进入 `bound`。
3. `send_prompt` 在 Rust 命令入口拒绝 cached、restoring、empty 或 unbound 上下文。
4. ChatGPT SPA 从 `/` 切到 `/c/...` 即使没有 WebView2 导航回调，也保留同一会话并更新可恢复地址。
5. Google AI 重启可恢复带 `udm=50` 或 `aep=11` 的官方 `/search` 会话；普通 Google 搜索不会恢复。
6. URL query 不出现在前端状态、应用诊断或导航日志中，只存在于当前 Windows 用户可解密的缓存和官方 WebView 导航中。
7. PC 用户浏览器合同、TypeScript 类型检查和 Tauri 定向 Rust 测试通过。
8. 真实多轮追问、厂商切换和重启恢复保留为需要用户授权的最终安装版验收。

## 预计实现范围

- `desktop-shell/src-tauri/src/local_ai_browser.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/adapter.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/google_ai_mode.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/snapshot_cache.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/state.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/state/context.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/state/tests.rs`
- `pc-frontend/src/features/user-browser/localAiBrowserApi.ts`
- `pc-frontend/src/features/user-browser/localAiUserState.ts`
- `pc-frontend/src/features/user-browser/useAiWebChatBackend.ts`
- `pc-frontend/scripts/test-local-ai-context-contract.cjs`
- `docs/user-browser-module-integration.md`
