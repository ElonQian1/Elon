---
version_status: current
requirement_status: implemented
reviewed_at: 2026-08-17
---

# Win AI 官网原生页与来源标签 V1

## 目标

Windows 生产首页继续保留一龙统一聊天，同时允许用户在同一主窗口切换到 ChatGPT 或
Google AI Mode 的官方原生页面。天气、地图、Logo、图片、公式、表格、交互卡片等复杂内容
以厂商官方 WebView2 页面为完整呈现真源，不要求一龙重新实现厂商全部私有组件。

## 本轮范围

- 生产 AI 首页增加“聊天 / 厂商官方页 / 来源网页”标签，不恢复独立测试聊天窗。
- 官方页复用现有 owner/provider WebView2 Profile，在后台、弹出窗口和主窗口标签之间迁移，
  不复制 Cookie，不创建第二份登录会话。
- 回答公开来源同时提供“一龙内部标签”和“系统浏览器”两个入口。
- 通用来源页只接受无凭据 HTTPS 地址，使用单个可复用的临时隔离 WebView，不共享 AI 官网
  Profile；弹窗继续交给系统浏览器。
- 内部标签提供返回、刷新、官网首页、系统浏览器和关闭控制，并随主窗口布局调整尺寸。

## 非目标

- 不实现地址栏、书签、下载中心、扩展、密码管理或无限标签等完整浏览器能力。
- 不读取、复制或持久化厂商 DOM/HTML、Cookie、Token、请求头和私有接口响应。
- 不承诺一龙语义消息层像素级复刻厂商卡片；需要完整视觉和交互时切换官方页。
- 不修改 PWA 或 Android 路线。

## 安全与生命周期

- 官方页仍由固定厂商白名单、原有 Profile 隔离和语义 capability 约束。
- 通用来源页由 Rust 再次验证 HTTPS、无 userinfo、正常端口与可导航地址。
- 主窗口关闭或切回聊天时隐藏子 WebView；恢复官方弹窗时把同一个 WebView 重新挂回原窗口。
- 通用来源标签有界为一个，关闭即销毁，防止后台无限创建 WebView 和内存增长。
- IPC 仅向 `main` WebView 开放；超时或不支持时提示使用系统浏览器，不伪造成功。

## 验收矩阵

| 场景 | 预期 |
|---|---|
| ChatGPT / Google 官方页 | 在生产首页内部标签显示官网原生内容，复用原登录/游客会话 |
| 天气、地图、Logo、复杂卡片 | 官方页按厂商页面原生呈现；统一聊天保留安全语义降级 |
| 公开来源 | 可选一龙临时标签或系统默认浏览器 |
| 切回聊天再返回官网 | 同一 WebView 恢复，不新建 owner/provider Profile |
| 来源网页弹窗 | 拒绝内嵌弹窗并交给系统浏览器 |
| 旧 Windows 壳 | 显示客户端升级/系统浏览器降级，不影响现有聊天 |
| PWA / Android | 行为和代码路径不变 |

## 验证入口

- `npm --prefix pc-frontend run test:user-browser`
- `npm --prefix pc-frontend run build`
- `npm --prefix pc-frontend run lint`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/validate-win-web-ai.ps1`
