# 一龙项目 AI 工作入口

本文件是 VS Code Copilot、Codex、Claude Code 等多 AI 工具共享的工作入口。  
**最后更新：2026-05-24**

---

## ⚡ 任务开始前必做（无论哪台 PC）

```powershell
cd "d:\rust\active-projects\elon cli"
git fetch origin main
git status --short --branch     # 检查是否有其他 AI 未提交的改动
```

同步规则：

- 工作区干净：执行 `git pull --rebase origin main` 后开始任务。
- 未提交改动属于本任务：先 `git stash push -u -m "wip-before-sync"`，再 `git pull --rebase origin main`，然后 `git stash pop` 并解决冲突。
- 未提交改动不属于本任务或来源不明：**不要 stash / pull / pop 当前工作区**，必须从 `origin/main` 创建独立 worktree，例如 `git worktree add ..\Elon-task -b codex/task origin/main`。
- 提交前再执行一次 `git fetch origin main` + `git rebase origin/main`，确认本次提交叠在最新远端之上。

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
> Android APK 主分支推送也会被 hook 检查：如果改了 APK 运行代码但没有递增
> `android/app/build.gradle` 里的 `versionCode/versionName`，会拒绝推送到 `main`。
> 后端运行代码主分支推送也会被 hook 检查：如果改了 `server/src/**`
> 但没有递增 `server/Cargo.toml` 的 `version`，会拒绝推送。
> 部署侧由服务器 flock 互斥锁 + CAS（compare-and-swap）保证不会并发覆盖。

---

## 🚀 服务端改动部署流程（铁律：先 git 再 deploy）

```
改代码
  → 如涉及 server/src/** 后端运行代码：递增 server/Cargo.toml 的 version
  → git status --short | Select-String "^\?\?"  ← ⚠️ 检查是否有新建文件未 add
  → git add <只加自己改的文件，含新建 .rs 文件>
  → git commit -m "type(scope): 描述"
  → git push origin main
  → 根据系统选择部署脚本：
       Windows:       cd scripts; .\publish-server.ps1
       Linux/macOS:   bash scripts/publish-server.sh
  → 验证健康: curl --noproxy '*' http://43.139.149.158:8080/health
  → 验证后端版本: curl --noproxy '*' http://43.139.149.158:8080/api/server/version
```

> **绝对禁止**：改完代码不 commit 直接运行部署脚本 ——  
> 脚本基于 git HEAD 打包，未提交的代码**不会**进入部署。

> **并发保护**：两个脚本都内置 SHA 顺序检查。如果另一台 PC 已部署更新版本，
> 本机旧版编译完成后**会自动中止**，不会回退服务器版本。用 `-Force`/`--force` 强制覆盖。

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

发布脚本会自动完成：同步 `main`、递增 `versionCode/versionName`、构建 release APK、提交 release commit、推送 `HEAD:main`、上传 `ElonSpeed-latest.apk` 和 `version.json`、校验服务器版本。

最终汇报必须写清楚：

- APK 发布状态：已发布 / 未发布（原因）
- APK 版本：`versionName` + `versionCode`
- 发布 commit SHA
- 服务器 `/app/version.json` 校验结果
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
- 以后即使接入其他 AI 模型，它们也只能作为旁路工具做轻量分类、摘要、图片或特殊分析；最终用户消息、旁路结论和后续动作必须整理后回灌到同一 APK 会话绑定的 Codex CLI 原生 session，不能打断 Codex CLI 上下文。
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
- 后端运行代码变更必须递增 `server/Cargo.toml` 的 `version`；部署脚本会把 git SHA 注入 `/api/server/version`，APK 会动态展示该后端版本。
- Android 新功能必须完成 APK 发布闭环，不能只停在 PR、Debug 包或本地验证。
- 不提交密钥、`.env`、APK 签名材料或任何敏感信息。
- push 被拒绝（non-fast-forward）→ `git pull --rebase` 解决后再 push，禁止 `--force`。
