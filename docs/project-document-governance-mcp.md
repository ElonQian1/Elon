# 项目文档治理 MCP

最后更新：2026-07-17

本文是接入或诊断项目文档治理 MCP 时才读取的按需手册。日常文档任务先遵循 `.github/instructions/document-authority.instructions.md`，不要把本文复制到 Codex、Claude、Gemini 或 Copilot 的私有桥接文件。

## 1. 知识架构、治理属性和实际目录

- Git 工作区中的 Markdown 和路径是内容真源。
- 路径决定文档权威性上限；正文只能降低自身生命周期，不能越过路径上限。
- PC 网页端“知识架构”按项目主题浏览，“治理视图”根据 `role`、`lifecycle`、`authority` 和 `ambiguous` 判断权威性；两者是正交维度。
- `.elon/document-sections.json` 保存项目类型、知识首页、层级主题、主题固定项、治理覆盖和文档关系，不移动实际文件。`assignments` 与 `governance_overrides` 分开，避免设置主题时覆盖权威性状态。
- 主题树只改变 OneNote 式浏览位置，不改变 `role`、`lifecycle`、`authority` 或 `default_retrieval`；AI 检索仍以真实路径元数据为准。
- `.elon/document-organization-suggestions.json` 保存待审核或已应用的 AI 建议，不是当前规则真源。
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

bootstrap 只接受现存 Git 工作区，返回：

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

第一步调用。它扫描最多 500 份候选 Markdown，但不返回正文；默认每页 80 份，最大 200 份。输出包含：

- `catalog_revision`；
- 路径、标题、大小、哈希和标题层级；
- `role`、`lifecycle`、`authority`、`default_retrieval`、`ambiguous`；
- 自动治理分区、知识架构健康度、用户覆盖和现有建议；
- 全量读取估算、默认读取估算和预计避免 token。

`classification_model_tokens` 固定为 `0`，表示预分类没有调用模型，不表示后续 AI 判断不消耗 token。`knowledge_architecture` 还返回项目类型推断、完整度分数、基础文档覆盖、过期/歧义/重复标题、缺失文档类型和推荐主题。

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
- 新分区不超过 16 个、层级不超过四层，建议归类不超过 500 条；
- 当前建议 revision 未被其他会话修改。

该工具只能写建议 JSON，不能写分区配置或 Markdown。

### `project_docs_get_suggestions`

读取当前结构化建议和 revision，不读取正文。

### `project_docs_apply_suggestions`

工具把建议的项目类型、知识首页、层级主题、文档关系和归类合并到知识架构清单，再将建议状态改为 `applied`。默认 `git_backed_full` 会先创建整理前提交；没有实体操作时同时创建整理后提交，有实体操作时返回 `git_baseline_commit` 交给下一工具；`review_all` 必须显式传 `reviewed=true`；`suggestions_only` 禁止调用：

- catalog、manifest 和 suggestions revision 必须一致；
- 重复调用是幂等的；
- 第二步状态写入失败时可安全重试；
- 永远不移动、删除或改写 Markdown。

### `project_docs_apply_file_operations`

它对请求中选定的结构化 `file_operations` 执行 Markdown 重命名/移动：

- 请求始终必须带选中的 operation id；默认 `git_backed_full` 自动授予 rename/move，并优先接收上一工具返回的 `git_baseline_commit`；`review_all` 才要求 `reviewed=true` 和对应 `allow_*`；
- 每项使用 analyze 目录里的 `content_hash` 作为 `source_revision`，防止文档变化后仍按旧建议操作；
- 只允许 Git 工作区内的 Markdown，禁止覆盖现有目标、删除文件、越过工作区或改写正文；
- 执行后同步 `.elon/document-sections.json` 中受影响的路径，并把操作标记为 `applied`；
- 不修改正文引用、不自动 push；`git_backed_full` 成功响应必须同时提供 `git_baseline_commit`、`git_result_commit` 和 `git_document_transaction_complete=true`。

AI 只能执行建议文件中列出的 operation id，不能借文档整理修改代码或 push。修改正文、批量修复引用、归档和删除尚未进入低 token 分类操作 schema；未来即使开放，也必须纳入同一整理前/后 Git 事务。

## 4. 网页端共享逻辑

PC 网页端通过以下云端 API 应用虚拟分区：

```http
POST /api/projects/:project_id/docs/organization/apply
```

请求携带 `authorization_mode`、对应审核状态和三类 expected revision。云端通过项目绑定的 PC 节点读写两份 `.elon` JSON，并复用 MCP 相同的 Rust schema、清洗、真实路径校验、合并和幂等规则。网页端只负责展示与用户交互，不再自行实现建议合并算法。

本机路线还通过 loopback 管理端点创建并轮询整理运行。页面展示从建议生成、虚拟分区应用到实体文件应用的每个 MCP 阶段、token、revision 和失败恢复建议；发起整理后停留在文档工作台。页面按项目保存三档权限，默认选中“AI 自动整理（可信且可恢复）”。实体操作通过 `/api/project-docs/organization/apply-files` 交给本机节点在 canonical Git 工作区执行相同 Rust 安全门禁。

知识首页允许用户固定项目模板；手工新建一级/子分区、删除主题子树和单篇主题/治理归类仍写 `.elon/document-sections.json`，不会自动创建或删除实际目录。AI 可以提出新的项目模板、首页、分区、缺失文档和关系建议；是否自动应用由统一权限模式决定。

## 5. 安全和失败原则

- MCP 会话绑定 canonical Git 工作区，URL 使用高熵短期令牌，默认两小时过期。
- 配置和会话文件位于系统临时目录；启动器只接受 loopback URL。
- 会话先在 staging 目录完整写入，再原子发布；并发清理跳过创建中目录，损坏会话也有宽限期，不会删除另一代理刚创建的会话。
- Markdown 读取继续经过工作区边界、符号链接、UTF-8 和 2 MiB 上限检查。
- 建议与分区写入采用原子替换和 optimistic revision；实体操作还校验 catalog、建议、分区和源文件四类 revision。
- 无效 JSON、未知路径、未知分区、过期 revision、权限模式不允许或缺少必要审核都必须显式失败。
- MCP 不可用时，AI 可以使用相同两份 JSON 契约完成建议，但仍要遵守先目录、再按需正文、最后按当前权限模式应用的顺序。

## 6. 验证入口

Rust 单元测试覆盖：元数据目录不泄露正文、分页与字符预算、路径越界、虚构建议路径、三档授权、revision 冲突、幂等应用、安全重命名/移动、禁止覆盖、自动授权阶段观测、失败恢复、终态不可回退、短期会话鉴权、并发会话创建、`tools/list` 和直接 `tools/call`。

发布前至少运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 test --manifest-path server\Cargo.toml --bin elon-pc-node project_document
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_project_docs_mcp
cd pc-frontend
npm run build
```
