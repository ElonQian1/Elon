---
name: elon-apk-release
description: 按一龙项目规则构建并发布 Android APK
agent: elon-implementer
argument-hint: "<发布原因或用户需求>"
---

你要按一龙项目 Android APK 发布流程完成发布：`${input:release_reason:请输入发布原因}`。

必须先读取并遵守：

- [全局项目指令](../copilot-instructions.md)
- [Git + 部署强制工作流](../instructions/git-deploy-workflow.instructions.md)
- [AI 代理完整工作流](../../docs/ai-agent-workflow.md)

发布要求：

1. 先 `git fetch origin main` 并确认 Git 状态；保护所有不属于本任务的未提交改动，来源不明时从 `origin/main` 新建 worktree。
2. 使用 `scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"`，不要手工拼接版本号、签名和上传步骤。
3. 发布脚本必须完成 `versionCode/versionName` 递增、release APK 构建、release commit、`HEAD:main` 推送、APK/version.json 上传和服务器校验。
4. 发布后运行 `powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature`。
5. 如果本次在隔离 worktree 完成，回到原主工作区用 `git fetch origin` + `git pull --ff-only origin main` 同步已跟踪文件，不碰未跟踪文件。
6. 结束时汇报 APK 发布状态、版本号、commit SHA、push 状态、主工作区同步状态、构建结果、服务器校验结果、APK 地址。
