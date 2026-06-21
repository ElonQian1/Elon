# Win 端 Codex 桌面版开发能力计划

## 目标

让用户安装 Win 端一龙 PC 节点后，可以在 PC 网页端绑定本机项目，选择运行路线和权限，在 AI 开发频道中像 Codex 桌面版一样发起、停止、继续开发任务，并能清楚看到当前是否真正开发就绪。

## 里程碑

1. 核验当前能力：安装包、节点进程、本机状态接口、项目绑定、运行路线、权限和任务卡。
2. 补齐可见就绪证据：在 PC 项目成员栏显示逐项检查，而不是只显示粗略 Ready/Check。
3. 强化任务闭环：保留停止、继续草稿、刷新和运行中状态。
4. 发布闭环：提交到 `origin/main`，发布服务器和 Windows 节点包，验证线上版本和本机安装态。

## 风险

- 主工作区存在未推送本地提交和冲突中间态，所有新改动必须在干净 worktree 中完成。
- 完全访问模式会扩大本机读写和命令执行范围，必须保留显式确认和可见提示。
- 服务器运行时代码变更需要服务器发布；PC 前端或节点行为变更还需要刷新 Windows 客户端包。
- 并行任务可能继续推进 `origin/main`，发布时必须以最新主线为准。

## 验证命令

- `node --check server\src\assets\pc_app_project_readiness.js`
- `node --check server\src\assets\pc_app_dev_composer.js`
- `node --check server\src\assets\pc_app.js`
- `git diff --check`
- `cargo check --manifest-path server\Cargo.toml --bin elon-server`
- `powershell -ExecutionPolicy Bypass -File scripts\publish-server.ps1`
- `powershell -ExecutionPolicy Bypass -File scripts\publish-node-agent.ps1`
- 验证 `/api/server/version`、`/api/node-agent/version`、`/assets/pc_app_project_readiness.js` 和本机 `http://127.0.0.1:7799/api/status`

## 回滚策略

- 前端 UI 回滚：撤回本阶段提交并重新发布服务器静态资源。
- 节点包回滚：重新执行 `scripts\publish-node-agent.ps1` 于上一稳定 SHA。
- 本机安装回滚：从上一稳定 Windows client zip 重新同步 `%LOCALAPPDATA%\ElonNode`，只保留一个 `--agent-runtime` 进程。
- 主工作区不同步：不 reset、不 stash、不覆盖，保留现状并报告阻塞文件。
