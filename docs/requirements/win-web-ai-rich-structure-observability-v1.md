---
status: current
reviewed_at: 2026-08-23
---

# Win 网页 AI 富结构观测与引用关联 V1

## 目标

在不改变 PWA、不中断 ChatGPT / Google AI 官方 WebView2 会话的前提下，补齐富内容从官网页面到一龙原生 UI 的可诊断链路，并让正文引用只在存在可验证 DOM 关联或经生产授权的结构化关联时显示为带站点 Logo 的来源标记。

## 范围

- 仅修改 Win/Tauri 网页 AI 适配器、Rust 清洗/诊断、PC 原生聊天渲染和对应测试；不修改 PWA。
- 复用现有 `yilong.rich-content.v1`、`AiSourceMark`、`AiSourceLinks`、`MarkdownContent` 与 WebView Portal，不建立第二套聊天 UI。
- DOM 路线只读取用户在官方页面已可见的回答节点。正文引用必须从同一引用控件的直接链接或 `aria-controls` / `aria-describedby` 目标取得公开 URL；不得按“Reuters”等正文文本猜测来源。
- 结构化响应路线只消费 `yilong.authorized-provider-response.v1`，并继续由逐厂商生产授权清单失败关闭。研究许可、口头说明或本机可读取状态不自动开启生产观察器。

## 验收标准

1. ChatGPT 可把有确定 DOM 关联的非链接引用控件序列化为 Markdown 引用链接；无确定关联时保持原文，不猜测、不伪造 URL。
2. 引用内容部件可携带有界的 `markerText`、`citationId` 与 `groupSize`，经 Rust 白名单、DPAPI 快照和 TypeScript 协议保留；未知、超限和凭证形字段失败关闭。
3. ChatGPT finance 与 Google weather 的已授权结构化 envelope 能映射为现有富内容 AST；未授权时返回空，不产生任何私有请求或原始响应落盘。
4. Win 脱敏诊断只报告消息内容部件类型、富卡 kind、引用关联数量和来源 Logo 可用数量，不报告正文、标题、URL、域名、Cookie、token、请求头或 owner 指纹。
5. 原生 UI 优先复用现有来源徽标和来源面板；有结构化引用关联时正文显示站点 Logo 与 `+N`，无关联时仍可在回答后的来源面板查看公开来源。
6. 合成 fixture 覆盖引用控件关联、finance/weather 授权映射、未知/未授权结构失败关闭和诊断隐私；Win Web AI 契约、PC 构建、lint 与定向 Rust 验证通过。

## 非目标

- 不复制或注入厂商整站 CSS/前端包，不伪造官方图表数据。
- 不把 Cookie、Authorization、Access Token、请求头、原始私有响应或带签名资源写入 Git、日志、诊断、fixture 或云端。
- 不在本功能中新增生产授权清单条目；只有取得可审计的厂商授权记录后才能单独评审和登记。
