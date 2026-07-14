# 一龙项目 AI 工作入口

最后更新：2026-07-14

本文件是 Codex、Codex CLI、Copilot、Copilot CLI 等工具的共享路由层。

**权威规则：`.github/copilot-instructions.md`。** 所有写任务必须遵守其中 `WF-START` 至 `WF-REPORT`；本文件不复制命令和完成步骤。

## 必读顺序

1. 读 `.github/copilot-instructions.md`。
2. 读本文件，只选择当前任务对应的专项文档。
3. Codex CLI 再读 `CODEX.md`。
4. 不固定全量读取 `.github/instructions/`、`docs/`、Prompt、Agent 或 Skill。

## 任务路由

| 任务类型 | 继续读取或执行 |
|---|---|
| 项目定位、架构、模块入口、任务模板 | `AI_PROJECT.md`、`AI_ARCHITECTURE.md`、`AI_INDEX.md`、`AI_TASK_TEMPLATE.md` |
| 项目文档、笔记、权威性、归档、低 token 整理 | `.github/instructions/document-authority.instructions.md` |
| Git、worktree、提交、push、部署、发布 | `.github/instructions/git-deploy-workflow.instructions.md` |
| Rust 验证、Cargo、格式化 | `.github/instructions/git-deploy-workflow.instructions.md` 对应章节 |
| PowerShell 版本兼容 | `docs/powershell-version-policy.md` |
| 模块化、拆文件、巨型文件 | `.github/instructions/modular-architecture.instructions.md` |
| 后端架构、API、数据流 | `docs/system-architecture.md` 和相关源码 |
| PC 工作台、`/pc`、React/Vite/TypeScript | `.github/instructions/pc-frontend-migration.instructions.md`、`docs/pc-frontend-migration.md`、`AI_INDEX.md` |
| Windows PC 节点客户端、自更新、推送更新 | Git/发布手册和相关源码 |
| 完整开发流程、复杂发布或任务卡住 | `docs/ai-agent-workflow.md` |
| Android APK 发布 | Git/发布手册的发布入口；环境异常再读 `docs/android-setup.md` |
| APP UI、主题、颜色、导航、卡片 | `docs/Design.md`、`docs/APP 颜色规范.md`；涉及 APK/网页同步再读 `.github/instructions/apk-web-ui-sync.instructions.md` |
| APP 当前版本记忆 | `docs/app-version-memory.md` |
| Copilot Customizations | 仅按目标读取 `.github/prompts/`、`.github/agents/`、`.github/skills/` |
| Prompt/Agent/Skill token 去重 | 先运行 `scripts/audit-ai-prompt-assets.ps1`，再读报告命中的文件 |
| 聊天记录、时间线、下载地址溯源、慢任务诊断 | `docs/query-chat-records.md` |

## 脚本信号优先

- 预检输出的 `EDIT_ROOT` 是唯一编辑根；平台 `conversation-worktrees` / `ai/session` 已隔离时不创建嵌套 worktree。
- 收尾必须执行预检输出的 `FINISH_COMMAND_*`，只有 `FINALIZABLE=true` 才可正常宣告完成。
- `origin/main` 前进不触发追车；只有 push 被 non-fast-forward 拒绝才 rebase。
- 业务状态与本机收尾状态分开报告；未知主工作区文件不自动提交、删除或忽略，也不再阻止无冲突的 `main` 快进。
- 脚本头有 `#requires -Version 7.0` 时使用 `pwsh`；其他 Windows bootstrap 脚本可用 `powershell.exe`。

修改预检、统一收尾、worktree 清理或这些工作流文档后，必须运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-ai-task-preflight-workflow.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\audit-ai-prompt-assets.ps1
```
