---
name: elon-apk-release
description: 按一龙项目规则构建并发布 Android APK
agent: elon-implementer
argument-hint: "<发布原因或用户需求>"
---

你要按一龙项目 Android APK 发布流程完成发布：`${input:release_reason:请输入发布原因}`。

必须先读 [AGENTS.md](../../AGENTS.md)，再按它路由到 Android APK 发布相关专项文档；不要固定全量读取无关 docs。

发布要求：

1. 先运行任务预检脚本：Windows 用 `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`，Linux/macOS/服务器 CLI 用 `bash scripts/ai-task-preflight.sh --create-worktree`。如果输出 `WORKTREE_CREATED=true`，必须切到 `WORKTREE_PATH` 后再发布；不要在主 `main` 工作区直接发布或改文件。
2. 使用 `scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"`，不要手工拼接版本号、签名和上传步骤。
3. 发布脚本负责版本 claim、release 构建、上传和服务器校验；版本号不进 git，也不生成 release-only commit。
4. 发布后运行 `powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature`。若脚本提示被更新的 `origin/main` 或服务器 APK 超越，汇报“代码已合并，发布交给最新主线”。
5. 如果本次在隔离 worktree 完成，回到原主工作区用 `git fetch origin` + `git pull --ff-only origin main` 同步已跟踪文件，不碰未跟踪文件。
6. 结束时汇报 APK 发布状态、版本号、commit SHA、push 状态、主工作区同步状态、构建结果、服务器校验结果、APK 地址。
