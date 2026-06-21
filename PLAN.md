# PC 节点黑窗风暴修复计划

## 目标

修复 Windows PC 端 `一龙PC节点.exe` 在启动、自更新、协议唤起、开机自启和本机 CLI 执行时可能疯狂弹出黑色 cmd/终端窗口的问题，重点处理几十到上百个黑窗的进程风暴。

## 里程碑

1. 启动器防重复：管理端口未健康时先识别当前安装目录已有的 `--agent-runtime` 进程，避免每次协议唤起都继续 spawn 新 runtime。
2. 启动链路隐藏：统一 launcher 的 PowerShell、cmd、浏览器打开、普通子进程隐藏策略，并加 `CREATE_NEW_PROCESS_GROUP`。
3. 自更新防风暴：替换脚本失败即停，成功后只隐藏重启一次，避免失败循环反复拉起旧 exe。
4. 旧入口清理：安装/修复/关闭自启时清理旧 Run 项、旧计划任务、Startup 快捷方式。
5. CLI 隐藏执行：Route A 和 legacy relay 的 `.cmd/.bat` shim 统一包装，避免只隐藏外层却让子进程开控制台。
6. 回归验证：跑 PC 节点目标 clippy、相关单测、全量测试和格式检查；记录全量 clippy 的仓库历史缺口。
7. 任务恢复补强：服务重启或 PC 节点超时把 running 任务标记为 interrupted/failed 时，同步补写频道 `ai_result`，并避免活跃 runner 双终态。
8. 任务现场快照：为 PC 开发频道任务提供只读 `snapshot/events` API，返回持久 task、频道消息、事件游标和 live/detached/terminal attach 状态，作为后续真正 attach/journal 的服务端骨架。
9. Route C 兜底可靠性：Route A 不再只因发现 CLI 路径就抢占自动路由；新节点必须通过 CLI 版本探测才算 Route A ready，残留坏 CLI 会自动落到 Route B/C。
10. Route C 云端健康预检：服务器提供 `/api/agent/runtime/status`，PC 节点用登录 token 预检服务器模型是否真实可用，避免只因有 token 就显示 Route C ready。
11. PC 前端任务现场接入：AI 开发频道前端消费 `snapshot` 接口缓存 attach/事件游标，用轻量快照轮询替代纯整频道刷新，并在任务卡显示 live/detached/terminal 现场状态。
12. PC 节点本地 journal：节点本机写入 CLI prompt registry/jsonl，记录 started/cancel_requested/finished，作为后续重启恢复和 attach 协议的数据底座。
13. 本机 journal 查询闭环：7799 本地管理 API 暴露受 token 保护的 task journal 查询；云端 snapshot 返回 `pc_req_id`，PC 前端按该映射合并本机 live/detached/terminal 状态，区分“重连原进程”和“基于快照继续”。
14. 恢复契约显式化：7799 journal API 返回 `resume` 合同，明确 live 只能重连控制句柄、暂不回放 stdout/stderr；detached/terminal 只能基于快照新开任务继续，前端据此关闭无效停止/审批入口和无限轮询。
15. 本机事件回放：PC 节点把 Route A stdout/stderr 与 Route B/C 结构化工具事件写入本机 journal；PC 前端按 `pc_req_id + since` 把本机 journal 事件补进任务消息时间线，提供快照/轮询式事件回放。
16. 同进程运行句柄显式化：PC 节点把活跃 `CliPrompt` 从裸取消 sender 升级为 live run handle，暴露 route、PID、lease、pending approvals，让前端只在本机仍有真实 waiter 时显示审批按钮。
17. Codex 会话续接锚点：云端 PC Codex 分发下发稳定 `--session-id`，PC 节点把真实 Codex session 写入本机 journal/resume 契约，前端提示“Codex 会话可续接”。

## 风险

- GUI 黑窗是否可见最终依赖 Windows 桌面烟测，自动化主要验证启动条件、隐藏 flags 和防重复逻辑。
- 全量 `cargo clippy --all-targets --all-features -- -D warnings` 当前被服务端/测试历史 lint 阻塞，未在本次黑窗任务中大范围清理。
- 原主工作区存在未提交改动，本任务只在隔离 worktree 修改并只 stage 本任务文件。
- 任务终态可见恢复不是同进程续跑；Codex Desktop 级恢复仍需要持久 run handle、PC 节点 journal、attach 协议和审批 waiter 重绑定。
- 云端 `task_id` 与本机 PC 节点 `req_id` 不是同一个 ID；前端必须使用云端 snapshot 返回的 `pc_req_id` 查询本机 journal，不能把 `tsk_*` 当作本机 key。
- 当前 `resume` 合同和事件回放仍不是完整 TTY attach；输出来自本机 journal 快照/轮询，不是直接接管原 CLI 进程的 stdout/stderr 句柄，页面刷新后的审批 waiter 仍需后续持久化。
- live run handle 只证明当前节点进程内还持有控制面；节点进程重启后仍不能重新接回原 CLI TTY 或恢复内存态审批 waiter。
- Codex session 现在可由本机节点自动续接，但仍是“新一轮 prompt 进入同一 Codex 原生会话”，不是恢复原进程正在执行的 TTY。

## 验证命令

- `cargo fmt --manifest-path server\Cargo.toml --all --check`
- `cargo clippy --manifest-path server\Cargo.toml --bin elon-pc-node -- -D warnings`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_client_launcher`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_cli_security`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_tool_guard`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_server_runtime`
- `cargo test --manifest-path server\Cargo.toml --bin elon-server task_recovery_tests -- --nocapture`
- `cargo test --manifest-path server\Cargo.toml --bin elon-server store::tasks::tests -- --nocapture`
- `cargo test --manifest-path server\Cargo.toml project_tool_approval -- --nocapture`
- `cargo test --manifest-path server\Cargo.toml --bin elon-server pc_agent_runtime_choice -- --nocapture`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_route_c_status -- --nocapture`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_task_journal -- --nocapture`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_task_resume -- --nocapture`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_tool_approval -- --nocapture`
- `cargo test --manifest-path server\Cargo.toml --bin elon-server pc_cli_passthrough -- --nocapture`
- `cargo test --manifest-path server\Cargo.toml --bin elon-server project_space_task_snapshot -- --nocapture`
- `cargo test --manifest-path server\pc-dev-runtime\Cargo.toml profile -- --nocapture`
- `node scripts\test-pc-dev-assets.js`
- `cargo check --manifest-path server\Cargo.toml --bin elon-server --bin elon-pc-node`
- `cargo test --manifest-path server\Cargo.toml --all-features`
- `git diff --check`

## 回滚策略

- 回滚本次提交即可恢复旧启动器、CLI shim 和隐藏窗口行为。
- 如节点包已经发布，回滚后基于上一稳定 SHA 重新运行 `scripts\publish-node-agent.ps1`。
- 不 reset 原主工作区，不覆盖用户或其他 AI 的未提交改动。
