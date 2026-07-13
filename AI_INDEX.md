# Elon AI Index

本文件是给 AI 的高信噪比入口索引。需要规则时先读 `AGENTS.md`；需要项目事实时读 `AI_PROJECT.md`；需要架构分层时读 `AI_ARCHITECTURE.md`；需要具体实现时再按本文件定位源码。

## 后端核心入口

| 领域 | 入口 |
|---|---|
| Rust 服务启动和路由 | `server/src/main.rs` |
| 项目 API / 项目空间 | `server/src/project_api.rs` 及同领域拆分模块 |
| AI CLI 调度 | `server/src/ai_cli/`、`server/src/agent.rs` |
| PC 节点 AI 三层架构 / Codex JSON、pipe sidecar 与 PTY 分工 | `AI_ARCHITECTURE.md` 的“PC 节点 AI 运行路线”、`docs/符号索引讨论/我们项目的cli能力.md` |
| Win 节点轻量工具箱 / Codex CLI 临时 PATH / 工具收录策略 | `docs/win-node-toolbox.md`、`server/src/node_agent_cli_env.rs`、`server/src/node_agent_cli_tool_catalog.rs` |
| PC 节点数据根 / 构建缓存 / C 盘治理 / 旧目录迁移 | `docs/pc-node-data-root.md`、`server/pc-dev-runtime/src/node_data_paths.rs`、`server/src/node_agent_data_root/` |
| 模型供应商和自定义模型 | `server/src/model_*`、`server/src/agent_model_*` |
| 用户等级、经验条、token 消耗/分享算力经验 | `server/src/user_progression.rs`、`server/src/store/user_progression.rs`、`server/src/token_usage_api.rs`、`server/src/store/node_ledger.rs` |
| context compiler / repo map | `server/src/context_compiler/` |
| 项目 RAG 工具上下文 | `server/src/context_compiler/agent_rag_context.rs` |
| 符号索引 API | `server/src/context_compiler/symbol_index_api.rs` |
| task pack / impact pack | `server/src/context_compiler/symbol_index_task_pack.rs`、`symbol_index_impact_pack.rs` |
| 向量检索 | `server/src/context_compiler/symbol_index_vector.rs` |
| embedding provider | `server/src/context_compiler/symbol_index_embedding_provider.rs` |
| SQLite 符号库 schema | `server/src/context_compiler/symbol_index_store.rs`、`symbol_index_embeddings.rs` |
| fb2 AI Center / 子项目聊天语音和业务上下文 | `docs/fb2-ai-center/`、`server/src/external_app_*`、`android/chat-voice-kit/` |
| AI-to-AI Skill、预言家 AI、demo 预演路线 | `docs/ai-to-ai-skill-oracle-roadmap.md`、`docs/群体ai开发/群体AI开发功能需求与架构设计.md` |

## Android 核心入口

| 领域 | 入口 |
|---|---|
| APK 主界面和导航 | `android/app/src/main/kotlin/com/elon/app/MainActivity.kt` |
| 应用更新 | `android/app/src/main/kotlin/com/elon/app/update/` |
| 网络/API/WebSocket | `android/app/src/main/kotlin/com/elon/app/net/` |
| 项目相关 UI | `android/app/src/main/kotlin/com/elon/app/project/` |
| `elon-self` 共享真机身份、无线 ADB 最近端点和连接约定 | `AI_PROJECT.md` 的“当前共享 Android 真机（项目记忆）”、`server/src/node_agent_android_inspector/` |

## Web/静态资源入口

| 领域 | 入口 |
|---|---|
| Web 项目页 | `server/src/assets/web_page.html` |
| PC 工作台旧静态入口 | `server/src/assets/pc_app.html`、`server/src/assets/pc_app.js`、`server/src/assets/pc_app.css`、`server/src/assets/pc_*.js`、`server/src/assets/pc_*.css` |
| PC 工作台新前端迁移目标 | `pc-frontend/`（建立后承接 `/pc-next` 和后续 `/pc`） |
| PC 静态资源服务端托管 | `server/src/web.rs`、`server/src/router.rs` |
| PC 前端迁移规则 | `.github/instructions/pc-frontend-migration.instructions.md`、`docs/pc-frontend-migration.md` |
| 项目广场/项目主页脚本 | `server/src/assets/project_*.js` |
| 节点管理本地页 | `server/src/node_agent_admin.html` |

## 脚本入口

| 任务 | 命令 |
|---|---|
| 任务预检并创建隔离 worktree | `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree` |
| 代码已推送检查 | `powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed` |
| 后端发布 | `powershell -ExecutionPolicy Bypass -File scripts\publish-server.ps1` |
| PC 前端本地预览 | `powershell -ExecutionPolicy Bypass -File scripts\start-pc-frontend-dev.ps1` |
| APK 发布 | `powershell -ExecutionPolicy Bypass -File scripts\publish-apk.ps1 -Changelog "<用户可见改动>"` |
| worktree 清理 | `powershell -ExecutionPolicy Bypass -File scripts\cleanup-task-worktrees.ps1 -Apply` |

## context compiler 产物

常见产物包括：

- `repo_map.md`
- `summaries.md`
- `symbols.jsonl`
- `symbol_index.jsonl`
- `symbol_edges.jsonl`
- `symbol_lookup.json`
- `symbol_index.sqlite`
- `chunks.jsonl`
- `tests.jsonl`
- `lsp_locations.jsonl`
- `semantic_facts.jsonl`
- `context_budget.json` / `context_budget.md`

面向 agent 的推荐入口优先级：

1. `repo_context_status`
2. `repo_context_task_pack`
3. `repo_symbol_search`
4. `list_dir` / `read_file` 作为兜底

## 修改前搜索建议

- 精确文案、函数名、错误信息：用 `rg`。
- Rust 类型、trait、调用关系：先查符号索引或 rust-analyzer 事实。
- 自然语言业务描述：优先 `repo_context_task_pack`，必要时启用 vector。
- 修改后影响面：查 impact pack，再跑建议测试。
