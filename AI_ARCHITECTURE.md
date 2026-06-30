# Elon AI Architecture Guide

本文件给 AI 代理快速理解 elon 的架构分层。完整系统说明见 `docs/system-architecture.md`；本文件保持高信噪比，帮助 AI 在修改前判断应该查哪一层。

## 总体分层

```text
Android APK / Web UI
  -> Rust API server
  -> Intent Router / Conversation Owner
  -> Demo Oracle（低成本产品预演，按需触发）
  -> Group AI Coordinator / Matter Planner
  -> AI-to-AI Skill Router / Skill Registry
  -> AI CLI / API agent routing
  -> Project workspace / Git worktree
  -> Context compiler / repo map / symbol index
  -> Build, publish, verification scripts
```

## 运行时主链路

1. 用户在 APK 或 Web 里通过多轮讨论表达自然语言需求。
2. 后端识别项目、用户、会话、模型配置以及需求成熟度。
3. 对目标仍不清晰、改动成本高或存在多个产品方向的需求，调用预言家 AI 生成低成本 demo、页面草图、用户流程和待确认问题；明确的小改动跳过该阶段。
4. 用户确认方向后，Group AI Coordinator 生成 Matter，Skill Router 从官方或已审核 Skill 中选择能力组合，并说明选择理由、成本和风险。
5. AI CLI、API agent 或 Worker Bot 在真实 Git 隔离工作区执行任务。
6. context compiler 为不同角色生成裁剪后的 repo map、symbol index、task pack 和验证线索。
7. AI 修改代码后由 Reviewer / Verifier 执行最小有效验证和独立审查。
8. 业务提交进入目标项目 Git 历史，再由后端或 APK 发布脚本按需构建、上传、部署和验证。
9. 运行结果、用户验收和失败原因沉淀为 Context、Taste、Skill 质量数据。

预言家 AI 不是正式开发者，也不拥有发布权限。它默认使用低成本模型和受限工具，只允许生成临时 demo 产物；不得直接修改正式项目主线、接入真实支付、执行生产部署或把假数据包装成已完成能力。

## 项目理解 / RAG 架构

当前项目理解系统不是纯向量库，而是混合检索：

```text
AI_PROJECT / AI_INDEX / AGENTS
  -> context compiler
  -> repo map + chunks
  -> rust-analyzer / semantic facts
  -> symbol_index.sqlite
  -> symbol search + graph + impact pack
  -> local-hash-v1 vector retrieval
  -> task pack / patch plan / verification hints
```

各层职责：

- 文档层：说明项目是什么、规则是什么、入口在哪里。
- repo map 层：压缩仓库结构、重要文件、目录摘要。
- 符号层：记录函数、类型、trait、模块、调用/引用/包含关系。
- chunk 层：为全文搜索和向量召回提供代码片段。
- 向量层：补充自然语言语义召回；当前默认 `local-hash-v1`。
- 验证层：生成 patch plan、dry run、review、verification、repair context。

## 关键模块边界

| 模块 | 职责 |
|---|---|
| `server/src/context_compiler/` | repo map、上下文包、符号索引、RAG/task pack |
| `server/src/context_compiler/agent_rag_context.rs` | 暴露给 API agent 的 RAG 工具定义和调用 |
| `server/src/context_compiler/symbol_index_task_pack.rs` | 自然语言任务到符号/影响/context pack 的主流程 |
| `server/src/context_compiler/symbol_index_embedding_provider.rs` | embedding provider 抽象与本地 hash provider |
| `server/src/agent_config.rs` | 用户模型/API key 配置和加密持久化 |
| `server/src/agent_llm_call.rs` | OpenAI-compatible chat 调用和用量记录 |
| `scripts/publish-server.ps1` | 后端构建、版本 claim、上传、部署、验证 |
| `scripts/publish-apk.ps1` | APK 构建、签名、上传和版本发布 |

## 当前缺口

架构上已经完成了“文档 + repo map + 符号索引 + 混合检索 + 验证闭环”的主干。剩余增强应按这个顺序做：

1. 远程 embedding provider：把用户 API key/模型配置接入 `SymbolEmbeddingProvider`，支持 OpenAI-compatible `/embeddings`。
2. 成本和权限边界：远程 embedding 必须记录模型、维度、调用来源和失败原因，避免默认高成本全量回填。
3. 增量索引：文件 hash 未变化时跳过 chunk、symbol、embedding 重算。
4. 检索回归集：把真实任务保存为 eval case，比较 symbol/chunk/vector 召回质量。
5. 面向 UI 的状态解释：让用户能看到“项目已索引/缺少 embedding/需要重新索引”的原因。

## 修改建议

- 需要改项目理解/RAG：优先进入 `server/src/context_compiler/`，不要另建平行管线。
- 需要接用户 API key：先查 `server/src/agent_config.rs` 和 `server/src/user_agent_secrets.rs`。
- 需要新增检索能力：先让 `repo_context_task_pack` 能解释和暴露，不要只做后台数据。
- 需要上线后端：按 `AGENTS.md` 和 `.github/copilot-instructions.md` 的 commit/push/deploy 流程执行。
