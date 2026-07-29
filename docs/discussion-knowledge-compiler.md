---
owner: project-docs
reviewed_at: 2026-07-27
role: architecture
lifecycle: active
authority: authoritative
default_retrieval: false
implementation_refs:
  - file:server/src/project_discussion_graph_model.rs
  - file:server/src/project_discussion_source_import.rs
  - file:server/src/project_discussion_source_chunks.rs
  - file:server/src/project_discussion_source_registration.rs
  - file:server/src/node_agent_project_docs_mcp_discussion_tools.rs
  - file:server/src/project_discussion_graph_history.rs
  - file:server/src/project_discussion_graph_review.rs
  - file:pc-frontend/src/features/project-docs/ProjectDocumentDiscussionMap.tsx
  - file:pc-frontend/src/features/project-docs/ProjectDocumentDiscussionProposalPanel.tsx
  - file:pc-frontend/src/features/project-docs/ProjectDocumentDiscussionSources.tsx
  - file:pc-frontend/src/features/project-docs/ProjectDocumentDiscussionTimeline.tsx
---

# 讨论知识编译器

本文定义如何把 ChatGPT、Codex 或其他供应商的长聊天，整理成可继续讨论、可分叉、可追溯、可晋升的项目知识。它是项目文档治理的按需架构说明，不是所有 AI 任务的必读规则。

## 目标

长聊天同时混有事实、想法、反对意见、临时假设和最终决定。直接把整段聊天放进当前知识会造成三个问题：

- 每次检索都要消耗大量 token；
- 早期想法容易冒充当前事实；
- 后续只能继续整段聊天，无法围绕一个功能或分支独立推进。

系统因此保留两层内容：

```text
原始聊天来源（低权威、默认不检索）
                 ↓ 编译
讨论推理图（节点、父子、交叉关系和来源锚点）
                 ↓ 人或 AI 确认
正式项目文档 / 功能图 / 实现任务
```

原始聊天用于追溯，讨论图用于思考，正式文档用于指导项目。三者不能混为一个权威层。

## 可复用拆分模型

每段聊天可拆为以下节点类型：

| 类型 | 用途 |
|---|---|
| `topic` | 一个可以独立展开的主题 |
| `question` | 尚待回答的问题 |
| `claim` / `hypothesis` | 主张或待验证假设 |
| `option` / `objection` / `risk` | 方案、反对意见和风险 |
| `evidence` | 支持或反驳观点的可追溯依据 |
| `decision` | 已明确采用的决定 |
| `requirement` / `feature` / `task` | 可进入产品与开发流程的节点 |
| `result` | 已验证结果 |

父子关系表达“这段讨论由哪里展开”，交叉边表达 `supports`、`opposes`、`alternative_to`、`depends_on`、`answers`、`spawns`、`leads_to`、`resolves`、`implements` 等语义。新分支不得覆盖旧节点；被否决或替代的节点仍保留状态和来源。

## 存储契约

- `.elon/discussion-graph.json`：当前讨论图真源。
- `.elon/discussion-graph-suggestions.json`：待审核或待应用的增量建议。
- `docs/inbox/conversations/*.md`：用户导入的原始聊天，frontmatter 固定为 `role=discussion`、`lifecycle=source_material`、`authority=none`、`default_retrieval=false`。
- `docs/**.md`：只有经过确认的节点才能晋升到正式文档。晋升不得覆盖已有不同内容。

讨论来源还保存 `content_revision`、格式、消息数、总 chunk、已处理 chunk 和 `pending|partial|complete`。讨论节点必须保留 `source_id#turn-xxxx` 形式的 `source_refs`；可选保存平台任务或会话 ID 到 `conversation_refs`，并通过 `document_paths`、`feature_node_ids` 连接正式文档和产品功能图。讨论图最多 512 个来源、4096 个节点、8192 条边；PC 页面按根主题和搜索条件显示局部图，默认最多 400 个节点。

图中的 `evolution` 保存本次变更的 `kind`、摘要、执行者、时间和前一 revision。它用于给不同 Git 实现提供相同的人类可读版本语义，不代替 Git 提交，也不允许客户端覆盖历史版本。稳定节点 ID 是跨版本追踪的身份；标题可以改进，ID 不随标题变化。

