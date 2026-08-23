---
version_status: current
reviewed_at: 2026-08-23
implementation_status: proposed
---

# Win 本地模式生产 AI 聊天入口 V1

## 用户问题

一龙 Win 工作台进入本地模式后，侧栏只显示本机任务、Codex 控制台和“官方 AI 会话检查”。
ChatGPT / Google AI 的生产聊天页 `/ai` 虽然已经具备游客身份、上下文绑定、官网回答节点、
富内容 AST 与原生缓存能力，却没有可见入口。普通用户只能进入诊断页，无法发现并使用已经完成的
生产聊天体验。

## 范围

- 只调整 Win 本地工作台的侧栏导航和对应静态合同；不修改 PWA、云端导航或 APK。
- 在本地模式保留本机任务、Codex 控制台和官方 AI 会话检查，同时增加 `/ai` 生产聊天入口。
- 入口文案明确指向 ChatGPT / Google AI，不再把生产聊天误解为诊断页或待接入能力。
- 本地工作台根路由仍默认进入本机任务，避免启动时自动创建第三方 WebView；只有用户打开 AI
  聊天入口后才按现有后台同步策略恢复官方会话。
- 继续复用现有 `AiHomePage`、`AiChatPage` 和 `useAiWebChatBackend`，不新增聊天窗口或渲染器。

## 验收标准

1. Win 本地模式侧栏同时显示本机任务、Codex 控制台、生产 AI 聊天和官方 AI 会话检查四个入口。
2. 点击生产 AI 聊天进入 `/ai`，首屏使用现有原生聊天 UI；ChatGPT / Google AI 的游客会话、
   输入框、缓存和官网回答节点继续由同一生产链路提供。
3. `/user-browser` 继续作为诊断与登录检查页存在，不替代或遮蔽 `/ai` 生产聊天入口。
4. 本地模式根路由仍为 `/local-tasks`，启动不会因新增入口主动打开或等待第三方网页。
5. PC 本地模式合同、网页 AI 合同、typecheck、lint、build 与 Win 全量验证通过；不改 PWA。

## 非目标

- 不改变云端工作台的侧栏顺序或平台 AI 业务路由。
- 不修改 ChatGPT / Google 官方网页适配器、Cookie、Profile 或富内容协议。
- 不新增独立测试聊天窗、弹窗 WebView 或完整浏览器。

## 预计实现范围

- `pc-frontend/src/features/shell/ServerRail.tsx`
- `pc-frontend/scripts/test-local-tasks.cjs`
- `pc-frontend/scripts/test-local-ai-browser-contract.cjs`
