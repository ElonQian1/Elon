# 项目文档治理 MCP

最后更新：2026-07-23

本文是接入或诊断项目文档治理 MCP 时才读取的按需手册。日常文档任务先遵循 `.github/instructions/document-authority.instructions.md`，不要把本文复制到 Codex、Claude、Gemini 或 Copilot 的私有桥接文件。

## 1. 知识架构、治理属性和实际目录

- Git 工作区中的 Markdown 和路径是内容真源。
- 路径决定文档权威性上限；frontmatter 和 manifest 元数据只能进一步收窄 `lifecycle`、`authority` 与 `default_retrieval`，不能越过路径上限。`version_status=current|draft|deprecated|superseded|archived` 与正文中的显式状态共同决定版本状态；旧讨论、旧需求和历史报告默认不进入当前知识上下文。
- PC 网页端“知识架构”按业务主题浏览；一个文档有一个主主题和最多 12 个副主题。治理总览把 `retrieval`、`lifecycle`、`authority`、`document_type` 四个维度分开保存和交叉筛选，旧的单选治理分区只是兼容快捷投影。
- PC 网页端“项目图谱”包含三张正交视图：产品功能图回答“用户能做什么”，技术架构图回答“系统怎样实现”，文档主题图回答“文档讲什么”。治理视图继续单独回答“能否作为当前事实”。网页和 MCP 都消费 Rust 后端生成的同一图谱，前端不再从主题树自行猜测功能。
- `.elon/document-sections.json` 保存项目类型、知识首页、层级主题、主主题 `assignments`、副主题 `secondary_assignments`、四维 `governance_facets`、兼容 `governance_overrides`、有类型的文档语义关系、`knowledge_graph` 节点/关系/文档引用/实现证据和最近 100 条结构操作审计，不移动实际文件、不复制 Markdown 正文。
- 主题树只改变 OneNote 式浏览位置，不改变 `role`、`lifecycle`、`authority` 或 `default_retrieval`；AI 检索仍以真实路径元数据为准。
- `.elon/document-organization-suggestions.json` 保存待审核或已应用的 AI 建议，不是当前规则真源。
- `.elon/knowledge-federation.json` 可为大型仓库声明“项目根 → 子项目 → 模块/主题”的知识节点；每个节点可用 `scope_path` 表示主目录、`include_globs` 纳入目录外的模块文档、`exclude_globs` 排除局部材料，并有独立 owner、项目类型、知识首页和健康度，最多 4096 个节点、六层。MCP 和网页端按 `parent_id + cursor/offset + limit` 惰性展开；旧的 16 分区或 500 文档窗口不参与联邦完整性判断。
- “AI 整理建议”是独立虚拟分区；建议进入这里不代表已经采用。

因此同一份项目可以在网页端按 OneNote 分区浏览，同时保留适合 Git、IDE 和所有 AI 供应商读取的普通目录结构。

## 2. 传输和发现

Windows 节点在实际本地管理端口提供标准 Streamable HTTP MCP。端口可能位于 `7799-7819`，接入方必须使用节点状态或平台返回的 `node_admin` 地址，不能写死端口。

项目级 bootstrap：

```http
POST http://127.0.0.1:<node_admin_port>/api/project-docs/mcp/bootstrap
Content-Type: application/json

{"projectRoot":"D:\\path\\to\\git-worktree"}
```

普通项目传 `projectRoot`。个人笔记或非 Git 内容传 `{"vaultId":"user-or-notebook-id"}`；平台会在本机数据目录建立不可见的托管 Git 知识库，用户无需理解 Git，但每次编辑仍有版本和恢复能力。两者必须且只能提供一个。bootstrap 返回：

- `mcp.configPath`：通用默认配置；`mcp.configPaths` 还提供 Codex、Copilot、Claude、Gemini 的会话配置；
- `mcp.sessionId`：项目绑定会话；
- `mcp.operationId`：可关联 PC 页面和 MCP 工具调用的整理运行 ID；
- `mcp.expiresAt`：短期令牌过期时间；
- `mcp.transport=streamable-http`。

任何支持 HTTP MCP 的供应商适配器都应读取这份通用配置。适配器可以把相同 URL 映射到自己的用户级配置格式，但不得复制工具逻辑或另建供应商专属文档真源。