## 低 token 编译流程

1. PC 页面通过本机 loopback 入口导入文件；没有页面交互时，任意 AI 供应商直接调用 `project_discussions_import_source`。两条路线复用同一 Rust 实现：规范化 ChatGPT 等 JSON 为稳定 turn 锚点，固定低权重 frontmatter，不覆盖已有来源，立即把来源登记为 `pending`，并为普通 Git 项目或托管笔记库创建导入前后版本。
2. Windows 节点给当前登录账号选择的 Codex、Copilot、Claude 或 Gemini CLI 注入同一项目文档 MCP；普通开发任务不加载这组工具。
3. AI 先调用 `project_discussions_get_graph` 读取已有节点和 revision，不读聊天正文。
4. AI 调用 `project_discussions_get_source_manifest` 获取稳定 chunk 清单，对照图中 `processed_chunk_ids`，只用 `project_discussions_read_source_chunk` 顺序读取未处理 chunk；普通 `project_docs_read` 不用于聊天全文。
5. AI 生成增量节点、边和可选晋升项，调用 `project_discussions_save_proposal`；每轮同时更新来源 revision、总 chunk、已处理 chunk 和编译状态。`expected_source_revision` 必须原样取自刚读取的 manifest；除 `topic` 外，每个新增或修改节点必须用 1 至 3 句话填写可复用摘要，状态只能使用 `open|exploring|accepted|rejected|superseded|implemented`。任务中断时保存 `partial`，下次从首个未处理 chunk 继续；全部处理后才能标记 `complete`。校验器拒绝摘要缺失、非法状态、进度越界、错误 complete、循环父子关系、无效路径和越界晋升。
6. 修改前调用 `project_discussions_review_graph`。程序只修正 root 等确定性错误；权威性、采纳状态、重复观点和未解决异议必须根据命中来源判断。
7. 根据统一权限模式调用 `project_discussions_apply`。所有可应用模式都为普通 Git 项目保存整理前和整理后提交；托管笔记库创建对应版本。晋升项声明的 `section_id`（未声明时使用来源节点的 `section_id`）若命中现有知识主题，系统会同时写入 `.elon/document-sections.json`，避免“生成了文档却在知识架构里找不到”；已有人工主题分配不会被覆盖。授权模式控制“能否应用”，不再控制“是否保留历史”。
8. PC 页面刷新讨论图。用户可查看每个来源的进度并续编；从任意节点继续或分叉时，用户新增文字会先作为新的低权重来源进入 Git，再由 AI 增量编译，不允许 AI 凭按钮文案猜测新需求。晋升时用户可指定文档类型和可选路径。
9. 应用后调用 `project_discussions_get_history` 和 `project_discussions_review_graph`；需要解释变化时用语义比较，不重读原聊天。

`classification_model_tokens=0` 只代表读取现有讨论图和目录不调用模型。首次理解聊天正文仍会消耗模型 token，但只读当前来源；后续围绕节点工作不需要重复读取整段聊天。

`project_discussions_import_source` 是 MCP 原生入口，不要求 AI 操作网页或直接拼写 frontmatter。相同标题、正文和建议文件名重复导入时返回已有来源并保留已处理进度；同名不同内容自动分配新文件名，永不覆盖旧聊天。即使后续 AI 任务断电或失败，`pending` 来源仍已在讨论图和 Git 中，PC 页面可以发现并续编。

## 版本与演化回看

讨论图不是一张不断覆盖的图片，而是 Git 快照上的持续演化模型：

| 能力 | 工具 | 返回内容 |
|---|---|---|
| 版本时间轴 | `project_discussions_get_history` | 每版摘要、变更类型、节点/关系/来源变化数量 |
| 旧版回看 | `project_discussions_get_graph_at_version` | 与当前图相同的分页、根主题和搜索投影 |
| 语义对比 | `project_discussions_compare_versions` | 节点状态/父级/内容及边、来源的增删改 |
| 节点生命线 | `project_discussions_trace_node` | 创建、更新、状态迁移、父级和关系变化 |

