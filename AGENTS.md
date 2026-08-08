# 一龙项目 AI 工作入口

**权威规则：`.github/copilot-instructions.md`。** 遵守 `WF-START` 至 `WF-REPORT`；这里只路由。

## 必读顺序

1. 读 `.github/copilot-instructions.md` 和本文件。
2. 读 `AI_CURRENT.md`，确认事实、提案、禁用路线。
3. 只读命中文档；Codex CLI 再读 `CODEX.md`。
4. 禁止全量读取 instructions、docs、Prompt、Agent、Skill。

## 任务路由

| 任务类型 | 继续读取或执行 |
|---|---|
| 定位/状态/架构/模块/任务模板 | `AI_CURRENT.md`、`AI_PROJECT.md`、`AI_ARCHITECTURE.md`、`AI_INDEX.md`、`AI_TASK_TEMPLATE.md` |
| 文档、笔记、归档、低 token 整理 | `.github/instructions/document-authority.instructions.md` |
| 功能登记、待开发模块、认领、验收 | `docs/project-feature-registry.md`；普通 Codex 用 `project_feature_workflow`，完整治理用 `project_features_list/plan`；禁止全读或手改注册表 |
| 长聊天/讨论分叉/脑图 | `.github/instructions/discussion-knowledge.instructions.md` |
| Git/worktree/提交/push/部署/发布 | `.github/instructions/git-deploy-workflow.instructions.md` |
| Rust/Cargo | Git/发布手册；`scripts/validate-rust.ps1` |
| PowerShell 版本兼容 | `docs/powershell-version-policy.md` |
| 模块化、拆文件、巨型文件 | `.github/instructions/modular-architecture.instructions.md` |
| 后端架构、API、数据流 | `docs/system-architecture.md` 和相关源码 |
| Codex 桌面监督/PC 节点 | 自动派发/续跑暂停；读 `docs/supervised-pc-project-development.md` |
| PC 工作台、`/pc`、React/Vite/TypeScript | `.github/instructions/pc-frontend-migration.instructions.md`、`docs/pc-frontend-migration.md`、`AI_INDEX.md` |
| Windows PC 节点、自更新、推送 | `docs/node-agent-upgrade-compatibility.md`、Git/发布手册、相关源码 |
| 完整流程/复杂发布/任务卡住 | `docs/ai-agent-workflow.md` |
| Android APK 发布 | Git/发布手册的发布入口；环境异常再读 `docs/android-setup.md` |
| APP 低风险视觉微调（Ripple、颜色、间距、圆角、字号、轻动画） | `docs/app-ui-fast-lane.md`；默认不启动真机、模拟器或 Renderer |
| APP 复杂 UI、主题、导航、按图还原 | `docs/Design.md`、`docs/APP 颜色规范.md`；涉及 APK/网页同步再读 `.github/instructions/apk-web-ui-sync.instructions.md` |
| APP 当前版本记忆 | `docs/app-version-memory.md` |
| 一龙自身品牌 Logo 替换 | `docs/brand-logo-workflow.md`；统一运行 `scripts/replace-brand-logo.ps1` |
| Prompt/Agent/Skill | 仅按目标读取；去重先运行 `scripts/audit-ai-prompt-assets.ps1` |
| 聊天记录、时间线、下载地址溯源、慢任务诊断 | `docs/query-chat-records.md` |

## 脚本信号优先

- `EDIT_ROOT` 是唯一编辑根；已隔离时不建嵌套 worktree。
- Gradle/Cargo/npm/发布长命令完整日志写 `.ai-tmp/`；Windows 用 `scripts/invoke-ai-logged-command.ps1`，成功最多回传 20 行，失败 80 行。
- commit 前运行 `scripts/check-source-size.ps1`，pre-push 兜底；禁止提交后才处理巨型入口增长。
- 收尾必须执行预检输出的 `FINISH_COMMAND_*`，只有 `FINALIZABLE=true` 才可正常宣告完成。
- 仅 push 被 non-fast-forward 拒绝时 rebase；不追车，只补受影响验证。
- 保留未知文件；业务/本机状态分报，不重复已推送项。
- 脚本头有 `#requires -Version 7.0` 时使用 `pwsh`；其他 Windows bootstrap 脚本可用 `powershell.exe`。

修改预检、统一收尾、worktree 清理或这些工作流文档后，必须运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-ai-task-preflight-workflow.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\audit-ai-prompt-assets.ps1
```
