# Win 端 Codex 桌面版开发能力进度

## 当前状态

- 工作目录：`D:\rust\active-projects\elon cli`
- 本阶段目标：把 Route B 自研 API runtime 接入 PC 项目开发真实链路，并校准 Route B/C 权限与前端文案。
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

## 本轮小目标

补齐 PC 项目“开发就绪”的逐项核验显示，让用户能直接判断当前安装和项目是否可以像 Codex 桌面版一样开发。

## 待完成

- 提交、推送、发布服务器和 Windows 节点包。
- 验证线上版本、节点包版本、本机安装目录和本地节点状态。
- 下一阶段继续评估 Route B/C 是否要做任务级二次确认后再开放真正跨目录/full-access 工具，而不是直接取消本机白名单。

## 当前阻塞

- 无功能实现阻塞。
- 无工作区阻塞。
