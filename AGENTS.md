# 一龙项目 AI 工作入口

本文件是 VS Code Copilot、Codex、Claude Code 等多 AI 工具共享的工作入口。  
**最后更新：2026-05-26**

---

## 🔒 最高级铁律

模块化、文件行数限制、职责拆分规则见 `.github/instructions/modular-architecture.instructions.md`（已 `applyTo: "**"` 自动加载）。

---

## ⚡ 任务开始前必做（无论哪台 PC）

共享文档里不要写某台 PC 的绝对路径。AI 如果已经在本仓库或任意子目录里启动，直接用当前 Git 仓库根目录；如果不是，先用本机环境变量 `ELON_REPO_PATH` 指向真实仓库。

```powershell
$repo = git rev-parse --show-toplevel 2>$null
if (-not $repo) { $repo = $env:ELON_REPO_PATH }
if (-not $repo) { throw "未找到一龙仓库：请在仓库目录启动，或先设置 ELON_REPO_PATH" }
Set-Location -LiteralPath $repo
powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree
```

如果脚本输出 `WORKTREE_CREATED=true`，必须切到 `WORKTREE_PATH` 继续本次任务；不要在原主工作区改代码。

Linux / macOS / 服务器 Codex CLI：

```bash
repo="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [ -z "$repo" ]; then
  repo="${ELON_REPO_PATH:-}"
fi
if [ -z "$repo" ]; then
  echo "未找到一龙仓库：请在仓库目录启动，或先设置 ELON_REPO_PATH" >&2
  exit 1
fi
cd "$repo"
bash scripts/ai-task-preflight.sh --create-worktree
```

同步规则：工作区干净用 `git pull --rebase origin main`；有未提交改动时按 `.github/instructions/git-deploy-workflow.instructions.md` 的归属规则处理（stash 或新建 worktree）。

### 🛡️ 新 clone 必须装一次 git hook（无论 Windows / Linux / 服务器 codex CLI）

```powershell
# Windows
pwsh scripts/install-hooks.ps1
```
```bash
# Linux / macOS / 服务器 codex CLI
bash scripts/install-hooks.sh
```

> 不依赖 AI 是否阅读文档：装好后，`git push` 会被 hook 自动拦截
> ——cargo check 失败或有未 add 的 `.rs` 文件时**直接拒绝 push**。
> 这是机器强制的防漏 add 守门（修复 56bad51 类事故）。
> v0.3.69+ 起，版本号由服务器 `/api/release/claim` 原子分配，不再写入 git，
> 因此 hook 不再检查 `build.gradle` 和 `server/Cargo.toml` 的 version 字段。
> 部署侧由服务器 flock 互斥锁 + CAS（compare-and-swap）保证不会并发覆盖。

---

## 🚀 服务端改动部署流程（铁律：先 git 再 deploy）

```
改代码
  → git status --short | Select-String "^\?\?"  ← ⚠️ 检查是否有新建文件未 add
  → git add <只加自己改的文件，含新建 .rs 文件>
  → git commit -m "type(scope): 描述"
  → git push origin main
  → 根据系统选择部署脚本（脚本会自动从服务器 claim 版本号，编译期注入，不写入 git）：
       Windows:       cd scripts; .\publish-server.ps1
       Linux/macOS:   bash scripts/publish-server.sh
  → 验证健康: curl --noproxy '*' http://43.139.149.158:8080/health
  → 验证后端版本: curl --noproxy '*' http://43.139.149.158:8080/api/server/version
```

---

## 📱 APK 新功能发布闭环（不能只 PR / Debug）

只要任务修改了 APK 用户可安装端能力，就不能停在 PR、分支、`assembleDebug` 或“代码已提交”。

适用范围包括但不限于：

- `android/app/src/main/**`
- `android/app/src/main/AndroidManifest.xml`
- Android 聊天链路、调试能力、更新链路、权限、前台/后台服务
- 任何用户需要在手机上安装后才能验证的新功能或修复

完成定义必须是：

```powershell
scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
```

最终汇报必须写清楚：

- APK 发布状态：已发布 / 未发布（原因）
- APK 版本：`versionName` + `versionCode`
- 发布 commit SHA
- 服务器 `/app/version.json` 校验结果
- 服务器 `/app/version.json` 中的 `gitSha` 是否等于发布 commit
- APK 下载地址

除非用户明确说“只改代码，不发布 APK”，否则 Android 新功能默认必须跑完上面的发布闭环。

---

## 🔑 关键信息速查

| 项目 | 值 |
|---|---|
| Git 远端 | `git@github.com:ElonQian1/Elon.git` |
| 主分支 | `main` |
| 服务器 SSH | `root@43.139.149.158`（加 `-o ProxyCommand=none` 绕代理） |
| 服务器端口 | `8080` |
| 健康检查 | `curl --noproxy '*' http://43.139.149.158:8080/health` |
| APK 版本信息 | `curl --noproxy '*' http://43.139.149.158:8080/app/version.json` |
| 后端版本信息 | `curl --noproxy '*' http://43.139.149.158:8080/api/server/version` |
| APK 下载 | `http://43.139.149.158:8080/app/ElonSpeed-latest.apk` |
| 服务日志 | `ssh -o ProxyCommand=none root@43.139.149.158 'tail -50 /root/elon-server.log'` |

---

## 📂 必读文件顺序

1. `.github/copilot-instructions.md`：项目定位、当前状态、部署速查、全局 AI 原则。
2. `.github/instructions/git-deploy-workflow.instructions.md`：多 AI 并发、worktree 隔离、push 冲突处理。
3. `.github/instructions/modular-architecture.instructions.md`：模块化、巨型文件治理、长期维护边界。
4. `docs/ai-agent-workflow.md`：需求分析→代码修改→编译→部署完整流程。
5. `docs/system-architecture.md`：架构、模块边界、数据流。

