# Elon AI Rules Bridge

本文件不复制规则，避免多套规则漂移。规则权威来源如下：

1. `AGENTS.md`：所有 AI 工具的共享入口和路由层。
2. `.github/copilot-instructions.md`：本仓库硬规则权威来源。
3. `.github/instructions/*.instructions.md`：按任务类型读取的专项规则。

## 必须记住的硬边界

- 修改代码前先跑预检脚本并使用隔离 worktree。
- 不回退、不覆盖其他 AI 或用户的未提交改动。
- 新建文件必须显式 `git add`。
- 后端运行代码改动必须 commit、push、CodePushed 检查后再发布。
- 不手动递增 `server/Cargo.toml`、`build.gradle` 版本号。
- Android UI 改动需要考虑 APK 与 Web 同步规则。
- 新逻辑按职责拆模块，不制造新的巨型文件。

如本文件与 `.github/copilot-instructions.md` 冲突，以 `.github/copilot-instructions.md` 为准。
