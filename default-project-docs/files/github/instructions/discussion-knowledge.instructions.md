---
applyTo: ".elon/discussion-graph*.json,docs/inbox/conversations/**"
---

# 供应商无关的讨论知识与脑图演化

本规则只在任务涉及长聊天导入、讨论拆分、脑图、分叉、节点晋升、历史回看或脑图修正时读取。Codex、Claude、Gemini、Copilot 及其他代理必须使用同一 JSON 与 MCP 契约。

## 权威边界

- `docs/inbox/conversations/*` 是无权威、默认不检索的原始来源。
- `.elon/discussion-graph.json` 是可继续推理的讨论图，不自动成为当前需求或实现事实。
- 只有已确认、可追溯且不重复的节点才可晋升为当前规范、决策、需求或实现文档。
- 旧方案、异议和失败分支不删除；用状态与 `supersedes`、`merged_into`、`resolves` 等关系保留演化原因。

## 低 Token MCP 顺序

1. 先 `project_discussions_get_graph` 或 `get_node`，不读聊天正文。
2. 回看演化用 `get_history`、`get_graph_at_version`、`compare_versions`、`trace_node`，不重复读取原聊天。
3. 新来源先用 `project_docs_plan_context`，只对当前来源调用 `project_docs_read`。
4. 修改前调用 `review_graph`。`prepare_safe_repair` 只处理无歧义结构错误；语义问题必须按命中来源判断。
5. `save_proposal` 使用稳定节点 ID、来源锚点、简短 `actor`，并设置 `change_kind=import|expand|refine|decision|implementation|review|repair|merge`。
6. `apply` 后回读当前图、历史、语义差异和审查结果。每次应用都创建新版本，不能重写旧 Git 历史。

元数据工具应报告 `chat_bodies_read=0`。首次理解新聊天会使用模型 Token，但只读本次来源；后续围绕稳定节点工作不重复读取整段聊天。

MCP 不可用时仍使用 `.elon/discussion-graph.json` 和 `.elon/discussion-graph-suggestions.json`，先做 Git 备份再写入；不得把某个供应商的私有聊天记忆当共享真源。
