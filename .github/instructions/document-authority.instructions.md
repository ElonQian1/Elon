---
applyTo: "**/*.md"
---

# 项目文档权威性与低 Token 检索

本规则供 Codex、Claude、Gemini、Copilot 及其它 AI 代理共同使用。只有任务涉及项目文档、笔记整理、需求追溯、规则冲突或知识检索时才读取；不要复制到各供应商桥接文件。

## 先按路径确定权威上限

| 路径或类型 | 权威性 | 默认检索 |
|---|---|---|
| `AGENTS.md` | 共享入口与路由，不承载详细规则 | 是 |
| `.github/copilot-instructions.md` | 仓库通用规则权威来源 | 是 |
| `.github/instructions/*.instructions.md` | 领域规则，只在任务命中时生效 | 按需 |
| 当前架构、规范、需求、运行手册 | 对所属领域有效 | 按需 |
| `decisions/`、ADR | 说明已接受决定及原因，不覆盖更高层规则 | 按需 |
| `drafts/`、`inbox/`、讨论、想法 | 未批准材料 | 否 |
| `reports/`、测试报告、交付证据 | 只能证明结果，不能定义需求 | 否 |
| `archive/`、`historical/`、带旧日期的讨论稿 | 历史材料 | 否 |
| 无法判断的 Markdown | `unclassified`，不得假设为当前真相 | 否 |

文件正文或 frontmatter 可以降低自身状态，例如标记 `draft`、`deprecated`、`superseded`、`archived`，但不能突破路径的权威上限。归档文件不能通过自称“权威”恢复为当前规则。

## 低 Token 检索顺序

1. 先用程序读取路径、标题、大小、哈希、标题层级和生命周期，不把全文送给模型。
2. 只加载当前任务命中的必须文档、领域指令和源码入口。
3. 只有分类为 `ambiguous` 或发生冲突时，才读取单篇正文；先标题与目录，仍不足再读全文。
4. 默认排除草稿、报告、归档和未知文档。用户明确要求历史追溯时才纳入，并标注其历史身份。
5. 不为了“了解项目”全量读取 `docs/`、聊天记录、Prompt、Agent 或 Skill。

## 冲突与整理边界

- 冲突优先级：用户当前要求 > 仓库通用规则 > 命中的领域规则 > 当前规范/架构/需求 > 已接受决策 > 草稿/报告/历史材料。
- 同级冲突时报告 `DOC_CONFLICT`，列出路径、冲突点和建议权威来源；不要静默任选一份。
- AI 首轮整理只提出分类、冲突和迁移建议，不自动提升权威性，不移动、删除或改写文档。
- 任何迁移必须保留 Git 历史；不能确认时留在 `unclassified`，等待用户批准。
- 报告实际读取的文档数、估算 token，以及未读取或默认排除的范围。

## 分区和 AI 建议的可移植约定

- `.elon/document-sections.json` 是项目共享的虚拟分区和文档归类配置；它不改变文件实际路径。
- 虚拟分区是 OneNote 式浏览维度，不是第二套权威性。把扁平文档审核归入 `current` 或自定义主题后，其 `role`、`lifecycle`、`authority` 和 `default_retrieval` 仍由真实路径决定；要提高路径权威上限必须另行审核 Git 迁移。
- `.elon/document-organization-suggestions.json` 是 AI 整理建议的结构化产物；AI 整理任务只可写这一份建议文件。
- 发起整理任务前不得在基线工作区预创建建议占位文件；建议 JSON 只能由隔离 AI 任务产出并进入正常 Git 收尾。
- AI 可建议新分区和虚拟归类，但必须经用户审核后才更新分区配置。
- AI 可在建议中提供结构化 `file_operations`，用于改善含糊文件名或错误目录；每项必须带 analyze 返回的源文件哈希。
- 应用虚拟分区建议不等于移动 Markdown。实体重命名/移动必须逐项勾选并按本次操作分别授予 `rename`/`move` 权限；只允许项目内 Markdown，禁止覆盖、删除、越界、改写正文或自动提交 Git。
- 修改正文、批量修复引用、归档、删除、commit 和 push 都是更高权限，不能由实体整理授权隐含获得。

## 供应商无关 MCP 顺序

- 当运行环境提供 `project_docs_*` MCP 工具时，先调用 `project_docs_analyze`；它只返回路径和元数据，`classification_model_tokens=0`。
- 诊断或观察整理运行时调用 `project_docs_get_status`；它返回阶段、revision、读取数、token 估算、错误代码和修复建议，不读取 Markdown。
- 只对 `ambiguous` 或当前任务命中的路径调用 `project_docs_read`，不得借 MCP 全量读取 Markdown。
- 模型完成判断后调用 `project_docs_save_suggestions`；该工具只写建议 JSON，并校验目录 revision、真实路径和结构上限。
- 没有用户或审核流程明确确认，不得调用 `project_docs_apply_suggestions`；确认后也只应用虚拟分区，不移动 Markdown。
- 只有用户逐项确认实体操作后才能调用 `project_docs_apply_file_operations`；必须传所选 operation id、`reviewed=true` 和本次允许的 `rename`/`move` 权限。
- MCP 不可用时遵循同一顺序和两份 `.elon` JSON 契约，不得改用供应商私有文档作为第二真源。
- 传输、短期会话和工具契约见 `docs/project-document-governance-mcp.md`；只有接入或诊断 MCP 时才读取。