这些工具读取 `.elon` Git 快照，不读取原始聊天或 Markdown 正文。旧版本只读；恢复或修正必须创建新提交，不能 reset 或改写历史。被否决和被替代的节点仍留在当前图时，可直接看关系；即使后来确需移出当前图，历史快照仍能回看。

PC 页面以横向时间轴展示版本。选择旧版后，继续讨论、分叉、导入和晋升按钮进入只读状态；“与当前比较”展示语义数量，不展示难以理解的原始 JSON patch。选中节点可打开生命周期时间线。

## 质量审查与修正版

`project_discussions_review_graph` 用零模型 Token 检查：

- 来源缺失、节点无来源、摘要缺失；
- 根主题不一致、同主题重复标题；
- `accepted` / `implemented` 与 authority 冲突；
- 已实现节点缺少文档或功能实现引用；
- 已采纳决策仍有开放异议或风险；
- `superseded` 节点没有指向后继；
- 关联来源或文档路径失效。

审查结果区分 `error`、`warning`、`advice`，并给出稳定 issue ID、证据节点、建议动作和 `auto_fixable`。`project_discussions_prepare_safe_repair` 只生成确定性修正 proposal，不直接写图。PC 端“让 Win 端 AI 修正并发布新版本”会让登录账号的 AI：

```text
review_graph
  → prepare_safe_repair（若有）
  → 只读取语义问题命中的来源
  → save_proposal(change_kind=repair)
  → apply
  → get_history + compare_versions + review_graph
```

这里的“发布新版本”是发布新的脑图文档版本，不等于每次发布一龙 PC/节点二进制。产品代码只有在功能本身升级时才需要统一构建和发布。

## 晋升规则

讨论节点满足以下条件后才适合晋升：

- 状态为 `accepted` 或 `implemented`，而不是 `open` 或 `exploring`；
- 事实性结论有来源或实现证据；
- 目标文档的用途明确，是需求、决策、架构、功能说明、任务或结果；
- 项目不存在同一主题的当前权威文档，或者新节点只建立关联而不另建重复文档；
- 假设、被否决方案和历史分支不会进入默认检索。

“晋升为文档”不是把聊天摘要复制进 `docs/`。它应生成可维护的当前结论，保留来源链接、状态、owner、复查日期和实现引用。

## PC 交互

OneNote 式文档侧边栏中的“讨论推理”是独立虚拟分区：

- 导入 `.md`、`.txt` 或 `.json` 聊天；
- 按根主题切换脑图，搜索观点、证据、功能和任务；
- 点击节点查看来源、关联会话、正式文档和功能节点；
- “继续讨论”沿原分支补证据和下一步；
- “创建备选分支”保留原观点并新增分支；
- “晋升为正式文档”先评估稳定性和重复文档，不强制创建。
- 顶部来源条展示 `processed/total chunks`；未完成来源可从断点续编；
- 待处理建议先展示新增/变更节点、关系、来源进度和拟晋升文档，用户确认后才发起应用；
- 版本时间轴可回看过去脑图、与当前版比较，并查看单节点如何发展到今天；
- 质量面板显示确定性问题，并可让当前 Windows 登录账号选择的 AI CLI 生成修正版。

UI 只负责收集用户输入、展示和发起带范围的 AI 任务。导入、继续和分叉都会先调用本机节点的统一来源/Git 事务；没有 UI 时，AI 可通过 `project_discussions_import_source` 完成同一操作。拆分、校验、revision、权限、Git 备份和应用均由 Windows 节点上的供应商无关 MCP 完成，网页不另建一套整理算法。

## 可迁移性

这套机制不依赖一龙项目的业务分类。其他项目只需保留同一 JSON schema、来源路径约定和 MCP 工具语义，即可使用自己的主题树、功能图和文档模板。平台托管笔记场景在后台使用用户不可见的 Git 仓库，因此用户无需理解 Git，也能获得导入前原文、整理结果、差异和恢复能力。

平台创建的新用户项目会得到同一套轻量入口：`AGENTS.md` 与各供应商桥接文件只指向共享规则，聊天/脑图任务才按需读取 `.github/instructions/discussion-knowledge.instructions.md`。因此普通开发不承担讨论工具说明的常驻 Token；换 Codex、Claude、Gemini、Copilot 或其他实现也不会改变文档真源。
