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

---

## 🚀 服务端改动部署流程（铁律：先 git 再 deploy）

```
改代码
  → git add <只加自己改的文件>
  → git commit -m "type(scope): 描述"
  → git push origin main
  → cd scripts; .\publish-server.ps1    ← 脚本自动 worktree 构建 + 上传 + 重启
  → 验证: curl --noproxy '*' http://43.139.149.158:8080/health
```

> **绝对禁止**：改完代码不 commit 直接运行 publish-server.ps1 ——  
> 脚本基于 git HEAD 打包，未提交的代码**不会**进入部署。

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
