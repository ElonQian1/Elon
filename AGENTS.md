# 一龙项目 AI 工作入口

最后更新：2026-06-24

本文件是所有 AI 工具（Codex / Codex CLI / Copilot / Copilot CLI）的共享路由层。

**规则权威来源：`.github/copilot-instructions.md`**
所有 AI 代理开始任务前必须先读取该文件，其中包含完整的硬规则、任务开始命令、部署速查表和 Android 完成定义。本文件只保留路由索引，不重复规则内容。

## 必读顺序

1. 读 `.github/copilot-instructions.md`（规则权威来源，含部署速查、任务开始命令）
2. 读本文件（按任务类型路由到专项文档）
3. 只读和当前任务相关的专项文档，不全量读取

## 按任务读取文档

| 任务类型 | 继续读取 |
|---|---|
| 项目定位、架构、模块入口、AI 任务模板 | `AI_PROJECT.md`、`AI_ARCHITECTURE.md`、`AI_INDEX.md`、`AI_TASK_TEMPLATE.md` |
| Git、worktree、提交、push、部署、发布 | `.github/instructions/git-deploy-workflow.instructions.md` |
| PowerShell 版本、`powershell`/`pwsh` 选择、PS5 设备兼容 | `docs/powershell-version-policy.md` |
| 模块化、拆文件、治理巨型文件 | `.github/instructions/modular-architecture.instructions.md` |
| 后端架构、API、数据流 | `docs/system-architecture.md` 和任务相关源码 |
| PC 工作台、`/pc` 页面、前端框架迁移、React/Vite/TypeScript | `.github/instructions/pc-frontend-migration.instructions.md`、`docs/pc-frontend-migration.md`、`AI_INDEX.md` |
| 完整开发流程或任务卡住 | `docs/ai-agent-workflow.md` |
| Android APK 发布 | `.github/instructions/git-deploy-workflow.instructions.md` 的 APK 部署章节 |
| Gradle 下载或 Android 首次编译环境异常 | `docs/android-setup.md` |
| APP UI、主题、颜色、按钮、卡片、底部导航、状态胶囊 | `docs/APP 颜色规范.md`；涉及 APK 与网页同步时再读 `.github/instructions/apk-web-ui-sync.instructions.md` |
| APP 当前版本代号、产品版本记忆 | `docs/app-version-memory.md` |
| Copilot 配置或 VS Code Customizations | `.github/prompts/`、`.github/agents/`、`.github/skills/` |
| 查询聊天记录、会话时间线、APK下载地址溯源、诊断"为什么这么慢" | `docs/query-chat-records.md` |

## 脚本优先

- 任务开始：先运行 `scripts\ai-task-preflight.ps1 -CreateWorktree`（Linux/macOS：`bash scripts/ai-task-preflight.sh --create-worktree`）。脚本会同步本地 `main` 基线并从最新 `origin/main` 派生独立任务 worktree；业务修改必须切到脚本输出的 `WORKTREE_PATH` / `EDIT_ROOT` 后进行，`main` 只做共享最新基线。
- 预检输出为准：如果看到 `EDIT_ROOT=BLOCKED_CREATE_WORKTREE_FIRST`，说明当前目录不能编辑，必须重新按脚本提示创建 worktree；如果看到具体路径，所有读写、格式化、测试、提交都先切到该路径。
- PowerShell 版本：Windows bootstrap 脚本可继续用系统自带 `powershell.exe`；凡脚本头部有 `#requires -Version 7.0`，必须用 `pwsh` 运行。PowerShell 5 设备先运行 `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-pwsh7.ps1` 检查；没有 PowerShell 7 时不要修改或降级 PS7 脚本，按 `docs/powershell-version-policy.md` 安装或转交给有 `pwsh` 的环境。
- 任务流程门禁：修改 `ai-task-preflight`、worktree 清理、并行 AI 说明或 Git 工作流文档后，必须运行 `powershell -ExecutionPolicy Bypass -File scripts\test-ai-task-preflight-workflow.ps1`，防止 `-CreateWorktree`、`WORKTREE_PATH`、`main` 基线规则漂移。
- 后端发布：业务代码 commit + push 后运行 `scripts\publish-server.ps1` 或 `scripts/publish-server.sh`，再验证 `/health` 和 `/api/server/version`；若发布期间被更新的 `origin/main` 或服务器版本超越，按脚本提示汇报“代码已推送，发布交由后续最新 main”，不要反复 rebase 重跑。
- Android 可安装端发布：业务代码 commit + push 后，Windows 运行 `scripts\publish-apk.ps1 -Changelog "<用户可见改动>"`，Linux 运行 `bash scripts/publish-apk.sh --changelog="<用户可见改动>"`；并行任务若只要求代码先合并，运行 `scripts\check-task-complete.ps1 -Kind CodePushed` 即可收尾；明确负责 APK 发布的任务再运行 `-Kind AndroidFeature`。
- 任务收尾清理 worktree：`scripts\cleanup-task-worktrees.ps1 -Apply`（Windows）或 `bash scripts/cleanup-task-worktrees.sh --apply`（Linux）。预览模式（不带 `-Apply`/`--apply`）只列不删；脏 worktree 和未合并分支会自动保留。
- 脚本已经负责版本 claim/finish、构建、上传、并发保护和清理。AI 不要手搓这些步骤。
