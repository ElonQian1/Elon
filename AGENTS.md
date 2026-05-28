# 一龙项目 AI 工作入口

最后更新：2026-05-27

本文件是所有 AI 工具共享的轻量入口，只保留常驻硬规则和文档路由。脚本、hook、测试已经能强制执行的细节，不再复制进常驻提示词。

## 常驻硬规则

1. 先保护现场：开始任务先运行 `scripts\ai-task-preflight.ps1 -CreateWorktree`；如果输出 `WORKTREE_CREATED=true`，必须切到 `WORKTREE_PATH` 继续。
2. 写代码前先给出 5-15 行文件计划，说明新建/修改哪些文件、预估行数、是否超过预算。
3. 不制造巨型文件：新源文件目标 <=500 行；501-800 行只允许单一职责；>800 行必须拆分。入口/路由文件优先控制在 500 行以内。
4. 不在已有巨型文件里继续堆功能：>1500 行文件除小修外，先把本次职责抽到独立模块。完整规则见 `.github/instructions/modular-architecture.instructions.md`。
5. 只 stage 本任务文件，提交前检查未跟踪文件，禁止混入其他 AI 或用户改动。
6. 任务起手和提交前都要 `git fetch origin main`；干净工作区可 rebase，脏工作区按归属 stash 或使用独立 worktree。
7. 后端和 APK 发布都必须基于已提交、已推送的 SHA；禁止未 commit 直接部署。
8. 发布版本号由服务器分配；禁止为了发布手动递增并提交 `server/Cargo.toml` 或 `build.gradle` 版本字段。
9. 不提交密钥、`.env`、APK 签名材料或任何敏感信息。
10. 新 clone 先装 hook：Windows 用 `pwsh scripts/install-hooks.ps1`，Linux/macOS 用 `bash scripts/install-hooks.sh`。

## 任务开始命令

Windows:

```powershell
$repo = git rev-parse --show-toplevel 2>$null
if (-not $repo) { $repo = $env:ELON_REPO_PATH }
if (-not $repo) { throw "未找到一龙仓库：请在仓库目录启动，或先设置 ELON_REPO_PATH" }
Set-Location -LiteralPath $repo
powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree
```

Linux / macOS / 服务器 Codex CLI:

```bash
repo="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [ -z "$repo" ]; then repo="${ELON_REPO_PATH:-}"; fi
if [ -z "$repo" ]; then echo "未找到一龙仓库：请在仓库目录启动，或先设置 ELON_REPO_PATH" >&2; exit 1; fi
cd "$repo"
bash scripts/ai-task-preflight.sh --create-worktree
```

## 按任务读取文档

不要默认全量读取所有说明文件。先读本入口和 `CODEX.md`，再按任务类型读取：

| 任务类型 | 继续读取 |
|---|---|
| Git、worktree、提交、push、部署、发布 | `.github/instructions/git-deploy-workflow.instructions.md` |
| 模块化、拆文件、治理巨型文件 | `.github/instructions/modular-architecture.instructions.md` |
| 后端架构、API、数据流 | `docs/system-architecture.md` 和任务相关源码 |
| 完整开发流程或任务卡住 | `docs/ai-agent-workflow.md` |
| Android APK 发布 | `.github/instructions/git-deploy-workflow.instructions.md` 的 APK 部署章节 |
| Gradle 下载或 Android 首次编译环境异常 | `docs/android-setup.md` |
| Copilot 配置或 VS Code Customizations | `.github/copilot-instructions.md`、`.github/prompts/`、`.github/agents/`、`.github/skills/` |
| 查询聊天记录、会话时间线、APK下载地址溯源、诊断"为什么这么慢" | `docs/query-chat-records.md` |

`copilot-instructions.md` 是 Copilot/VS Code 入口。Codex 只有在任务涉及 Copilot 配置、VS Code Customizations、或比较 Copilot/Codex 行为时才需要读取它。

## 脚本优先

- 后端发布：业务代码 commit + push 后运行 `scripts\publish-server.ps1` 或 `scripts/publish-server.sh`，再验证 `/health` 和 `/api/server/version`。
- Android 可安装端发布：业务代码 commit + push 后运行 `scripts\publish-apk.ps1 -Changelog "<用户可见改动>"`，再运行 `scripts\check-task-complete.ps1 -Kind AndroidFeature`。
- 脚本已经负责版本 claim/finish、构建、上传、并发保护和清理。AI 不要手搓这些步骤。

## Codex 记忆边界

Codex/Codex CLI 不会天然记住本项目流程。需要长期生效的规则必须写进仓库入口、专项说明文档、脚本或后端注入 prompt；不要只靠一次聊天说明。
