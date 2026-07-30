---
version_status: current
reviewed_at: 2026-07-30
---

# 一龙项目当前事实

本文件是 Codex、Copilot、Claude、Gemini 和平台内 AI 的短事实快照。开始实现或判断“项目现在有什么”时先读本文件；详细架构、接口和历史原因再按链接读取。讨论、草稿和 Git 历史不能覆盖这里及正式决策。

## 当前产品主链

- 已实现：用户通过 Android APK 或 PC 工作台描述需求，AI CLI/API 代理在真实 Git 工作区开发、验证、构建和发布应用。
- 已实现：项目、频道、多人/多 AI 协作、Windows 节点执行、上下文编译、项目文档治理与版本恢复等基础能力。
- 已实现：开放商业网络 V1 的商户节点、能力、授权、调用、计量和审计主干；HTTP 与 MCP 共用领域服务。
- 已实现但边界有限：API Token 保管、远程节点和计算使用台账。它们目前不是开放算力或 Token 交易市场。

## 已接受的产品方向

- 先用 AI 应用开发、ERP、营销内容、小游戏和经营分析帮助商户获得直接价值，再逐步形成开放商业网络。
- 商户数据由商户控制；消费者 AI、商户 AI 和第三方应用通过授权接口连接，不把所有参与者锁进同一个 App。
- 开放商业当前权威入口是 `docs/open-commerce/README.md`；实现状态先看 `docs/open-commerce/capability-baseline.md`。

## 尚未成为当前事实

- 消费者 AI 跨 App 公共发现网络、闲置算力公开市场、自动收费与跨主体结算仍在路线图或提案阶段。
- Sui 的 NET、CREDIT、RevenuePosition、链上治理和收入权益均是草案；不得表述为已上线、已发币或公司股权。
- 任何提案只有经过正式 ADR、实现引用和验收后，才能进入本文件的“已实现”部分。

## 明确禁用

- 不设置独立“预言家 AI / Demo Oracle / DemoPreview”前置角色。见 `docs/decisions/reject-demo-oracle-role.md`。
- 不恢复 AI-to-AI Skill 路线或 Skill 市场作为当前主架构。多 AI 协作继续使用项目频道、Matter/Assignment、运行路线和供应商工具协议。见 `docs/decisions/reject-ai-to-ai-skill-route.md`。
- Codex 桌面监督与 PC 自动续跑自 2026-07-26 起暂停；暂停解除前不得自动派发或恢复该流程。

## 读取顺序

1. 当前状态：本文件。
2. 项目定位：`AI_PROJECT.md`。
3. 系统分层：`AI_ARCHITECTURE.md`。
4. 实现入口：`AI_INDEX.md`。
5. 开放商业专题：`docs/open-commerce/README.md`。