PC 网页端发起的明确文档整理任务带 `<elon-project-docs-task version="1">` 标记。节点启动器据此为 Codex、Copilot、Claude 和 Gemini 分别使用会话级参数或临时设置注入同一个 MCP；普通开发任务不加载这些工具，不承担额外工具 schema token。新增供应商适配器也应识别同一标记并转换 `mcp.configPaths`，不能复制工具逻辑。

## 3. 工具顺序

整理任务携带统一的 `authorization_mode`，所有供应商使用同一语义：

- `git_backed_full`（默认）：先创建“整理前”仅文档 Git 提交，再开放分类、建分区及 Markdown 重命名/移动；完成后创建“整理后”仅文档提交，不需要逐项 `reviewed` 或 `allow_*`。
- `trusted_reversible`（兼容）：不自动创建 Git 提交，但仍可自动应用虚拟分区和安全 Markdown 路径操作。
- `review_all`：应用工具必须传 `reviewed=true`；实体操作还必须分别声明 `allow_rename`、`allow_move`。
- `suggestions_only`：只能分析、按需读取和保存建议，所有应用请求都会失败。

默认开放不等于任意仓库写权限。四种模式都禁止越过 Git 工作区、符号链接逃逸、非文档操作和自动 push；当前整理器的实体操作只包含 rename/move，不覆盖、不删除、不改写正文。catalog、manifest、suggestions 与源文件 revision 校验始终启用。

### `project_docs_analyze`

第一步调用。它尊重 `.gitignore` 扫描最多 20000 份候选 Markdown，但不返回正文；默认每页 80 份，最大 200 份。大型项目可传 `scope_id` 只分析某个联邦节点。所有目录类工具统一支持 `projection=summary|page|detail|full`、`offset/limit/cursor`、`topic` 和按工具定义的 `detail`。`summary` 只返回计数、revision、评分摘要和空集合，不得把全量文档、问题、图节点或建议藏在 `structuredContent` 或文本收据中；响应的 `response_budget.serialized_bytes` 是最终 JSON 的真实 UTF-8 字节数，`estimated_tokens` 使用同一序列化结果估算。输出包含：

- `catalog_revision`；
- 路径、标题、大小、哈希和标题层级；
- `role`、`lifecycle`、`authority`、`default_retrieval`、`ambiguous`；
- 自动治理分区、知识架构健康度、用户覆盖和现有建议；
- 全量读取估算、默认读取估算和预计避免 token。

`classification_model_tokens` 固定为 `0`，表示预分类没有调用模型，不表示后续 AI 判断不消耗 token。`document_health` 是服务端统一真源，包含 `architecture`、`quality`、`maintenance`、`federation` 和 `governance_workflow`；后者提供问题状态、负责人、期限、筛选项、健康趋势和评分组成。PC 网页端与 MCP 不再各算一套分数。

`project_docs_get_health` 提供健康摘要或按 `detail=issues` 分页返回问题；`project_docs_get_federation` 按父节点分页返回直接子节点和 `has_children`，用于项目 → 子项目 → 模块/主题惰性展开。二者和 `analyze` 使用同一目录快照，不用 transport 页大小冒充全库大小。

目录索引保存到工作区外的 SQLite，不污染项目 Git。文件大小和修改时间未变化时复用目录与质量事实；创建、修改、删除形成持久事件。已访问工作区每 60 秒后台重扫，并异步复查外链。

### `project_docs_get_map` / `project_docs_get_node` / `project_docs_review_map`

这组工具让 AI 不依赖网页点击就能真实理解和评估项目图谱：

- `project_docs_get_map(view=overview)` 先返回三张图的来源、结构分、节点统计和根节点；再按任务只查询 `capabilities`、`architecture` 或 `topics`，可用 `root_id`、`depth`、`query`、`max_nodes` 限定局部图。
- 图谱响应的 `identity` 同时返回工作区、规范化工作区、manifest revision 和 knowledge-map revision。PC 页面必须用这些字段识别旧目录快照或错误工作区；项目已有正式节点却收到 `profile_template` 时，应报告数据一致性异常并刷新，不能把模板当作项目事实。
- `project_docs_get_node` 返回单节点的入口、文档路径、六类文档覆盖、实现证据、相邻关系和确定性缺口，不读取正文。
- `project_docs_review_map` 按视图给出结构诊断和评审问题。产品功能不能由文档类别代替，技术组件应与真实进程/部署单元/数据流一致，主题位置不得改变权威性。

