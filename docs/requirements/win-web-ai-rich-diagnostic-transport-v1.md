---
status: current
reviewed_at: 2026-08-23
---

# Win 网页 AI 富结构诊断传输 V1

## 目标

把 Tauri 已生成的富内容类型计数，通过 Windows 节点控制回执的第二层脱敏白名单完整传递给本机验收工具，以便不读取用户正文、URL 或凭证也能定位富内容丢失阶段。

## 验收标准

1. 节点回执只接受固定的内容部件类型和富卡 kind，并对每个计数设置上限；未知键、正文、URL、域名和凭证字段全部丢弃。
2. `content_part_counts`、`rich_card_kind_counts`、`citation_count`、`linked_citation_count` 与 `citation_logo_count` 可从生产 `list_ai_windows` 回执读取。
3. Rust 单元测试覆盖正常计数、未知结构、超限计数和秘密字段不泄漏。
4. 定向节点控制 Rust 验证、Win Web AI 全量验证、正式 Win 发布及重启后实机回读通过。

## 非目标

- 不传输回答正文、标题、来源 URL、域名、Cookie、token、请求头、owner 指纹或原始厂商响应。
- 不修改 PWA，不新增聊天 UI，不开启任何私有响应观察器。
