---
applyTo: "**/*.md"
---

# 项目文档权威性与低 Token 检索

本规则供 Codex、Claude、Gemini、Copilot 及其它 AI 代理共同使用。任务涉及文档、笔记整理、需求追溯、规则冲突或知识检索时才读取。

## 路径权威上限

| 路径或类型 | 权威性 | 默认检索 |
|---|---|---|
| `AGENTS.md` | 所有 AI 的共享入口与路由 | 是 |
| `.github/copilot-instructions.md` | 仓库通用规则权威来源 | 是 |
| `.github/instructions/*.instructions.md` | 领域规则 | 任务命中时 |
| 当前规范、架构、需求、运行手册 | 对所属领域有效 | 按需 |
| `decisions/`、ADR | 已接受决定及原因 | 按需 |
| `drafts/`、`inbox/`、讨论、想法 | 未批准材料 | 否 |
| `reports/`、测试报告、交付证据 | 只能证明结果，不能定义需求 | 否 |
| `archive/`、`historical/`、旧讨论稿 | 历史材料 | 否 |
| 无法判断的 Markdown | `unclassified` | 否 |

正文或 frontmatter 可以把自身降为 `draft`、`deprecated`、`superseded` 或 `archived`，但不能突破路径权威上限。归档文件不能通过自称“权威”恢复为当前规则。

## 检索与冲突

1. 先由程序读取路径、标题、大小、哈希、标题层级和生命周期，不把全文送给模型。
2. 只加载当前任务命中的必须文档、领域指令和源码入口。
3. 只有 `ambiguous` 或冲突文档才按单篇读取；先目录，仍不足再读全文。
4. 草稿、报告、归档和未知文档默认排除；历史追溯时显式标注身份。
5. 冲突优先级：用户当前要求 > 通用规则 > 领域规则 > 当前规范/架构/需求 > 已接受决策 > 草稿/报告/历史材料。
6. 同级冲突报告 `DOC_CONFLICT`，不要静默任选一份。

AI 首轮只根据路径和元数据形成分类、冲突和迁移建议，不自动提升权威性；无法确认的内容留在 `unclassified`。报告实际读取的文档数、估算 token 和默认排除范围。

## 分区和 AI 建议

- `.elon/document-sections.json` 保存所有 AI 供应商共享的项目知识架构：项目类型 `profile`、知识首页 `home`、最多四层主题 `sections`、主题固定项 `assignments`、治理覆盖 `governance_overrides`、文档关系/共享顺序/固定状态 `document_metadata`、功能/技术节点与证据引用 `knowledge_graph` 和最近 100 条结构操作 `audit_log`；不改变文件实际路径，也不复制正文。
- 主题知识树回答“讲什么”，治理视图回答“能否作为当前事实”。两条轴必须分离；主题位置不能提高路径权威性。PC 工作台分别展示“知识架构”和“治理视图”。
- 项目模板可选软件平台、API/SDK、产品、研究、运维或个人知识库；模板是起点，不要求所有项目采用同一分区。路径和标题能确定主题时由程序自动归类，关键入口再显式固定。
- 个人按名称、数量、路径或权威性的查看排序只保存在浏览器；手工分区顺序、文档固定/顺序、入口和归类才写共享清单并记入审计。改变父级必须拒绝循环和第五层，治理覆盖不能突破真实路径权威上限。
- `.elon/document-organization-suggestions.json` 保存 AI 的结构化整理建议；AI 整理任务只可写这一份建议文件。
- 功能图回答“用户能做什么”，技术架构图回答“系统怎样实现”，主题树回答“文档讲什么”，治理视图回答“能否作为当前事实”。四个维度必须分开；有文档不等于功能已实现，功能和组件应关联 `file:`、`route:`、`symbol:` 或 `test:` 证据。
- 大型仓库可用 `.elon/knowledge-federation.json` 声明项目根、子项目和模块节点；先选择命中任务的 `scope_id`，再在该节点内分页。
- 当前入口和高权威文档应维护 `owner`/`owners`、`reviewed_at`、`review_interval_days`；用显式 `implementation_refs` 让程序先定位实现证据。
- 发起整理任务前不得在基线工作区预创建建议占位文件；建议 JSON 只能由隔离 AI 任务产出并进入正常 Git 收尾。
- 默认 `git_backed_full`：先提交整理前原始文档，再自动应用新分区、虚拟归类及结构化建议中的项目内 Markdown 重命名/移动，最后提交整理结果。
- 用户可切换 `review_all`（逐项审核）或 `suggestions_only`（只生成建议）；所有 AI 供应商使用同名模式。
- `review_all` 才要求用户逐项核对并授予本次 `rename`/`move`；`suggestions_only` 禁止应用。
- 实体整理只允许项目内 Markdown，必须校验源文件哈希，禁止覆盖、删除、越界、改写代码或自动 push。当前分类操作不改正文；正文修改、引用重写、归档和删除未来也必须纳入同一整理前/后 Git 事务。

## 供应商无关 MCP 顺序

1. `project_docs_analyze` 只读路径和元数据，并返回零模型 token 的服务端统一 `document_health`；大型项目优先传 `scope_id`。
2. 需要理解项目时，`project_docs_get_map` 先取 overview，再只查任务命中的 capabilities、architecture 或 topics；单节点用 `project_docs_get_node`，结构评审用 `project_docs_review_map`。
3. `project_docs_get_issues` 按页返回链接、孤立文档、owner/复查和实现引用问题；发现问题不需要先读正文。
4. 先用 `project_docs_plan_context` 在 token 预算内规划阅读；只对歧义、冲突或缺失入口调用 `project_docs_read`，不得全库读取正文。
5. `project_docs_save_suggestions` 保存项目类型、首页、层级主题、文档关系、`proposed_knowledge_graph`、归类和实体操作建议。
6. 默认 `git_backed_full` 继续应用建议与文件操作，并确认整理前、整理后两个 Git commit；`review_all` 等用户确认，`suggestions_only` 禁止应用。
7. MCP 不可用时仍使用相同两份 `.elon` JSON，不建立供应商私有真源。

个人笔记等非 Git 场景由平台用 `vaultId` 创建隐藏托管 Git 库；AI 可通过历史与恢复工具操作版本，用户无需理解 Git。