图谱响应始终包含 `classification_model_tokens=0`、`markdown_bodies_read=0`。父节点自底向上聚合子节点文档和实现证据，但聚合来源会保留为可解释证据。有文档只证明覆盖，不能据此声称功能已经实现；实现状态单独由 `file:`、`test:` 的存在性和 `route:`、`symbol:` 等声明证据表达。`implementation_declared=0` 时实现分固定为 0，不能靠文档覆盖得到高健康分；总分同时返回文档、实现和 finding 三个组件及公式。
图谱查询复用目录增量索引，但不会为每次节点讨论重复执行链接、owner、复查周期和联邦等完整健康分析；完整健康仍由 `project_docs_analyze` 和 PC 目录快照维护。

### `project_docs_plan_context`

根据自然语言任务或 `node_id`，在 `max_rule_tokens`、`max_tokens` 与 `max_documents` 预算内返回推荐阅读顺序、权威元数据和估算 token。强制规则预算与相关内容预算分别计算；`AGENTS.md` 等最小规则入口不挤占业务正文。系统架构、PC 桌面监督和项目文档治理手册对相关任务显式优先；旧讨论、旧需求、历史报告和 E2E trace 只有任务明确要求时才进入计划。它不返回正文；AI 只有在目录、图谱和标题层级仍不足以判断时，才对计划中的少量路径调用 `project_docs_read`。

### `project_docs_get_issues` / `project_docs_update_issue` / `project_docs_get_health_history`

`project_docs_get_issues` 分页读取确定性质量问题及最小证据，可按类型、严重度、处理状态和负责人筛选。当前覆盖本地链接/标题锚点、缓存的外部链接、孤立文档、重复或冲突标题、缺少 owner、缺少复查日期、复查逾期、显式 `implementation_refs` 不存在，以及文件、测试、route、symbol 或 API 实现晚于文档复查日期。每项都返回证据、严重度和修复建议。孤立判断同时识别 Markdown 链接和反引号路径引用；Agent、Prompt、Skill 等按任务加载的定制资产不强制进入项目知识地图。

`project_docs_update_issue` 把问题设为 `open`、`assigned`、`snoozed`、`ignored` 或 `resolved`。分派必须填写 owner；忽略和延期必须填写 reason；延期还必须填写 `snoozed_until`。恢复日期到期后自动回到待处理。状态保存在工作区外的 SQLite，不污染 Git，但会在下一次分析时与当前问题 fingerprint 对齐。

`project_docs_get_health_history` 返回最近 365 个有变化的健康快照。总分公式和每个组件的权重、得分、贡献都随分析返回，避免用户只看到一个不可解释分数。AI 处理健康问题时应让用户选中明确问题，再把 fingerprint 作为范围，不扩张为全库改写。

### `project_docs_get_status`

读取最近一次整理运行的结构化观测状态，不读取 Markdown，也不写项目文件。返回：

- 当前阶段和从请求、任务发送、MCP 就绪到应用完成的事件时间线；
- catalog、suggestions、manifest revision；
- 目录总数、歧义数、实际正文读取数和估算 token；
- 稳定错误代码、错误详情和对应修复建议。

成功和等待审核是不可被普通刷新降级的终态；新的整理请求会创建新的 `operationId`。

### `project_docs_read`

只读取目录中真实存在且需要判断的路径：

- 一次最多 12 份；
- 单篇默认 6000 字符、最大 24000；
- 总计最多 48000 字符；
- 可传 `expected_catalog_revision` 防止基于过期目录继续判断。

优先读取 `ambiguous`，或与当前需求直接相关的文档。禁止通过分页加批量读取规避预算做全库正文扫描。

### `project_docs_save_suggestions`

模型携带当前 `authorization_mode` 提交 `status=ready` 的结构化建议。服务会验证：

