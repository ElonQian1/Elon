# 一龙项目 AI 工作入口

本文件是 VS Code Copilot、Codex、Claude Code 等多 AI 工具共享的工作入口。仓库级规则以 `.github` 下的 VS Code Copilot customization 文件为准，本文件只做索引，避免多处复制长规则。

## 必读顺序

1. `.github/copilot-instructions.md`：项目定位、全局 AI 原则、VS Code Copilot 工作方式记忆。
2. `.github/instructions/git-deploy-workflow.instructions.md`：Git、push、worktree、部署、版本号、交付汇报强制流程。
3. `docs/vscode-copilot-working-model.md`：本仓库如何采用 VS Code instructions / prompts / agents。
4. `docs/ai-agent-workflow.md`：需求分析、代码定位、修改、验证、部署的完整业务流程。
5. `docs/system-architecture.md`：架构、模块边界、数据流和安全约束。

## VS Code 快捷入口

- 常规代码任务：运行 `/elon-dev-task`。
- APK 发布任务：运行 `/elon-apk-release`。
- 只做规划：选择 `elon-planner` agent，或运行 VS Code 内置 `/plan`。
- 执行实现：选择 `elon-implementer` agent。
- 提交前审查：选择 `elon-reviewer` agent。

## 工作原则

- 先读上下文，再改文件。
- 有未提交并发改动时，用临时 worktree 隔离。
- 只 stage 当前任务文件。
- 每次任务必须 commit，并按要求 push 到 `origin/main`。
- 部署必须基于干净、已提交、已推送的 SHA。
- 不提交密钥、`.env`、APK 签名材料或任何敏感信息。
