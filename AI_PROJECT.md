# Elon AI Project Guide

本文件给 AI 代理快速理解 elon 自项目。规则权威仍是 `.github/copilot-instructions.md`，本文件只记录项目事实、架构入口和项目理解/RAG现状。

## 项目是什么

elon 是一个云端 APK 开发平台。用户在手机 APK 里用自然语言描述需求，后端把任务交给 AI CLI/Codex/Copilot 等代理，在真实 Git 工作区修改代码、验证、构建、发布，再把结果回传给用户。

本仓库同时包含：

- `server/`：Rust 后端、项目管理、AI CLI 调度、发布接口、context compiler、RAG/符号索引能力。
- `android/`：一龙 APK 客户端，负责对话、项目入口、任务进度、APK 更新。
- `scripts/`：后端发布、APK 发布、预检、worktree 清理等自动化脚本。
- `default-project-docs/`：给用户新项目种下的 AI 工作入口和默认规则模板。
- `.github/`：本仓库 AI 代理的规则权威、专项 instructions、skills。

## 当前项目理解能力

项目已经不只是基础 repo map。当前已有分层上下文系统：

- 项目规则层：`AGENTS.md`、`.github/copilot-instructions.md`、`.github/instructions/*.instructions.md`。
- Repo map / context compiler：`server/src/context_compiler/` 生成文件树、摘要、repo map、context pack 和任务上下文包。
- Rust 代码结构层：Rust 符号扫描、rust-analyzer/LSP 事实、`semantic_facts.jsonl`、`lsp_locations.jsonl`。
- 符号索引层：`symbol_index.sqlite`、符号/边/lookup、impact pack、task pack、graph query、retrieval eval。
- 混合检索层：关键词/符号/chunk/vector 多路召回，`repo_context_task_pack` 和 `repo_symbol_search` 面向 agent 使用。
- 向量层：已有本地 `local-hash-v1` embedding provider 和 `embeddings` 表；schema 支持同一 chunk 存多个模型向量。
- 验证闭环：patch plan、dry run、review、verification、repair context、目标测试建议。

## 还没完全做到的部分

这些是后续最有价值的完善方向：

1. 真正的远程 embedding provider：接入用户自带 API key 和模型配置，把云端 embedding 写入现有 `embeddings(chunk_id, model)`；设计边界见 `AI_ARCHITECTURE.md`。
2. 多模型检索策略：按项目/用户选择 embedding 模型，并在 task pack 中记录模型来源、维度、召回质量。
3. 索引增量更新：文件变更后只重算受影响 chunks、symbols、embeddings，而不是全量重建。
4. 用户项目默认 AI 文档：新项目应默认具备 `AI_PROJECT.md`、`AI_INDEX.md`、`AI_RULES.md`、`AI_TASK_TEMPLATE.md` 和 `.aiignore`。
5. 检索质量回归集：把真实任务沉淀为 retrieval eval cases，防止后续改动降低召回质量。

## 常用完成标准

后端运行代码改动必须：

1. 在隔离 worktree 中修改。
2. `git add` 仅添加本任务文件。
3. commit 后立即 push 到 `origin/main`。
4. 运行 `scripts\check-task-complete.ps1 -Kind CodePushed`。
5. 需要上线后端时运行 `scripts\publish-server.ps1`。
6. 验证 `/health` 和 `/api/server/version`。

Android/APK 改动只有用户明确要求安装包或下载链接时才发布 APK；普通代码任务到 CodePushed 即可收尾。