- catalog revision 未变化；
- 建议路径全部存在于目录；
- 系统分区或自定义分区引用有效；
- 新分区不超过 256 个、层级不超过四层，建议归类不超过 20000 条；超限必须显式失败，不能静默截断；
- 当前建议 revision 未被其他会话修改。

该工具只能写建议 JSON，不能写分区配置或 Markdown。

### `project_docs_get_suggestions`

读取当前结构化建议和 revision，不读取正文。

### `project_docs_apply_suggestions`

工具把建议的项目类型、知识首页、层级主题、文档关系、`proposed_knowledge_graph` 和归类合并到知识架构清单，再将建议状态改为 `applied`。默认 `git_backed_full` 会先创建整理前提交；没有实体操作时同时创建整理后提交，有实体操作时返回 `git_baseline_commit` 交给下一工具；`review_all` 必须显式传 `reviewed=true`；`suggestions_only` 禁止调用：

- catalog、manifest 和 suggestions revision 必须一致；
- 重复调用是幂等的；
- 第二步状态写入失败时可安全重试；
- 永远不移动、删除或改写 Markdown。

建议可带有类型化 `section_operations`：新增、重命名、移动、合并和删除分区/子分区。每项必须包含理由与影响；共享顺序写入 manifest，个人显示排序仍只保存在浏览器。网页端和 MCP 复用同一 Rust 校验、revision、权限与回滚语义。

### `project_docs_apply_file_operations`

它对请求中选定的结构化 `file_operations` 执行 Markdown 重命名/移动：

- 请求始终必须带选中的 operation id；默认 `git_backed_full` 自动授予 rename/move，并优先接收上一工具返回的 `git_baseline_commit`；`review_all` 才要求 `reviewed=true` 和对应 `allow_*`；
- 每项使用 analyze 目录里的 `content_hash` 作为 `source_revision`，防止文档变化后仍按旧建议操作；
- 只允许 Git 工作区内的 Markdown，禁止覆盖现有目标、删除文件、越过工作区或改写正文；
- 执行后同步 `.elon/document-sections.json` 中受影响的路径，并把操作标记为 `applied`；
- 不修改正文引用、不自动 push；`git_backed_full` 成功响应必须同时提供 `git_baseline_commit`、`git_result_commit` 和 `git_document_transaction_complete=true`。

AI 只能执行建议文件中列出的 operation id，不能借文档整理修改代码或 push。修改正文、批量修复引用、归档和删除尚未进入低 token 分类操作 schema；未来即使开放，也必须纳入同一整理前/后 Git 事务。

### `project_docs_get_history` / `project_docs_get_version_diff` / `project_docs_restore_version`

这组工具同时支持普通 Git 项目和平台托管知识库。历史最多返回 100 个与 Markdown 或 `.elon` 清单相关的提交；差异最多返回 60000 字符，并只展示文档路径。托管知识库在初始化、整理前和整理后都有提交；可恢复任意当前祖先快照，恢复始终创建新提交，不重写历史。恢复写入或提交失败时会回到操作前 HEAD 和文件内容，不留下半恢复状态。普通项目只有“完全由文档组成、不是 merge、工作区 clean”的祖先提交可一键恢复；混合代码提交明确拒绝。托管私有笔记默认没有 remote，也不自动 push。

## 4. 网页端共享逻辑

PC 网页端通过以下云端 API 应用虚拟分区：

```http
POST /api/projects/:project_id/docs/organization/apply
```

请求携带 `authorization_mode`、对应审核状态和三类 expected revision。云端通过项目绑定的 PC 节点读写两份 `.elon` JSON，并复用 MCP 相同的 Rust schema、清洗、真实路径校验、合并和幂等规则。网页端只负责展示与用户交互，不再自行实现建议合并算法。

本机路线还通过 loopback 管理端点创建并轮询整理运行。页面展示从建议生成、虚拟分区应用到实体文件应用的每个 MCP 阶段、token、revision 和失败恢复建议；发起整理后停留在文档工作台。页面按项目保存三档权限，默认选中“AI 自动整理（可信且可恢复）”。实体操作通过 `/api/project-docs/organization/apply-files` 交给本机节点在 canonical Git 工作区执行相同 Rust 安全门禁。

