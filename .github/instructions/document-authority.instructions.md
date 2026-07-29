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
- AI 首轮只根据路径和元数据形成分类、冲突和迁移建议，不自动提升权威性；是否继续应用由统一权限模式决定。
- 任何迁移必须保留 Git 历史；不能确认时留在 `unclassified`，等待用户批准。
- 报告实际读取的文档数、估算 token，以及未读取或默认排除的范围。

## 分区和 AI 建议的可移植约定

- `.elon/document-sections.json` 是所有 AI 供应商共享的知识架构清单；它包含项目类型 `profile`、知识首页 `home`、最多四层的主题 `sections`、主主题 `assignments`、多副主题 `secondary_assignments`、四维治理 `governance_facets`、兼容快捷视图 `governance_overrides`、有类型语义关系/共享顺序/固定状态 `document_metadata`、功能/技术节点与实现证据引用 `knowledge_graph` 和最近 100 条结构操作 `audit_log`，不改变文件实际路径，也不复制 Markdown 正文。
- 主题知识树回答“文档讲什么”；`retrieval`、`lifecycle`、`authority`、`document_type` 分别回答“是否读取、是否当前、能否作为事实、是什么类型”。这些维度必须分开：同一文档可以有一个主主题和多个副主题，同时是草稿或历史材料；不得把“决策记录、工作区、当前、草稿”等状态伪装成业务主题，主题位置绝不提升路径权威上限。
- 新项目从软件平台、API/SDK、产品、研究、运维或个人知识库模板开始；模板只是可迭代起点，不要求所有项目使用同一分区。程序可根据路径和标题自动归类，关键入口再用 `assignments` 固定。
- PC 工作台的“知识架构”用于项目地图、层级主题和推荐阅读；“治理视图”用于必须、按需、当前、草稿、证据、归档和等待整理。用户和 AI 都可新增主题或子主题，删除父主题时其子树一并移除，但不删除 Markdown。
- 个人查看排序（例如按名称、数量、路径或权威性）不得写入共享清单；项目共同的手工分区顺序、文档固定/顺序、入口和归类才写清单并进入 `audit_log`。改变父级必须拒绝循环和第五层；治理覆盖不得突破真实路径权威上限。
- `.elon/document-organization-suggestions.json` 是 AI 整理建议的结构化产物；AI 整理任务只可写这一份建议文件。
- 功能图回答“用户能做什么”，技术架构图回答“系统怎样实现”，主题树回答“文档讲什么”，治理属性回答“能否作为当前事实”。四个维度必须分开；有文档只证明文档覆盖，不能冒充功能已经实现。功能和技术节点用 `file:`、`route:`、`symbol:`、`test:` 等引用关联证据。
- 大型仓库可用 `.elon/knowledge-federation.json` 声明“项目根 → 子项目 → 模块”节点，并用 `scope_path`、`include_globs`、`exclude_globs` 组合知识范围；先选择命中任务的 `scope_id`，再在该节点内分页，不能用大仓库规模作为全量读取正文的理由。
- 当前入口和高权威文档应在 `document_metadata` 维护 `owner`/`owners`、`reviewed_at`、`review_interval_days`；需要核对实现时使用显式 `implementation_refs`（`file:`、`route:`、`symbol:`），程序先定位证据，AI 再按需语义复核。
- 发起整理任务前不得在基线工作区预创建建议占位文件；建议 JSON 只能由隔离 AI 任务产出并进入正常 Git 收尾。
- 文档整理默认使用 `git_backed_full`：先创建整理前仅文档 Git 提交，再自动创建虚拟分区、应用归类及执行结构化建议中选定的 Markdown 重命名/移动，最后创建整理后仅文档提交。
- 用户可切换 `review_all`（逐项审核）或 `suggestions_only`（只生成建议）；所有供应商必须使用同名模式，不能另建私有权限语义。
- AI 建议可同时包含项目类型、知识首页、层级主题、缺失文档类型、文档关系、`proposed_knowledge_graph` 和结构化 `file_operations`；每个实体操作必须带 analyze 返回的源文件哈希。图谱建议应用后由网页和 MCP 同时消费。
- 应用虚拟分区建议不等于移动 Markdown。`git_backed_full` 只对项目内、建议中明确列出的 Markdown rename/move 开放完全整理权限；始终禁止覆盖、删除、越界、非文档操作、代码改动或自动 push。
- 修改正文、批量修复引用、归档、删除、两次仅文档事务提交之外的 commit，以及任何 push 都是更高权限，不能由实体整理授权隐含获得。

