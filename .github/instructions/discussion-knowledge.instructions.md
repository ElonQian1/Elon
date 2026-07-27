---
applyTo: ".elon/discussion-graph*.json,docs/inbox/conversations/**"
---

# 供应商无关的讨论知识与脑图演化规则

本规则只在任务涉及长聊天导入、讨论拆分、脑图、分叉、节点晋升、历史回看或脑图修正时读取。Codex、Claude、Gemini、Copilot 及其他代理必须使用同一 JSON 与 MCP 契约，不建立供应商私有真源。

## 三层权威

1. `docs/inbox/conversations/*` 是原始聊天来源：`authority=none`、`lifecycle=source_material`、`default_retrieval=false`。
2. `.elon/discussion-graph.json` 是可继续推理的当前讨论图；它保留观点、假设、异议、证据、决定、功能与任务，但不自动成为项目事实。
3. 当前规范、需求、决策或实现文档才指导开发。只有已确认、可追溯且不重复的节点可以晋升；旧分支继续保留。

不得把聊天摘要直接当当前需求，不得因为节点出现在脑图中就把它写成已实现。

## 低 Token 顺序

1. 若原始聊天尚未进入项目，先调用 `project_discussions_import_source`；不得要求用户代替 AI 点击页面，也不得手写成高权威文档。
2. 先 `project_discussions_get_graph` 或 `get_node`，不读聊天正文。
3. 需要回看时用 `get_history`、`get_graph_at_version`、`compare_versions` 或 `trace_node`；禁止为了了解版本重新读取整段聊天。
4. 只有新增或语义不明的来源，才用 `project_docs_plan_context` 后按需 `project_docs_read`；不得全库读取。
5. 保存增量时使用稳定节点 ID、来源锚点和明确的 `change_kind`，调用 `save_proposal`。
6. 修改前调用 `review_graph`。`prepare_safe_repair` 只处理可无歧义修正的结构问题；采纳、否决、合并和权威性由 AI 根据命中来源形成 proposal。
7. `apply` 后回读当前图、历史和审查结果。每次应用都必须创建新版本，不能重写旧 Git 历史。

所有元数据查询必须报告 `chat_bodies_read=0`；首次理解新聊天会消耗模型 Token，但只读本次来源。后续围绕稳定节点工作不应重复读取原聊天。

## 演化语义

- 父子关系表示“从哪个问题展开”；交叉边使用 `supports`、`opposes`、`alternative_to`、`depends_on`、`answers`、`spawns`、`leads_to`、`resolves`、`merged_into`、`supersedes`、`implements` 等明确关系。
- 不删除有意义的失败方案、反对意见或旧决定。用 `rejected`、`superseded`、`merged_into` 和关系边说明它为何退出当前路线。
- 每次 proposal 用 `change_kind=import|expand|refine|decision|implementation|review|repair|merge`，并填写简短 `summary` 和 `actor`。
- `accepted` 或 `implemented` 的决策、需求、功能、任务和结果必须有相称的 authority、来源或实现引用；未解决风险不能静默消失。
- 节点晋升为 Markdown 时应带 owner、复查日期、来源和实现引用；禁止覆盖已有不同内容。

## 失败与降级

- revision 冲突时刷新当前图与建议，重新合并，不覆盖其他会话。
- MCP 不可用时仍遵守 `.elon/discussion-graph.json` 和 `.elon/discussion-graph-suggestions.json` 契约，在 Git 备份后写入；不得改用某个供应商的聊天记忆作为共享事实。
- 程序审查只能自动修正确定性结构错误；语义判断需要来源。审查无问题只代表规则未命中，不代表产品判断必然正确。

详细模型、PC 交互和可迁移边界见 `docs/discussion-knowledge-compiler.md`。
