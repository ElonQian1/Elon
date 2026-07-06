# 一龙项目 AI 工作入口

最后更新：2026-07-05

本文件是所有 AI 工具（Codex / Codex CLI / Copilot / Copilot CLI）的共享路由层。

**规则权威来源：`.github/copilot-instructions.md`**
所有 AI 代理开始任务前必须先读取该文件，其中包含完整的硬规则、任务开始命令、部署速查表和 Android 完成定义。本文件只保留路由索引，不重复规则内容。

## 必读顺序

1. 读 `.github/copilot-instructions.md`（规则权威来源，含部署速查、任务开始命令）
2. 读本文件（按任务类型路由到专项文档）
3. **如使用 Codex CLI**，读 `CODEX.md`（含 Codex 专用脚本信号优先级、Prewarm 约束、Stale session 重试规则）
4. 只读和当前任务相关的专项文档，不全量读取

## 按任务读取文档

| 任务类型 | 继续读取 |
|---|---|
| 项目定位、架构、模块入口、AI 任务模板 | `AI_PROJECT.md`、`AI_ARCHITECTURE.md`、`AI_INDEX.md`、`AI_TASK_TEMPLATE.md` |
| Git、worktree、提交、push、部署、发布 | `.github/instructions/git-deploy-workflow.instructions.md` |
| Rust 验证、`cargo check`、`cargo test`、`cargo build`、`cargo clippy` | `.github/instructions/git-deploy-workflow.instructions.md` 的 Cargo 验证共享缓存与锁章节 |
| Rust 格式化、`cargo fmt`、纯格式化拆提交 | `.github/instructions/git-deploy-workflow.instructions.md` 的 Rust 格式化章节 |
| PowerShell 版本、`powershell`/`pwsh` 选择、PS5 设备兼容 | `docs/powershell-version-policy.md` |
| 模块化、拆文件、治理巨型文件 | `.github/instructions/modular-architecture.instructions.md` |
| 后端架构、API、数据流 | `docs/system-architecture.md` 和任务相关源码 |
| PC 工作台、`/pc` 页面、前端框架迁移、React/Vite/TypeScript | `.github/instructions/pc-frontend-migration.instructions.md`、`docs/pc-frontend-migration.md`、`AI_INDEX.md` |
| Windows PC 节点客户端、启动器、安装/自更新、节点推送更新 | `.github/instructions/git-deploy-workflow.instructions.md` 的 Windows PC 节点客户端部署章节 |
| 完整开发流程或任务卡住 | `docs/ai-agent-workflow.md` |
| Android APK 发布 | `.github/instructions/git-deploy-workflow.instructions.md` 的 APK 部署章节 |
| Gradle 下载或 Android 首次编译环境异常 | `docs/android-setup.md` |
| APP UI、主题、颜色、按钮、卡片、底部导航、状态胶囊 | `docs/Design.md`、`docs/APP 颜色规范.md`；涉及 APK 与网页同步时再读 `.github/instructions/apk-web-ui-sync.instructions.md` |
| APP 当前版本代号、产品版本记忆 | `docs/app-version-memory.md` |
| Copilot 配置或 VS Code Customizations | `.github/prompts/`、`.github/agents/`、`.github/skills/` |
| Prompt / Agent / Skill token 体检、去重复、路由化 | `scripts/audit-ai-prompt-assets.ps1`，再按需读取 `.github/prompts/`、`.github/agents/`、`.github/skills/` |
| 查询聊天记录、会话时间线、APK下载地址溯源、诊断"为什么这么慢" | `docs/query-chat-records.md` |

## 脚本优先