## 供应商无关 MCP 顺序

- 长聊天导入、讨论拆分、脑图分叉、历史回看或修正任务，先按需读取 `.github/instructions/discussion-knowledge.instructions.md`；普通文档分类不加载讨论工具说明。
- 当运行环境提供 `project_docs_*` MCP 工具时，先调用 `project_docs_analyze`；它只返回路径和元数据以及服务端统一 `document_health`，`classification_model_tokens=0`。大型仓库优先传 `scope_id`。
- 需要理解项目时调用 `project_docs_get_map`：先取 `overview`，再按任务只查 `capabilities`、`architecture` 或 `topics` 的局部图；单节点用 `project_docs_get_node`，讨论结构是否合理用 `project_docs_review_map`。
- 在读取正文前先用 `project_docs_plan_context` 按任务、节点和 token 预算生成推荐阅读计划；不得把图谱查询退化为全库正文读取。
- 调用 `project_docs_get_issues` 获取失效链接、孤立文档、owner/复查缺口和实现引用证据；可按严重度、状态、负责人筛选。用 `project_docs_update_issue` 分派负责人、设置期限、解决，或在填写原因后忽略/延期；不要为发现这些问题先读全库正文。
- 用 `project_docs_get_health_history` 查看健康分和可执行问题趋势；健康分必须同时返回组成、权重和贡献，不能只显示一个不可解释的总分。
- 诊断或观察整理运行时调用 `project_docs_get_status`；它返回阶段、revision、读取数、token 估算、错误代码和修复建议，不读取 Markdown。
- 只对 `ambiguous` 或当前任务命中的路径调用 `project_docs_read`，不得借 MCP 全量读取 Markdown。
- 长文档只需核对一个主题时优先调用 `project_docs_read_sections`；整理前调用 `project_docs_review_modularity`，来源材料保留原始记录，正式文档按职责拆分。
- 文档入口、权威性或排序发生变化后调用 `project_docs_test_retrieval`，用真实任务验证应命中和禁止命中的文档，再宣告整理完成。
- 模型完成判断后携带当前 `authorization_mode` 调用 `project_docs_save_suggestions`；该工具只写建议 JSON，并校验目录 revision、真实路径和结构上限。
- `git_backed_full` 下继续调用 `project_docs_apply_suggestions`；如有实体操作，把返回的 `git_baseline_commit` 传给 `project_docs_apply_file_operations` 并执行全部 proposed operation id。成功必须同时确认整理前和整理后提交，不需要 `reviewed` 或 `allow_*`。
- `review_all` 下保存建议后停止，只有用户确认才传 `reviewed=true` 和实体操作对应的 `allow_rename`/`allow_move`；`suggestions_only` 下不得调用任何 apply 工具。
- MCP 不可用时遵循同一顺序和两份 `.elon` JSON 契约，不得改用供应商私有文档作为第二真源。
- 普通 Git 项目和个人笔记都可用 `project_docs_get_history`、`project_docs_get_version_diff` 查看文档版本与差异。普通项目只允许以新提交回滚“仅文档、非合并”的祖先提交；个人笔记通过 `vaultId` 使用隐藏托管 Git 库恢复，用户无需理解 Git。
- 传输、短期会话和工具契约见 `docs/project-document-governance-mcp.md`；只有接入或诊断 MCP 时才读取。
