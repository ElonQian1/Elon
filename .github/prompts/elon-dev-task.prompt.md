---
name: elon-dev-task
description: 按一龙项目强制工作流完成一次代码或文档任务
agent: elon-implementer
argument-hint: "<用户需求或任务描述>"
---

你要按一龙项目工作流完成用户给出的任务：`${input:task:请输入用户需求}`。

必须先读取并遵守：

- [全局项目指令](../copilot-instructions.md)
- [Git + 部署强制工作流](../instructions/git-deploy-workflow.instructions.md)
- [模块化与长期维护规则](../instructions/modular-architecture.instructions.md)
- [AI 代理完整工作流](../../docs/ai-agent-workflow.md)
- [系统架构](../../docs/system-architecture.md)

执行要求：

1. 先运行 `git status --short --branch`，判断主工作区是否有并发改动。
2. 先 `git fetch origin main`；如有不属于本任务或来源不明的未提交改动，使用从 `origin/main` 创建的独立 worktree。
3. 定位并阅读目标文件，不要盲改。
4. 避免继续向巨型文件追加逻辑；触碰 1500 行以上文件时，除小修外优先抽出本次职责模块。
5. 只做当前任务需要的最小修改。
6. 根据影响范围运行最小有效验证。
7. 如任务修改后端运行代码，递增 `server/Cargo.toml` 的 `version`，提交后部署并校验 `/api/server/version`。
8. 只 stage 当前任务文件并 commit。
9. push 到 `origin/main`；如被拒绝，fetch/rebase 或 merge 后重试。
10. 如果本次在隔离 worktree 完成，回到原主工作区用 `git fetch origin` + `git pull --ff-only origin main` 同步已跟踪文件，不碰未跟踪文件。
11. 如任务修改 APK 可安装端能力，继续运行 `scripts\publish-apk.ps1` 和 `scripts\check-task-complete.ps1 -Kind AndroidFeature`，不能只停在 PR 或 Debug 包。
12. 结束时汇报提交 SHA、push 状态、主工作区同步状态、验证结果、部署状态；Android 任务还必须汇报 APK 发布状态、版本号和下载地址，后端任务必须汇报服务器版本接口结果。
