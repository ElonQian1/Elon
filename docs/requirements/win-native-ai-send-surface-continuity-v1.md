---
version_status: current
requirement_status: accepted
reviewed_at: 2026-08-20
---

# Win 原生 AI 发送界面连续性 V1

## 目标

用户在 Win 生产首页的一龙原生聊天输入框向 ChatGPT 或 Google AI 发送消息后，
界面必须留在一龙聊天页，并在后台等待官方 WebView 的可见语义回复同步回来。

## 必须实现

- 成功发送不得自动派发“打开官方 AI 标签页”事件。
- 保留乐观用户消息、后台回复刷新、失败回滚和上下文绑定门禁。
- “官网完整内容”和“显示官方页”继续作为用户主动选择的入口。
- ChatGPT 与 Google AI 使用同一发送界面连续性规则，不建立厂商分支。

## 非目标

- 不修改官方网页适配器、Cookie、Profile 或登录规则。
- 不复制官方富文本到第二套实现。
- 不修改 PWA、Android 或聊天 UI 布局。
- 不发送真实测试消息。

## 验收标准

1. 发送成功分支继续启动原生回复刷新，但不请求官方标签页。
2. 用户主动打开官方页的现有入口和内部标签功能保持可用。
3. 用户浏览器合同测试与 PC 生产构建通过。
4. Win 发布后，发送链路不会因成功回执自动切换聊天 surface。

## 实现范围

- `pc-frontend/src/features/user-browser/useLocalAiWebChatController.ts`
- `pc-frontend/scripts/test-ai-browser-tabs.cjs`
- `docs/user-browser-module-integration.md`
