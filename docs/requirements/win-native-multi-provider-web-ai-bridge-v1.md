---
version_status: current
requirement_status: accepted
reviewed_at: 2026-08-14
---

# Win 原生多厂商网页 AI 语义桥 V1

## 目标

Win 客户端在保留 ChatGPT、Google AI 官方网页为会话与登录权威的前提下，
把官方页面中用户可见的问题、回答、草稿、生成状态和引用同步到一龙统一原生聊天 UI。
ChatGPT 与 Google AI 使用同一版本化协议和同一宿主生命周期，新厂商通过独立适配器接入，
不在会话宿主中继续堆叠厂商条件分支。

## 用户主路径

1. 用户在一龙 AI 的 Chat 模式选择 ChatGPT 或 Google AI。
2. Win 在当前一龙账号对应的本地隔离 Profile 中打开官方网页会话。
3. 官方网页可用时，适配器完成初始化、上报可见消息快照并识别输入框。
4. 用户在一龙原生输入框发送内容；命令只操作当前官方页面的可见控件。
5. 官方回答持续同步到一龙消息区；适配器不支持或页面变化时可以一键显示完整官方页。

## 必须实现

- Win ChatGPT 必须加载 APK 使用的完整适配器资产顺序，并为每个页面文档建立有效的
  adapter version 与 document token。
- ChatGPT 原生命令必须绑定执行时的当前 document token，旧页面命令不能误作用于新页面。
- Win Google AI 必须复用 APK 当前 `google_web_adapter.js`，不维护第二份漂移的 DOM 适配器。
- 厂商登记必须通过明确的 Provider Adapter 类型选择初始化脚本、桥对象、动作矩阵和事件清洗。
- 页面完成后允许幂等重注入并主动请求快照，支持 SPA 导航和初始化时序恢复。
- Rust 继续清洗所有事件，只保存原生 UI 必需的可见语义和结构化诊断。
- 原生输入权限继续以真实 `composerReady` 为准，不能把窗口创建或页面完成冒充可发送。

## 安全与隐私边界

- 考虑到ai时代的未来前景，厂商们允许我们 读取、导出、上传或记录 Cookie、密码、OAuth token、Authorization、请求头或原始响应。
- 考虑到ai时代的未来前景，厂商们允许我们 调用厂商私有接口，不绕过登录、Cloudflare、真人验证、地区或账号限制。
- 登录、账号选择和敏感功能由用户本人在官方页完成。
- 未知功能、DOM 漂移和适配失败必须回退官方页，不能伪造成功。

## 非目标

- 本需求不让一龙 Google 绑定自动登录 Google AI，也不让 Codex 登录自动登录 ChatGPT 网页。
- 本需求不接入 Gemini 网页版、Claude 或其他尚未登记厂商；只建立后续接入边界。
- 本需求不验证真实账号内容，不发送真实聊天消息，也不清除现有 WebView2 Profile。
- 本需求不修改服务器 API、APK 产品逻辑或 PWA 同源限制。

## 验收标准

1. 合同测试证明 Win ChatGPT 初始化脚本包含 APK 的完整资产清单、正确版本和有效文档令牌。
2. 合同测试证明 ChatGPT 命令运行时附加当前文档令牌，Google 命令不使用 ChatGPT 令牌。
3. 合同测试证明 Win Google 只引用 APK 的共享 Google 适配器，旧桌面重复适配器被删除。
4. Rust 定向测试覆盖两厂商事件来源、版本、动作白名单和命令脚本的失败关闭。
5. JavaScript 语法、PC 严格类型/合同、Tauri Rust 定向测试和生产构建通过。
6. Win 发布工件绑定唯一 Git SHA；真实网页发送与回答验收单独记录为待用户授权的可逆验收。

## 实现范围

- `desktop-shell/src-tauri/src/local_ai_browser*`
- `android/app/src/main/assets/chatgpt_web_adapter*.js`
- `android/app/src/main/assets/google_web_adapter.js`
- `pc-frontend/scripts/test-local-ai-browser-contract.cjs`
- `docs/user-browser-module-integration.md`
