---
version_status: current
requirement_status: accepted
reviewed_at: 2026-08-15
---

# Win 网页 AI 原生能力补齐 V1

## 目标

在一龙首页同一个聊天 UI 中补齐 Win 与 APK 已有 ChatGPT WebView 语义能力的代码路径，
并保持 Google AI 模式继续使用同一消息流。厂商官方网页仍是账号、会话、模型、工具和回答的
权威来源；一龙只呈现经过清洗的可见语义并调用白名单页面动作。

## 本轮代码范围

- 命令使用唯一 `requestId`，结果必须同时匹配动作与请求，不能复用旧回执。
- ChatGPT 与 Google 消息保留 Markdown、引用以及图片、文件、代码、表格、公式、音视频、
  图表和交互内容的安全结构化描述。
- ChatGPT 原生工具栏提供模型、官网工具、附件、听写和产品功能入口。
- 附件只由 ChatGPT 官方文件输入控件选择；一龙不读取文件路径或文件正文。
- 模型、工具和功能选项只使用当前页面动态快照，不把已失效的 DOM 控件当成长期配置。
- 厂商切换继续先显示按 owner/provider 隔离的快照，再在后台刷新官方页。
- 本机诊断只记录事件类型、消息/助手回复计数、流式状态、命令动作、请求号和成功状态。

## 能力矩阵

| 能力 | ChatGPT Win 代码 | Google AI Win 代码 | 真实网页验收 |
|---|---|---|---|
| 原生发送、停止、新对话 | 已接线 | 已接线 | 待现场 |
| Markdown 与公开引用 | 已接线 | 已接线 | 待现场 |
| 代码/表格/文件等结构化卡片 | 已接线 | 适配器返回时可呈现 | 待现场 |
| 模型选择 | 已接线 | 官网未登记 | 待 ChatGPT 现场 |
| Composer 工具 | 已接线 | 官网未登记 | 待 ChatGPT 现场 |
| 官方附件选择与移除 | 已接线 | 官网未登记 | 待 ChatGPT 现场 |
| 官网听写控制 | 已接线 | 官网未登记 | 待权限现场 |
| ChatGPT 功能导航 | 已接线 | 不适用 | 待 ChatGPT 现场 |
| 历史、项目与快照回显 | 已接线 | 当前搜索会话 | 待性能现场 |

“已接线”只表示协议、宿主、React UI 和白名单动作具有完整代码路径；不表示厂商当前 DOM、
账号灰度、浏览器权限或地区策略已经在用户设备通过。

## 安全与失败关闭

- 不读取或导出 Cookie、密码、OAuth token、Authorization、请求头和私有 API 响应。
- URL 只保留 HTTPS host 与 path，丢弃 query、fragment、userinfo 和异常端口。
- 结构化内容只保留类型、用户可见标签和有界元数据，不保存 DOM 或 HTML。
- 模型、工具、听写或附件动作失效时显示官方页回退，不伪造成功能力。
- 登录、Cloudflare、人机验证、麦克风权限和文件选择必须由用户本人完成。

## 本轮验证边界

代码全部铺设后统一执行一次 PC TypeScript/Vite、用户浏览器合同和 Tauri Rust 定向检查。
本轮不安装客户端、不打开用户窗口、不登录真实账号、不发送真实消息，也不发布 PC 工件。
真实 ChatGPT/Google 收发、流式、模型/工具菜单、附件、听写、重启缓存和切换时延统一留到
后续现场验收。

## 实现范围

- `pc-frontend/src/features/user-browser/`
- `pc-frontend/src/features/ai/`
- `desktop-shell/src-tauri/src/local_ai_browser*`
- `android/app/src/main/assets/google_web_adapter.js`
- `pc-frontend/scripts/test-local-ai-browser-contract.cjs`
- `scripts/validate-win-web-ai.ps1`
