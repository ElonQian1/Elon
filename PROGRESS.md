# Win 端 Codex 桌面版开发能力进度

## 当前状态

- 工作目录：`D:\rust\active-projects\elon cli`
- 本阶段目标：把 Route B/C 的 `run_command` 从优先 shell 字符串推进到结构化 `program + args` 协议，减少命令注入和解析歧义。
- 当前分支：`main`，提交前需要先与最新 `origin/main` 对齐。

## 已完成

- 已读取 `AGENTS.md`、`.github/copilot-instructions.md`、`docs/ai-agent-workflow.md`、`docs/符号索引讨论/我们项目的cli能力.md`。
- 已确认仓库根目录没有 `README.md` 和根 `Cargo.toml`，Rust 入口在 `server/Cargo.toml` 与 `server/pc-dev-runtime/Cargo.toml`。
- 已启动 Explorer / Reviewer / Tester 三个只读子代理并行审查。
- 已确认现有 PC 能力包括：项目绑定、AI 开发频道、Route A/B/C 展示、完全访问授权、任务停止、任务继续草稿、Windows 节点包发布。
- 已实现 PC 项目“开发就绪”逐项核验清单：项目目录、PC 节点、节点在线、运行路线、开发频道、执行权限。
- 已新增 `scripts/test-pc-dev-assets.js`，覆盖任务继续按钮、就绪清单和开发作业栏 Route/权限文案。
- 已通过 `node --check`、`git diff --check`、`node scripts\test-pc-dev-assets.js` 和 `cargo check --manifest-path server\Cargo.toml --bin elon-server`。
- Reviewer 发现 Codex resume 会丢 `project_write` sandbox；已修复 `codex_resume_args` 保留 `--sandbox workspace-write`，并补测试。
- Explorer 标出的 P0 已处理：server 侧 PC CLI 调用现在带取消守卫，节点侧按 `req_id` 登记运行中 CLI 子进程并在收到 `Cancel` 后 kill。
- 已通过 `cargo test --manifest-path server\Cargo.toml --bin elon-server cli_prompt_cancel_handle_sends_cancel_for_req_id` 和 `cargo check --manifest-path server\Cargo.toml --bin elon-server --bin elon-pc-node`。
- 已把 Route B `api-runtime` 接入 PC 节点真实执行链路：网页/PC 项目开发触发时可在无 Route A CLI 的情况下选择本机 OpenAI-compatible API runtime。
- 已调整运行路线选择顺序：显式可用 CLI 优先，其次 Route A CLI，再 Route B API runtime，最后 Route C 服务器模型。
- 已给 Route C/server-runtime 和 Route B/api-runtime 共用的 runtime loop 接入取消信号，停止任务时不再只停 UI。
- 已将 Route B readiness 与实际执行对齐：必须同时存在 API key 和 model，API base 可默认 `https://api.openai.com/v1`。
- 已把 Route B/C 的本地工具权限改成只认可 `project_write` / `full_access` 两个已知值；未知权限默认只读。
- 已修正 PC 和网页端 Route B/权限文案：Route B 显示为“本机 API runtime”，`full_access` 明确说明 Route B/C 仍保留本机路径和命令白名单。
- 已通过 `node scripts\test-pc-dev-assets.js`、`cargo test --manifest-path server\pc-dev-runtime\Cargo.toml`、`cargo test --manifest-path server\Cargo.toml --bin elon-server route_`、`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node api_runtime_config`、`command_policy`、`tool_guard_only_known_runtime_permissions_enable_project_tools`、`safe_path_stays_inside_workspace`、`cargo check --manifest-path server\Cargo.toml --bin elon-server --bin elon-pc-node`、`git diff --check`。
- 已将 PC 节点 Route B/C 本地工具守卫拆到 `server/src/node_agent_tool_guard.rs`，让权限模式、路径限制、命令白名单和测试不再堆在 runtime loop 大文件里。
- 已短期硬化 Route B/C 命令与路径边界：拒绝 PowerShell 元字符、绝对路径参数、`..` 路径段、大小写变体 `.git` 以及 symlink/reparse-point 祖先路径。
- 已同步硬化 `pc-dev-runtime` 生成的 `scripts\elon-agent.ps1`，新生成项目不会继续使用更松的 PowerShell 白名单。
- 已把 PC/网页端 `full_access` 确认语改成真实边界：Route A 可按授权绕过项目沙箱；Route B/C 仍保留项目路径和命令白名单，但 build/test 会执行项目代码。
- 已通过本阶段验证：`node scripts\test-pc-dev-assets.js`、`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_tool_guard`、`cargo test --manifest-path server\pc-dev-runtime\Cargo.toml`、`cargo test --manifest-path server\Cargo.toml --bin elon-server pc_agent_runtime_choice`、`cargo check --manifest-path server\Cargo.toml --bin elon-server --bin elon-pc-node`、`git diff --check`。
- 已在 PC AI 开发栏新增自动 / A / B / C 分段路线选择：自动模式不传参，强制 A/B/C 时通过 `runtimeRoute` 进入频道 AI 任务 API。
- 已让后端 `StartChannelAiTaskRequest` 解析 `runtimeRoute`，并在 PC 项目快速路径里强制选择对应 Route；Route 不可用时返回可读错误，不再悄悄回退到 Route A。
- 已补 Route 选择单测：强制 Route B 会跳过可用 Route A；强制不可用 Route C 会返回明确“未就绪”错误。
- 已补 PC 前端资产测试：默认自动模式、Route B 标签、强制 Route C 本地偏好和请求参数输出。
- 已通过本阶段验证：`node --check server\src\assets\pc_app_dev_composer.js`、`node --check server\src\assets\pc_app.js`、`node --check scripts\test-pc-dev-assets.js`、`node scripts\test-pc-dev-assets.js`、`cargo test --manifest-path server\Cargo.toml --bin elon-server pc_agent_runtime_choice`、`cargo check --manifest-path server\Cargo.toml --bin elon-server --bin elon-pc-node`、`git diff --check`。
- 已让 Win 节点内置 Route B/C `ToolGuard` 支持结构化 `run_command`：模型可返回 `program` 和 `args`，节点直接用 `Command::new(program).args(args)` 执行，不再需要 shell 拼接。
- 已保留旧版 `command` 字符串兼容路径，但提示词和脚手架文档改为优先 `program + args`。
- 已同步 `pc-dev-runtime` 生成的 `scripts\elon-agent.ps1`：项目本地 Route B/C 也支持结构化命令，并沿用确认、DryRun、路径和命令白名单。
- 已补结构化命令策略测试：允许 `git status --short`、`cargo test --all-features`、`npm run build`、Gradle assembleDebug；拒绝未知程序、shell 连接符、绝对路径和 `..` 路径。
- 子代理复核结论：当前已有本地节点、Route A/B/C、工具守卫、会话 worktree 和任务卡片骨架；距离 Codex Desktop 最大差距仍是工具时间线、逐工具审批、持久恢复和 apply_patch/diff 体验。
- 已通过本阶段验证：`rustfmt --edition 2021` 针对本次 Rust 文件、`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_tool_guard`、`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node api_runtime_config`、`cargo test --manifest-path server\pc-dev-runtime\Cargo.toml`、`node --check scripts\test-pc-dev-assets.js`、`node scripts\test-pc-dev-assets.js`、`cargo check --manifest-path server\Cargo.toml --bin elon-server --bin elon-pc-node`、`git diff --check`。

## 本轮小目标

先发布并验证 Route B/C 结构化命令协议；下一阶段继续推进工具时间线、逐工具审批、apply_patch/diff 预览和持久任务恢复。

## 待完成

- 提交、推送、发布服务器和 Windows 节点包。
- 验证线上版本、节点包版本、本机安装目录和本地节点状态。
- 下一阶段实现 Route B/C 工具时间线和逐工具审批，而不是只输出 `[tool] xxx` 粗粒度文本。
- 下一阶段补 `apply_patch` / diff preview / read_file_range 等更像 Codex Desktop 的文件编辑工具。

## 当前阻塞

- 无功能实现阻塞。
- 无工作区阻塞。