- 任务开始：先运行 `scripts\ai-task-preflight.ps1 -CreateWorktree`（Linux/macOS：`bash scripts/ai-task-preflight.sh --create-worktree`）。脚本会同步本地 `main` 基线并从最新 `origin/main` 派生独立任务 worktree；业务修改必须切到脚本输出的 `WORKTREE_PATH` / `EDIT_ROOT` 后进行，`main` 只做共享最新基线。
- 预检输出为准：如果看到 `EDIT_ROOT=BLOCKED_CREATE_WORKTREE_FIRST`，说明当前目录不能编辑，必须重新按脚本提示创建 worktree；如果看到具体路径，所有读写、格式化、测试、提交都先切到该路径。
- PC 节点 MCP 会话例外：如果当前目录已经是一龙平台创建的 `conversation-worktrees/<project>/<conversation>`，或当前分支形如 `ai/session/<project>/<conversation>`，说明平台已经完成隔离 worktree 准备；不要再运行 `ai-task-preflight.ps1 -CreateWorktree` 创建嵌套 worktree。直接在当前 worktree 运行 `git status --short --branch`，按任务继续执行，不能只停在“已读取规则、后续会执行”的说明。
- 并行 rebase 边界：`origin/main` 前进是并行常态，不能把“远端前进”当作自动 rebase / 重跑 / 重新发布的条件。只在 `git push origin HEAD:main` 被 non-fast-forward 拒绝时，才 `git fetch origin` + `git rebase origin/main` 一次；本任务提交已经包含在 `origin/main` 后，代码层面即完成，后续更新 main 由后续任务或发布协调者接管。
- AI 任务完成状态与收尾状态分离：业务提交已包含在 `origin/main`，或发布脚本已验证服务器 / APK / PC 节点指向本次或更新主线后，业务完成状态不得被本地主工作区 `main` 同步失败、worktree 清理失败覆盖。此类问题只能记录为收尾状态，例如 `cleanup_failed` 或 `local_main_diverged`，最终汇报必须同时说明“业务已完成/已发布”和“本机收尾待处理”。
- PowerShell 版本：Windows bootstrap 脚本可继续用系统自带 `powershell.exe`；凡脚本头部有 `#requires -Version 7.0`，必须用 `pwsh` 运行。PowerShell 5 设备先运行 `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-pwsh7.ps1` 检查；没有 PowerShell 7 时不要修改或降级 PS7 脚本，按 `docs/powershell-version-policy.md` 安装或转交给有 `pwsh` 的环境。
- 任务流程门禁：修改 `ai-task-preflight`、worktree 清理、并行 AI 说明或 Git 工作流文档后，必须运行 `powershell -ExecutionPolicy Bypass -File scripts\test-ai-task-preflight-workflow.ps1`，防止 `-CreateWorktree`、`WORKTREE_PATH`、`main` 基线规则漂移。
- Rust 日常验证：不要并行裸跑 `cargo check` / `cargo test` / `cargo build` / `cargo clippy` 到同一个 target。Windows 用 `powershell -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 check --manifest-path server\Cargo.toml`，Linux/macOS 用 `bash scripts/cargo-dev.sh check --manifest-path server/Cargo.toml`；脚本读取 `.env.local` / `ELON_DEV_CARGO_TARGET_DIR`，复用开发 target 并加锁。发布构建仍由 `RUST_SERVER_MUSL_TARGET_DIR` + `publish-server.*` 管理。
- 后端发布：业务代码 commit + push 后运行 `scripts\publish-server.ps1` 或 `scripts/publish-server.sh`，再验证 `/health` 和 `/api/server/version`；若发布期间被更新的 `origin/main` 或服务器版本超越，按脚本提示汇报“代码已合并，发布交给最新主线”，不要反复 rebase 重跑。
- Windows PC 节点发布：影响 Win 节点客户端、启动器、安装/自更新、节点托盘或 `elon-pc-node` 的用户可见改动，业务代码 commit + push 后运行 `scripts\publish-node-agent.ps1`，再运行 `scripts\check-task-complete.ps1 -Kind NodeAgent`；脚本默认调用 `/api/admin/nodes/push-update` 通知在线节点自动更新并重连，优先用本机 `ADMIN_TOKEN` / `ELON_ADMIN_TOKEN`，没有时通过 SSH 在服务器本机读取 token。
- Android 可安装端发布：业务代码 commit + push 后，Windows 运行 `scripts\publish-apk.ps1 -Changelog "<用户可见改动>"`，Linux 运行 `bash scripts/publish-apk.sh --changelog="<用户可见改动>"`；并行任务若只要求代码先合并，运行 `scripts\check-task-complete.ps1 -Kind CodePushed` 即可收尾；明确负责 APK 发布的任务再运行 `-Kind AndroidFeature`。
- 任务收尾清理 worktree：`scripts\cleanup-task-worktrees.ps1 -Apply`（Windows）或 `bash scripts/cleanup-task-worktrees.sh --apply`（Linux）。预览模式（不带 `-Apply`/`--apply`）只列不删；脏 worktree 和未合并分支会自动保留。
- 脚本已经负责版本 claim/finish、构建、上传、并发保护和清理。AI 不要手搓这些步骤。
- Prompt / Agent / Skill 文档体检：运行 `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\audit-ai-prompt-assets.ps1`；发现固定全量读取多份 instructions/docs 时，优先改成先读 `AGENTS.md` 再按任务路由。
