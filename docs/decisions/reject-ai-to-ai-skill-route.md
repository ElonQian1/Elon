---
version_status: current
decision_status: accepted
reviewed_at: 2026-07-30
---

# 否决 AI-to-AI Skill 路线

## 决定

项目不把 AI-to-AI Skill、Skill 市场或“由一个 AI 自动选择并调用另一个 AI Skill”作为当前产品主架构，也不恢复已经删除的 `docs/ai-to-ai-skill-*.md` 路线文档。

当前多 AI 协作继续建立在项目频道、Matter/Assignment、明确的运行路线、真实 Git 工作区、MCP/API 工具和可审计任务回执之上。Skill 可以作为单个代理的本地工作说明，但不能成为项目业务状态、代理身份、计费或跨主体信任的真源。

## 原因

- Skill 适合封装工作方法，不适合承担项目任务状态、权限、结算和审计。
- AI 之间隐式选择 Skill 会形成难以解释的第二套路由，与现有频道、Matter 和运行路线重叠。
- Skill 市场会把项目重点从可验证的应用交付和商业能力调用带向尚未证明需求的分发层。
- 各供应商对 Skill 的格式和加载方式不同，不能作为供应商无关协议。

## 当前边界

- Codex、Copilot、Claude、Gemini 可以继续使用各自本地 Skill 辅助执行，但必须遵守共享项目规则和当前业务架构。
- 跨 AI 能力复用通过 MCP/API、结构化任务、权限和回执实现，不通过隐式 Skill-to-Skill 调度实现。
- 若未来重新评估，必须提交新的独立提案，说明相对现有 Matter/MCP 路线的不可替代价值，并取得用户明确批准。

## 对 AI 代理的要求

检索到已删除的 AI-to-AI Skill 路线、旧 Git 版本或旧讨论时，应先读取本决定并标记其为已否决历史。不得将它写回 `AI_CURRENT.md`、当前架构、能力图或默认检索入口。