---

## 🤖 服务器 Codex CLI 记忆规则

- Codex CLI 不会天然“记住”本项目流程；它每次进入项目时，必须先读取当前仓库里的 `AGENTS.md`、`CODEX.md`、`.github/instructions/*.md` 和相关 `docs/`。
- 本仓库把流程记忆固化在文件里；以后修改 Git、构建、部署、发布规则时，必须同步更新这些说明文件并提交。
- APK 里的一龙项目只是一个默认登记的 `local_path` 项目，和其他本地/GitHub 项目走同一条项目通道，不使用隐藏旁路。
- 任意本地项目都可以通过服务器环境变量 `ELON_PROJECT_<项目ID大写并把非字母数字替换为_>_PATH` 指向真实 Git 工作区。
- `local_path` / GitHub 项目必须是真实 Git 仓库，包含 `.git` 和可用远端；服务器不会为这类项目偷偷 `git init`。

---

## 后端指挥 Codex CLI 的方式

- 后端负责“能不能做、在哪做、按什么顺序做”：检查项目、Git、权限、队列和锁。
- Codex CLI 负责“怎么改代码”：进入指定项目目录，先读项目文档，再按任务修改、验证、提交。
- 每次 APK 任务都要由后端把通用项目工作流写进 CLI 提示词；不能假设 CLI 自动记得上一轮讨论。
- APK 代码开发请求进入 Codex CLI 前，后端会做低成本源文件体量预检，只把超过 500 行的摘要注入提示词；Codex 必须先按 5-15 行文件计划控制 500/800/1500 行边界，再写代码。
- 以后即使接入其他 AI 模型，它们也只能作为旁路工具做轻量分类、摘要、图片或特殊分析；最终用户消息、旁路结论和后续动作必须整理后回灌到同一 APK 会话绑定的 Codex CLI 原生 session，不能打断 Codex CLI 上下文。
- CLI 返回后，后端继续负责任务状态、下载链接、版本号、发布、部署和并发保护。

---

## APK Codex Session Prewarm

- When the APK opens or resumes a project conversation, it may call `/api/.../prewarm` to create or reuse the native Codex CLI session id for that project/user/conversation.
- Prewarm is not a development task. It must not inspect files, run Git, edit code, build, deploy, publish, or inject the full project workflow.
- The first real user message still decides the route: ordinary chat uses the lightweight Codex prompt, and development requests enter the queued project workflow.
- The server records whether a native Codex session has already received the full chat/development bootstrap. Later turns in the same session should use the shorter resume prompt, and stale/expired native sessions should be marked stale and retried once with a fresh Codex session.
- When retrying with a fresh native session after stale resume, include the previous `codex://threads/<thread_id>` URI plus recent backend conversation messages so the new session can bridge the old context.
- Every traced APK/Web task should expose where time went. Preserve `trace_id` through backend routing into Codex CLI calls and record `codex_cli_start`, `codex_cli_done`, `codex_cli_error`, `codex_cli_retry`, and prewarm hit/skip events with prompt size, session hit/miss, operation, attempt, and elapsed time.

## Build And Attachment Transport Rules

- Desktop Codex work must build locally: `scripts/publish-server.ps1` cross-compiles the Rust server locally and uploads the binary; `scripts/publish-apk.ps1` builds/signs the Android APK locally and uploads the APK plus `version.json`.
- The production server is low-spec. Do not use it as the normal Rust release build machine or Android build machine.
- Shared scripts must not hardcode one PC's drive letter. Machine-specific build paths belong in local environment variables or untracked `.env.local`; for server builds prefer `RUST_SERVER_MUSL_TARGET_DIR` when sharing one musl Cargo target across Rust backend repos, and use `ELON_BUILD_TARGET_DIR` only for this repo's legacy per-project cache root.
- APK project messages with photos/files should upload attachments first through HTTP, then send only server-side attachment references in the chat payload. Avoid putting large base64 blobs into the project WebSocket message.

## 🌐 Android 编译环境首次配置（每台新机器必做）

> 详见 `docs/android-setup.md`。此配置仅新机器运行一次，包含 Gradle 测速、Wrapper 缓存修复和全局镜像配置。

## VS Code 快捷入口

- 常规代码任务：运行 `/elon-dev-task`。
- APK 发布任务：运行 `/elon-apk-release`。
- 只做规划：选择 `elon-planner` agent。
- 执行实现：选择 `elon-implementer` agent。
- 提交前审查：选择 `elon-reviewer` agent。
- 可移植技能入口：启用 Agent Skills 后使用 `cloud-apk-dev` skill。
- 跨项目模块化治理：把 `.github/skills/modular-long-term-dev` 复制到其他项目，或在支持 Agent Skills 的工具里启用 `modular-long-term-dev` skill，防止 AI 继续制造巨型文件。

## VS Code Customization 检查

- 用 `Chat: Open Customizations` 打开 Agent Customizations editor。
- 用 diagnostics 视图确认 instructions、prompts、agents、skills 都已加载且没有 frontmatter 错误。
- 如果只打开了 `android/`、`server/` 等子目录，启用 `chat.useCustomizationsInParentRepositories`，让 VS Code 发现仓库根目录 `.github` 配置。
- Agent Skills 需要 `github.copilot.chat.skillTool.enabled`；本项目官方路径是 `.github/skills/cloud-apk-dev/SKILL.md`。
- Hooks 目前是 Preview；没有明确需求前，不在仓库启用会自动执行命令的 hooks。
