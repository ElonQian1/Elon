# 一龙项目 AI 工作入口

本文件是 VS Code Copilot、Codex、Claude Code 等多 AI 工具共享的工作入口。  
**最后更新：2026-05-24**

---

## ⚡ 任务开始前必做（无论哪台 PC）

```powershell
cd "d:\rust\active-projects\elon cli"
git pull --rebase origin main   # 同步最新代码
git status --short              # 检查是否有其他 AI 未提交的改动
```

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
> 部署侧由服务器 flock 互斥锁 + CAS（compare-and-swap）保证不会并发覆盖。

---

## 🚀 服务端改动部署流程（铁律：先 git 再 deploy）

```
改代码
  → git status --short | Select-String "^\?\?"  ← ⚠️ 检查是否有新建文件未 add
  → git add <只加自己改的文件，含新建 .rs 文件>
  → git commit -m "type(scope): 描述"
  → git push origin main
  → 根据系统选择部署脚本：
       Windows:       cd scripts; .\publish-server.ps1
       Linux/macOS:   bash scripts/publish-server.sh
  → 验证: curl --noproxy '*' http://43.139.149.158:8080/health
```

> **绝对禁止**：改完代码不 commit 直接运行部署脚本 ——  
> 脚本基于 git HEAD 打包，未提交的代码**不会**进入部署。

> **并发保护**：两个脚本都内置 SHA 顺序检查。如果另一台 PC 已部署更新版本，
> 本机旧版编译完成后**会自动中止**，不会回退服务器版本。用 `-Force`/`--force` 强制覆盖。

---

## 🔑 关键信息速查

| 项目 | 值 |
|---|---|
| Git 远端 | `git@github.com:ElonQian1/Elon.git` |
| 主分支 | `main` |
| 服务器 SSH | `root@43.139.149.158`（加 `-o ProxyCommand=none` 绕代理） |
| 服务器端口 | `8080` |
| 健康检查 | `curl --noproxy '*' http://43.139.149.158:8080/health` |
| 版本信息 | `curl --noproxy '*' http://43.139.149.158:8080/app/version.json` |
| APK 下载 | `http://43.139.149.158:8080/app/ElonSpeed-latest.apk` |
| 服务日志 | `ssh -o ProxyCommand=none root@43.139.149.158 'tail -50 /root/elon-server.log'` |

---

## 📂 必读文件顺序

1. `.github/copilot-instructions.md`：项目定位、当前状态、部署速查、全局 AI 原则。
2. `.github/instructions/git-deploy-workflow.instructions.md`：多 AI 并发、worktree 隔离、push 冲突处理。
3. `docs/ai-agent-workflow.md`：需求分析→代码修改→编译→部署完整流程。
4. `docs/system-architecture.md`：架构、模块边界、数据流。

---

## 🤖 服务器 Codex CLI 记忆规则

- Codex CLI 不会天然“记住”本项目流程；它每次进入项目时，必须先读取当前仓库里的 `AGENTS.md`、`CODEX.md`、`.github/instructions/*.md` 和相关 `docs/`。
- 本仓库把流程记忆固化在文件里；以后修改 Git、构建、部署、发布规则时，必须同步更新这些说明文件并提交。
- APK 里的一龙项目只是一个默认登记的 `local_path` 项目，和其他本地/GitHub 项目走同一条项目通道，不使用隐藏旁路。
- 任意本地项目都可以通过服务器环境变量 `ELON_PROJECT_<项目ID大写并把非字母数字替换为_>_PATH` 指向真实 Git 工作区。
- `local_path` / GitHub 项目必须是真实 Git 仓库，包含 `.git` 和可用远端；服务器不会为这类项目偷偷 `git init`。

---

## APK 项目并发规则

- 服务器按 `project_id` 设置项目级执行权：不同项目可以同时运行，同一个项目当前按顺序排队运行。
- 这个排队只保护同一份项目工作区，避免两个手机同时 `git pull`、改文件、commit、push 造成覆盖或冲突。
- `worktree` 仍然是推荐的多 AI 并行开发模型；后续实现同项目多任务 worktree 时，也必须把 merge、版本号递增、APK 发布、服务器部署这些共享动作保留为串行。
- 一龙自项目不走特殊旁路，它与其他 GitHub/local_path 项目遵守同一套并发、Git、构建、发布规则。

## 后端指挥 Codex CLI 的方式

- 后端负责“能不能做、在哪做、按什么顺序做”：检查项目、Git、权限、队列和锁。
- Codex CLI 负责“怎么改代码”：进入指定项目目录，先读项目文档，再按任务修改、验证、提交。
- 每次 APK 任务都要由后端把通用项目工作流写进 CLI 提示词；不能假设 CLI 自动记得上一轮讨论。
- CLI 返回后，后端继续负责任务状态、下载链接、版本号、发布、部署和并发保护。

---

## VS Code 快捷入口

- 常规代码任务：运行 `/elon-dev-task`。
- APK 发布任务：运行 `/elon-apk-release`。
- 只做规划：选择 `elon-planner` agent。
- 执行实现：选择 `elon-implementer` agent。
- 提交前审查：选择 `elon-reviewer` agent。
- 可移植技能入口：启用 Agent Skills 后使用 `cloud-apk-dev` skill。

## VS Code Customization 检查

- 用 `Chat: Open Customizations` 打开 Agent Customizations editor。
- 用 diagnostics 视图确认 instructions、prompts、agents、skills 都已加载且没有 frontmatter 错误。
- 如果只打开了 `android/`、`server/` 等子目录，启用 `chat.useCustomizationsInParentRepositories`，让 VS Code 发现仓库根目录 `.github` 配置。
- Agent Skills 需要 `github.copilot.chat.skillTool.enabled`；本项目官方路径是 `.github/skills/cloud-apk-dev/SKILL.md`。
- Hooks 目前是 Preview；没有明确需求前，不在仓库启用会自动执行命令的 hooks。

---

## 工作原则

- 有未提交并发改动时，用临时 worktree 隔离（见 git-deploy-workflow）。
- 只 stage 当前任务文件，不夹带其他 AI 的改动。
- 每次任务必须 commit + push，部署必须基于已推送的 SHA。
- 不提交密钥、`.env`、APK 签名材料或任何敏感信息。
- push 被拒绝（non-fast-forward）→ `git pull --rebase` 解决后再 push，禁止 `--force`。