知识首页允许用户固定项目模板。治理模式先进入“治理总览”，可按四个维度交叉筛选并编辑副主题；旧的必须/按需/当前等分区保留为快捷视图。“文档健康”分区展示服务端统一的结构分、质量问题、维护事件和可分页惰性展开的联邦节点，并允许筛选、分派、设期限、填写忽略/延期原因、选择问题让 AI 定向建议、查看趋势、评分解释、版本差异和安全恢复；“AI 整理建议”分区展示分区新增/合并/移动/重命名建议、理由、影响和应用状态。右键、每项 `⋯`、键盘 `Shift+F10` 和触摸长按调用同一套命令定义，不建立桌面端专属语义。正文面板同时提供 Markdown 编辑器和安全渲染预览。

项目图谱使用确定性树/关系布局，顶部切换产品功能、技术架构和文档主题。默认只展开一级以保持节点可读，支持展开/折叠、缩放/平移、缩略图、节点/文档/实现证据搜索和文档覆盖筛选。详情区把“Markdown 覆盖”和“实现证据”分开显示；`topics` 节点可回到对应 OneNote 分区。图级“与 AI 评审此图”和节点级“与 AI 讨论此节点”会在同一整理任务中要求代理直接调用图谱 MCP，并把确认有价值的变更写入 `proposed_knowledge_graph`，不靠页面点击，也不为凑指标生成重复文档。

显示排序和项目结构分开保存：按名称、数量、路径或权威性的个人查看偏好只留在浏览器；手工分区顺序、文档固定/顺序、入口和归类属于项目共同知识架构，写 `.elon/document-sections.json`。其中 `document_metadata.order` 与 `document_metadata.pinned` 保存共享文档顺序，`audit_log` 保存最近 100 条结构操作。前端对清单写入使用 revision 防并发覆盖，并提供最多 20 步会话内撤销；Git 仍是跨会话恢复和 AI 实体整理的最终历史。

手工新建分区、删除主题子树和主题/治理归类只修改虚拟知识架构，不会自动创建、移动或删除实际目录与 Markdown。治理覆盖只是检索与工作台标记，不能突破真实路径权威上限。AI 可以提出新的项目模板、首页、分区、缺失文档和关系建议；是否自动应用由统一权限模式决定。

## 5. 安全和失败原则

- MCP 会话绑定 canonical Git 工作区或平台创建的托管 Git 知识库，URL 使用高熵短期令牌，默认两小时过期。
- 配置和会话文件位于系统临时目录；启动器只接受 loopback URL。
- 会话先在 staging 目录完整写入，再原子发布；并发清理跳过创建中目录，损坏会话也有宽限期，不会删除另一代理刚创建的会话。
- Markdown 读取继续经过工作区边界、符号链接、UTF-8 和 2 MiB 上限检查。
- 建议与分区写入采用原子替换和 optimistic revision；实体操作还校验 catalog、建议、分区和源文件四类 revision。
- 无效 JSON、未知路径、未知分区、过期 revision、权限模式不允许或缺少必要审核都必须显式失败。
- MCP 不可用时，AI 可以使用相同两份 JSON 契约完成建议，但仍要遵守先目录、再按需正文、最后按当前权限模式应用的顺序。

## 6. 验证入口

Rust 单元测试覆盖：summary 响应字节预算与集合不泄露、多维治理与副主题、功能/架构/主题分离、父子图谱聚合、实现证据评分、局部图与分离 token 阅读预算、增量索引和事件去重、链接/孤立/重复标题/owner/复查/实现漂移、问题处理状态与趋势、联邦 glob 与惰性分页、普通项目和托管库的版本差异/失败恢复、分页与字符预算、路径越界、授权模式、revision 冲突、安全重命名/移动、短期会话鉴权、`tools/list` 和直接 `tools/call`。PC 前端合约测试覆盖自定义分区 CRUD、AI 分区建议、Markdown 编辑/渲染和联邦展开。

发布前至少运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 test --manifest-path server\Cargo.toml --bin elon-pc-node project_document
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_project_docs_mcp
cd pc-frontend
npm run build
```
