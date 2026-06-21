# Win 端 Codex 桌面版开发能力计划

## 目标

让用户安装 Win 端一龙 PC 节点后，可以在 PC 网页端绑定本机项目，选择运行路线和权限，在 AI 开发频道中像 Codex 桌面版一样发起、停止、继续开发任务，并能清楚看到当前是否真正开发就绪。

## 里程碑

1. 核验当前能力：安装包、节点进程、本机状态接口、项目绑定、运行路线、权限和任务卡。
2. 补齐可见就绪证据：在 PC 项目成员栏显示逐项检查，而不是只显示粗略 Ready/Check。
3. 强化任务闭环：保留停止、继续草稿、刷新和运行中状态。
4. 显式运行路线：PC AI 开发栏提供 Auto / Route A / Route B / Route C 选择，并把选择传到后端执行链路。
5. 结构化工具协议：Route B/C 的 `run_command` 优先使用 `program + args`，保留旧 `command` 兼容。
6. 发布闭环：提交到 `origin/main`，发布服务器和 Windows 节点包，验证线上版本和本机安装态。
7. 本机管理安全：7799 本地管理/电脑医生写接口要求启动期随机 token 和 trusted origin 校验，前端自动刷新并携带授权头。
8. 工具时间线：Route B/C 本机 runtime 把 `tool_call/tool_result` 结构化回传，AI 开发频道持久化并在 PC 页面以工具卡片展示。
9. 本机 CLI 执行边界：Route A 只允许已发现的 `codex` / `copilot` / `claude` / `gemini`，拒绝云端传来的任意可执行名、危险提权参数和裸 cwd；legacy relay 同步收紧。
10. Codex Desktop 体验补齐：Route B/C 增加 `apply_patch`、diff preview、逐工具审批、超时/拒绝回传和可恢复任务。

## 风险

- 主工作区存在未推送本地提交和冲突中间态，所有新改动必须在干净 worktree 中完成。
- 完全访问模式会扩大本机读写和命令执行范围，必须保留显式确认和可见提示。
- 服务器运行时代码变更需要服务器发布；PC 前端或节点行为变更还需要刷新 Windows 客户端包。
- 并行任务可能继续推进 `origin/main`，发布时必须以最新主线为准。
- Route B/C 仍不是完整 OS 沙箱；本阶段只减少 shell 注入和命令解析歧义，不把 B/C 扩展成通用 PowerShell。
- 本机 `/api/status` 只应向受信任云端来源或本机同源页面返回启动期随机 token；若可信 PC 网页自身出现 XSS，仍需要后续补本机确认弹窗/原生授权页进一步收紧。
- Route A 的 `full_access` 不能只靠云端字段放大权限，后续需要本机原生确认或配对确认，避免网页/XSS 直接触发全盘级开发能力。
- legacy relay 是兼容路径，不应承载 Route B/C 内置 runtime；若收到内置 runtime 请求必须 fail-closed，让用户升级到一龙 PC 节点客户端。

## 验证命令

- `node --check server\src\assets\pc_app_project_readiness.js`
- `node --check server\src\assets\pc_app_dev_composer.js`
- `node --check server\src\assets\pc_app.js`
- `node scripts\test-pc-dev-assets.js`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_runtime_events`
- `cargo test --manifest-path server\Cargo.toml --bin elon-server pc_agent_runtime_choice`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_tool_guard`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_cli_security`
- `cargo test --manifest-path server\Cargo.toml --bin elon-server node_agent_cli_security`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node api_runtime_config`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_local_admin`
- `cargo test --manifest-path server\pc-dev-runtime\Cargo.toml`
- `git diff --check`
- `cargo check --manifest-path server\Cargo.toml --bin elon-server --bin elon-pc-node`
- `powershell -ExecutionPolicy Bypass -File scripts\publish-server.ps1`
- `powershell -ExecutionPolicy Bypass -File scripts\publish-node-agent.ps1`
- 验证 `/api/server/version`、`/api/node-agent/version`、`/assets/pc_app_project_readiness.js` 和本机 `http://127.0.0.1:7799/api/status`

## 回滚策略

- 前端 UI 回滚：撤回本阶段提交并重新发布服务器静态资源。
- 节点包回滚：重新执行 `scripts\publish-node-agent.ps1` 于上一稳定 SHA。
- 本机安装回滚：从上一稳定 Windows client zip 重新同步 `%LOCALAPPDATA%\ElonNode`，只保留一个 `--agent-runtime` 进程。
- 主工作区不同步：不 reset、不 stash、不覆盖，保留现状并报告阻塞文件。
