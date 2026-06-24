# Elon AI Task Template

把复杂任务交给 AI 时，建议使用这个模板。它的目标是让 AI 先定位、再计划、再修改、再验证，避免直接乱改。

```md
## 任务目标

请修改：

## 必读上下文

1. `AGENTS.md`
2. `.github/copilot-instructions.md`
3. `AI_PROJECT.md`
4. `AI_ARCHITECTURE.md`
5. `AI_INDEX.md`
6. 与任务相关的 `.github/instructions/*.instructions.md`

## 工作要求

1. 先运行任务预检脚本：Windows 用 `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`，Linux/macOS/服务器 CLI 用 `bash scripts/ai-task-preflight.sh --create-worktree`。如果输出 `WORKTREE_CREATED=true`，必须切到 `WORKTREE_PATH` 后再修改。
2. 先列出相关文件和为什么相关。
3. 修改前输出文件计划 JSON。
4. 不修改无关文件，不回退他人改动。
5. 新增逻辑优先放入职责明确的小模块。
6. 修改后运行最小有效验证命令。
7. 提交前检查 `git status --short`，确认没有漏加新文件。
8. commit 后 push 到 `origin/main`。
9. 运行 CodePushed 检查。
10. 后端运行代码改动需要发布并验证 `/health`、`/api/server/version`。

## 验证命令

按任务选择：

- `cargo test --manifest-path .\server\Cargo.toml <module-or-test-name>`
- `cargo check --manifest-path .\server\Cargo.toml`
- `git diff --check`
- Android 任务按发布规则使用 `scripts\publish-apk.ps1`

## 最终回复必须包含

- 当前项目情况判断。
- 本次修改了什么。
- 验证结果。
- commit SHA。
- 是否已 push。
- 是否已部署后端或 APK。
- 仍然缺什么。
```
