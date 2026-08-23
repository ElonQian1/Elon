---
status: current
reviewed_at: 2026-08-23
---

# Win 网页 AI 富内容运行时门禁 V1

## 目标

在 Rust 白名单清洗之后，为 PC 原生聊天渲染再增加一层轻量、版本感知的运行时门禁。DPAPI 旧缓存、上游结构漂移或异常本机状态中的未知/残缺富卡不得造成空卡、React 渲染异常或黑屏；正文和公开来源仍应正常展示。

## 范围

- 仅修改 Win/PC 原生聊天的 `yilong.rich-content.v1` TypeScript 协议、映射策略和契约测试；不修改 PWA。
- 复用现有 Rust sanitizer 与 `AiRichContentCard`，不建立第二套清洗器或聊天 UI。
- 运行时门禁只验证 schema、kind、source、必要字段、数组上限、公开 HTTPS 媒体地址和有限数值；不改写已清洗内容。
- 未知 schema/kind/source、缺少必要字段、非有限图表点、空天气行、无效媒体和超限集合失败关闭，仅丢弃对应 `rich_card`。

## 验收标准

1. `isYilongRichContent` 能识别 finance、weather、media_gallery、map 四类合法 V1 富卡。
2. 未知版本、未知 kind/source、残缺 payload、NaN/Infinity 图表点、HTTP/带凭据媒体 URL 和超限集合返回 false。
3. Web AI 消息映射只把通过门禁的富卡交给原生 renderer；坏富卡不会生成空卡，但同消息正文与 citation 保留。
4. `AiStructuredContent` 在组件边界再次失败关闭，防止其他调用方绕过映射层。
5. 合成契约执行真实 TypeScript 门禁逻辑并覆盖合法/异常样本；完整 Win Web AI、typecheck、lint、build 与源码体积门禁通过。

## 非目标

- 不重复 Rust 的文本清洗、URL 归一化或生产授权判断。
- 不读取私有响应，不改变官网回答节点优先策略，不伪造缺失富内容。
