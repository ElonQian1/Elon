# Win 端 Codex 桌面版开发能力进度

## 当前状态

- 工作目录：`D:\rust\active-projects\elon cli`
- 本阶段目标：继续把 Win 端开发能力逼近 Codex Desktop；本轮聚焦 Route B/C 的范围读取能力和补丁路径安全。
- 当前分支：`codex/win-codex-parity-stage2-20260621`，基于最新 `origin/main` 的隔离 worktree。

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
- 本轮已新增 Route B/C 工具事件格式化模块，节点 runtime 执行 `list_dir/read_file/write_file/run_command` 前后会回传结构化 `tool_call/tool_result`。
- 本轮已让 PC CLI 桥接层透传结构化工具事件，并让 AI 开发频道把工具事件写入 `task_events` 与 `ai_progress` 频道消息。
- 本轮已让 PC 任务卡识别工具事件 JSON，渲染为工具调用/工具结果卡片，而不是裸露 JSON 文本。
- 本轮已把 `/api/status` 改为只在受信任云端来源或本机同源浏览器上下文中返回 `local_admin_token`；普通无浏览器来源请求仍能看状态但拿不到管理 token。
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
- 已新增 `server/src/node_agent_local_admin.rs`：启动期随机 `X-Elon-Local-Admin-Token`、trusted origin 校验、缺失/过期 token 拒绝和单测。
- 已把 7799 本地管理路由拆成公开探测和受保护管理两组：`/api/status` 继续用于发现节点，环境安装、登录登出、项目绑定、电脑医生快照/修复/记忆、存储配置、TTS 配置等都要先通过本机 admin guard。
- 已把 PC 工作台、电脑医生、节点页 fallback 和独立 `node_agent_admin.html` 接入本机 token 自动刷新、授权头注入和 403 后重试一次。
- 已补 `scripts/test-pc-dev-assets.js` 静态断言，防止以后删掉本机 admin token wiring。
- 已通过本阶段验证：`rustfmt --edition 2021 server\src\node_agent_main.rs server\src\node_agent_local_admin.rs`、`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_local_admin`、`node --check server\src\assets\pc_app.js`、`node --check server\src\assets\pc_app_doctor.js`、`node --check server\src\assets\pc_app_node.js`、`node --check scripts\test-pc-dev-assets.js`、`node scripts\test-pc-dev-assets.js`、`cargo check --manifest-path server\Cargo.toml --bin elon-server --bin elon-pc-node`、`git diff --check`。
- 本轮新增 `server/src/node_agent_cli_security.rs`，统一 Route A/B/C CLI 名称、路径、cwd 和参数校验：未知 CLI、路径型 CLI、危险提权参数、裸 cwd、相对 cwd 都会 fail-closed。
- 本轮已让 `elon-pc-node` 的 Route A 执行只使用本机发现并 canonicalize 后的 `codex` / `copilot` / `claude` / `gemini` 路径；`api-runtime` / `server-runtime` 仍走内置 runtime，不需要本机外部可执行文件。
- 本轮已让 plan 模式继续携带项目上下文，但权限降为 `read_only`；节点侧 read-only 不再创建会话 worktree，`project_write/full_access` 才进入隔离 worktree。
- 本轮已把 Codex session 持久化 key 扩展为 `session_id + permission + cwd hash`，避免同一个会话 ID 在不同权限或不同项目目录之间串用。
- 本轮已同步收紧 legacy `pc_relay_client`：复用同一套 CLI 白名单和参数/cwd 校验，明确拒绝内置 runtime，并隐藏 Windows 子进程窗口。
- 本轮已从 PC 节点能力上报里移除 `gh`，避免前端显示一个实际会被安全层拒绝的 CLI。
- 已通过本轮验证：`rustfmt --edition 2021 server/src/main.rs server/src/node_agent_main.rs server/src/node_agent_cli_security.rs server/src/pc_relay_client.rs server/src/ai_cli/mod.rs`、`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_cli_security`、`cargo test --manifest-path server\Cargo.toml --bin elon-server node_agent_cli_security`、`cargo test --manifest-path server\Cargo.toml --bin elon-server pc_agent_runtime_choice`、`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_local_admin`、`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_tool_guard`、`cargo check --manifest-path server\Cargo.toml --bin elon-server --bin elon-pc-node`、`git diff --check`。
- 上一阶段已发布 Route B/C `apply_patch`、diff preview 和逐工具审批，服务器版本 `v0.3.542`，节点包 SHA `563ea870`。
- 本轮子代理审计确认：`read_file_range` 是低风险 Codex Desktop parity 增量；审批状态落库收益更高但牵涉 DB/store/API/UI，适合后续单独阶段。
- 本轮已新增 `server/src/node_agent_file_range.rs`，Route B/C 支持 `read_file_range`，输出带行号片段并限制最多 400 行、24k 字符。
- 本轮已把 `read_file_range` 接入 `ToolGuard` 和 runtime prompt，继续复用项目路径安全校验；非法 `start_line` / `line_count` fail-closed。
- 本轮已修复 `tools_patch` 对 `.git` 路径的 Windows 大小写变体拒绝，`.GIT/config` 等路径现在会被显式拦截。

## 本轮小目标

验证并发布 Route B/C 的 `read_file_range` 和 `tools_patch` `.git` 大小写安全修复。

## 待完成

- 提交、推送、发布服务器和 Windows 节点包。
- 验证线上版本、节点包版本、本机安装目录和本地节点状态。
- 下一阶段实现 Route B/C `write_file` 的真实 diff preview。
- 下一阶段修复审批并发状态机，并评估审批状态落库/恢复。
- 下一阶段补 Route A `full_access` 的本机确认页或原生确认弹窗，不能只由云端请求字段决定。
- 下一阶段补任务恢复和更完整的工具时间线筛选。

## 当前阻塞

- 无功能实现阻塞。
- 无工作区阻塞。
