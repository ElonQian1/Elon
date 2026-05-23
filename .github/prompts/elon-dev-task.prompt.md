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
- [AI 代理完整工作流](../../docs/ai-agent-workflow.md)
- [系统架构](../../docs/system-architecture.md)

执行要求：

1. 先运行 `git status --short --branch`，判断主工作区是否有并发改动。
2. 如有不属于本任务的未提交改动，使用独立 worktree。
3. 定位并阅读目标文件，不要盲改。
4. 只做当前任务需要的最小修改。
5. 根据影响范围运行最小有效验证。
6. 只 stage 当前任务文件并 commit。
7. push 到 `origin/main`；如被拒绝，fetch/rebase 或 merge 后重试。
8. 结束时汇报提交 SHA、push 状态、验证结果、部署状态。
