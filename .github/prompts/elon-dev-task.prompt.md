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

1. 先运行任务预检脚本：Windows 用 `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`，Linux/macOS/服务器 CLI 用 `bash scripts/ai-task-preflight.sh --create-worktree`。脚本会同步 `main` 基线并创建/复用安全工作区。
2. 如果预检输出 `WORKTREE_CREATED=true`，必须切到 `WORKTREE_PATH` 后再定位和修改文件；不要在主 `main` 工作区直接编辑业务文件。
3. 定位并阅读目标文件，不要盲改。
4. 避免继续向巨型文件追加逻辑；触碰 1500 行以上文件时，除小修外优先抽出本次职责模块。
5. 只做当前任务需要的最小修改。
6. 根据影响范围运行最小有效验证。
7. 如任务修改后端运行代码，不要递增 `server/Cargo.toml` 的 `version`；版本号由服务器 release API 分配。提交并 push 后先用 `scripts\check-task-complete.ps1 -Kind CodePushed` 校验代码已进入远端，再按需运行发布脚本。
8. 只 stage 当前任务文件并 commit。
9. push 到 `origin/main`；只有 push 被 non-fast-forward 拒绝时，才 `git fetch origin` + `git rebase origin/main`，解决冲突后重推。不要因为 `origin/main` 在编码或构建期间前进就主动 rebase、重跑验证或重新发布。
10. 如果本次在隔离 worktree 完成，回到原主工作区用 `git fetch origin` + `git pull --ff-only origin main` 同步已跟踪文件，不碰未跟踪文件。
11. 如任务修改 APK 可安装端能力，至少运行 `scripts\check-task-complete.ps1 -Kind CodePushed`；明确负责 APK 发布时再运行 `scripts\publish-apk.ps1` 和 `scripts\check-task-complete.ps1 -Kind AndroidFeature`。发布被更新 main 超越时汇报“代码已合并，发布交给最新主线”，不要反复 rebase 重跑。
12. 结束时汇报提交 SHA、push 状态、主工作区同步状态、验证结果、部署状态；Android 任务汇报 APK 发布状态（已发布 / 被更新 main 接管 / 未尝试），发布成功时再汇报版本号和下载地址；后端发布成功时汇报服务器版本接口结果。
