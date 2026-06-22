# PC 节点黑窗风暴修复进度

## 当前状态

- 工作目录：`D:\一龙\一龙参考库-task-20260621-205803`
- 分支：`codex/task-20260621-205803`
- 基线：已 rebase 到 `origin/main` 的 `13e06854`
- 目标：修复 PC 端 exe 启动/更新/协议唤起/CLI 执行时几十到上百个黑色 cmd 窗口的问题。
- 追加阶段工作目录：`D:\rust\active-projects\elon-win-task-recovery-20260621`
- 追加阶段目标：补齐服务重启/PC 节点超时后的频道 AI 任务终态可见恢复，并继续补强 PC 节点本机 journal 查询，让前端能区分“重连原进程”和“基于快照继续”。

## 已完成

- 已读取项目入口、Codex/Git/模块化说明、README/Cargo 入口、主要 PC 节点源码和相关测试目录。
- 已启动 Explorer / Implementer / Reviewer / Tester 子代理并整合结论。
- 已新增 `node_client_launcher::command`，统一 Windows 隐藏启动、PowerShell/cmd 参数、可捕获输出命令和 URL 打开。
- 已把管理页打开从 `cmd /C start` 改为隐藏 `explorer.exe`。
- 已让安装后启动直接隐藏启动目标 exe，不再经 PowerShell 二次 `Start-Process`。
- 已在 admin 未健康时检查当前安装目录已有的 `--agent-runtime` 进程，避免协议/双击反复 spawn。
- 已把健康检查改为 `/api/status` 并要求响应包含 `local_admin_token_header`，避免随机端口 200 被误判。
- 已给自更新脚本加入 `$ErrorActionPreference = 'Stop'`，替换失败不再继续重启旧客户端。
- 已清理旧 Run 项、旧计划任务和 Startup 快捷方式，降低历史版本残留重复启动源。
- 已统一 Route A / legacy relay 的 `.cmd/.bat` shim 包装，并给相关 tokio/本机命令加隐藏窗口 + 新进程组 flags。
- 已为本次触发的 PC 节点 clippy 问题做小范围结构调整，不降低 lint 强度。
- 已修复最终 reviewer 发现的 `output_hidden` 未捕获 stdout 问题，并新增测试覆盖。
- 已确认新增 `server/src/node_client_launcher/command.rs` 是必要源码文件，提交时必须 stage。
- 已新增任务终态可见恢复：服务启动把 running 任务标记为 interrupted、超时清理把 running 任务标记为 failed 时，会为有 `ai_task` 但没有 `ai_result` 的 AI 开发频道任务补写一条终态 `ai_result`。
- Reviewer 发现超时清理可能误伤仍在本进程内运行的长任务；已修复为 stale cleanup 排除 `CHANNEL_AI_TASKS` 里的活跃任务 ID，避免 runner 后续再写 done/error 导致双终态。
- 已把频道 AI 任务的 done/error/cancel 结果写入统一改为 `insert_project_channel_ai_result_once`，正常完成、取消、恢复补写都遵守同一任务只写一条 `ai_result` 的规则。
- 已新增 `server/src/store/task_recovery_tests.rs`，覆盖 interrupted/stale 状态和 error、重复清理幂等、已有 `ai_result` 不重复、未超时任务不清理、活跃任务排除、通用 once helper。
- 已新增 `finish_running_task`，频道 AI runner 结束时只允许从 `running` 状态写入最终结果，避免恢复逻辑已写入 `interrupted/failed` 后又被迟到 runner 覆盖。
- 已新增频道消息任务状态投影：`ProjectChannelMessage` 返回 `task_status`、`task_error`、`task_apk_url`，PC 任务卡优先使用后端任务状态判断是否仍在运行，终态任务的历史审批按钮会失效并提供继续入口。
- 已新增 PC 开发任务现场快照 API：`/ai-tasks/:task_id/snapshot` 和 `/ai-tasks/:task_id/events` 返回持久任务信息、频道消息、事件 `rowid` 游标、`has_more` 以及 `live/detached/terminal` attach 状态，作为后续 PC node journal/attach 的服务端骨架。
- 已修复 Route A 假 ready 抢占 Route C 的一类问题：PC 节点 profile 现在要求 CLI 版本探测成功才标记 Route A ready；服务端自动/强制 Route A 会尊重该状态，坏的本机 CLI 会让自动路线继续落到 Route B/C。
- 已新增 Route C 云端健康预检：服务端暴露 `/api/agent/runtime/status`，PC 节点启动时用登录 token 验证服务器模型 runtime 是否真实 ready，避免“有 token 但服务器模型未配置”时误报可用。
- 已把 PC 任务 `snapshot` 接入前端并补齐 `/assets` 路由：AI 开发频道现在会按任务快照游标轮询，缓存 attach 状态，并在任务卡展示“现场可连接 / 现场已脱离 / 终态快照”。
- 已新增 PC 节点本地任务 journal 基础：节点本机写入 CLI prompt `registry.json` 与 `events.jsonl`，记录 started / cancel_requested / finished，给后续重启恢复和 attach 协议使用。
- 本轮已新增 PC 节点本地 journal 查询闭环：`/api/task-journal`、`/api/task-journal/:pc_req_id` 挂在 7799 local-admin token 保护链路下，返回 record、events、游标和 `live/detached/terminal/missing` attach 状态。
- 本轮已修正云端 `task_id` 与本机 `pc_req_id` 的映射：PC CLI dispatch 开始时写入非敏感 `pc_dispatch_started` 事件，云端任务 snapshot/events 响应返回 `pc_req_id`，前端只用该 ID 查询本机 journal，不再误把 `tsk_*` 当成本机 key。
- 本轮已让 PC AI 开发任务卡合并本机 journal 状态：本机仍持有 active handle 时显示“本机现场可连接”，本机 journal 残留 running 但无 handle 时显示“本机现场已脱离”，终态时显示“本机终态快照”。
- 本轮已新增恢复契约：本机 journal API 返回 `resume`，明确 live 只能重连控制句柄且暂不回放 stdout/stderr；detached/terminal/missing 不能重连原进程，只能基于快照或云端快照继续。
- 本轮已让 PC AI 开发任务卡消费 `resume`：live 卡提示本机 journal 能回放事件，detached 卡改为“需要基于快照继续”，关闭无效停止/审批按钮，并从开放任务轮询列表移除，避免无限轮询已经丢失句柄的任务。
- 本轮已新增本机事件回放：Route A stdout/stderr、Route B/C 工具事件和运行时进度会写入本机 journal；PC 前端把 local journal 的 `cli_chunk/tool_event` 转成 `ai_progress` 补进任务消息，任务卡提示“本机事件可回放”。
- 本轮已新增同进程 live run handle：PC 节点把活跃 `CliPrompt` 从裸 `watch::Sender` 升级为 `ActiveCliPromptHandle`，本机 journal API 可返回 route、run_handle_id、PID、lease 和当前 pending approvals。
- 本轮已把工具审批按钮绑定到真实本机 waiter：Route B/C 工具审批注册后可被本机 resume 契约列出，前端只有在 `can_approve_tools=true` 且 `approval_id` 仍处于 active 列表中时才显示批准/拒绝按钮，避免历史审批卡误导用户。
- 本轮全量测试暴露并修复了一个非 PC 节点的历史门禁问题：`mark_project_suggestion_updated` 复用统一频道消息 row mapper，但 SELECT 少返回 `task_status/task_error/task_apk_url` 三列，导致建议消息标记更新测试报 `Invalid column index: 15`；已补齐列和 `LEFT JOIN tasks`。
- 本轮已补齐 PC Codex 会话续接锚点：云端 PC Codex 分发现在和 Copilot 一样下发稳定 `--session-id`，PC 节点按 `session-id + 权限 + cwd` 保存真实 Codex session，并优先从 task journal 读取后执行 `codex exec resume <session>`。
- 本轮已把 Codex session 元数据写入本机 journal 与 resume 契约：`registry.json` 保存 `codex_session_id/scope_key/updated_at_ms`，`codex-sessions.json` 保存 scope 到真实 session 的映射，前端任务卡显示“Codex 会话可续接”。
- 本轮已新增 Codex stale resume 自愈：如果本机 `codex exec resume <session>` 返回 session/thread/resume not found、invalid、expired 等失效信号，PC 节点会清理 task journal 与旧版 `%TEMP%\elon_codex_sessions.json` 映射，并用同一 `req_id` 自动 fresh retry。
- 本轮已把 Codex session 读取、失效判断和清理旧缓存抽到 `node_agent_codex_session.rs`，避免继续把 Route A session 逻辑堆进 3000 行以上的 `node_agent_main.rs`。

## 验证结果

- 通过：`cargo fmt --manifest-path server\Cargo.toml --all --check`
- 通过：`cargo clippy --manifest-path server\Cargo.toml --bin elon-pc-node -- -D warnings`
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_client_launcher`，11 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_cli_security`，7 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_tool_guard`，16 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_server_runtime`，5 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --all-features`，503 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-server task_recovery_tests -- --nocapture`，7 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-server store::tasks::tests -- --nocapture`，11 passed（新增频道消息任务状态投影）
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-server store::tasks::tests -- --nocapture`，13 passed（新增事件游标与频道任务快照绑定）
- 通过：`cargo test --manifest-path server\Cargo.toml project_tool_approval -- --nocapture`，8 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-server pc_agent_runtime_choice -- --nocapture`，9 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_route_c_status -- --nocapture`，1 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_task_resume -- --nocapture`，3 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_task_journal -- --nocapture`，6 passed（新增本机输出、工具事件回放和 Route A pid 记录）
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_task_resume -- --nocapture`，4 passed（新增 Codex session resume 契约）
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_task_journal -- --nocapture`，7 passed（新增 Codex session 持久化）
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_task_journal -- --nocapture`，8 passed（新增 stale Codex session 清理）
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_codex_session -- --nocapture`，3 passed（新增 stale resume 检测、journal 优先读取和旧缓存清理）
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_tool_approval -- --nocapture`，4 passed（新增 pending waiter 查询）
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_server_runtime -- --nocapture`，5 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-server pc_cli_passthrough -- --nocapture`，4 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-server pc_cli_passthrough -- --nocapture`，7 passed（新增 Codex/Copilot Route A session 参数测试）
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-server project_space_task_snapshot -- --nocapture`，2 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-server store::project_space::tests::suggestion_message_can_be_marked_updated -- --nocapture`，1 passed
- 通过：`cargo test --manifest-path server\pc-dev-runtime\Cargo.toml profile -- --nocapture`，2 passed
- 通过：`node scripts\test-pc-dev-assets.js`
- 通过：`cargo check --manifest-path server\Cargo.toml --bin elon-server --bin elon-pc-node`
- 通过：`cargo clippy --manifest-path server\Cargo.toml --bin elon-pc-node -- -D warnings`
- 通过：`cargo test --manifest-path server\Cargo.toml --all-features -- --test-threads=1`，`elon-pc-node` 117 passed，`elon-server` 534 passed
- 通过：`git diff --check`，仅有 Git 的 CRLF 工作区提示
- 通过：`cargo fmt --manifest-path server\Cargo.toml --all --check`
- 未通过：`cargo clippy --manifest-path server\Cargo.toml --all-targets --all-features -- -D warnings`，退出 101；剩余为服务端历史 lint（例如 `billing_pay.rs`、`agent.rs`、`store.rs`、`tools.rs`、`project_membership.rs`、`project_mobile.rs` 等），不属于本次恢复补丁。
- 未通过后复核：`cargo test --manifest-path server\Cargo.toml --all-features` 默认并发模式曾出现 `tools_patch::tests::apply_patch_changes_file` 偶发失败；该用例单独重跑通过，单线程全量测试通过。
- 未通过后复核：本轮第一次 `cargo test --manifest-path server\Cargo.toml --all-features -- --test-threads=1` 在 `elon-server` 进程阶段异常退出；停靠点附近的 `store::node_ledger::tests::unbilled_usage_does_not_increase_provider_balance` 单独重跑通过，随后全量单线程重跑通过。

## 剩余任务

- 只 stage 本任务文件，commit、push。
- 按发布脚本发布服务端；本阶段改动了 Windows 节点启动侧 Route C ready 判断，需要同步重新发布 Windows 节点包。
- 下一阶段实现真正任务恢复：定义持久 run handle，绑定 `task_id/pc_req_id/node_id/route/cwd/codex_session_id/lease/last_event_seq/resume_strategy`。
- 下一阶段扩展 homecli attach 协议，把 `pc_req_id/node_id/pid-or-handle/lease` 纳入可订阅控制面；当前能查询同进程 run handle 和 pending approvals，但尚不能真正把浏览器接入原 CLI TTY。
- 下一阶段持久化更完整的运行出口状态（exit code、finished reason、Codex session/thread 元数据），并评估 ConPTY/管道代理，避免节点重启后只能基于快照继续。

## 剩余风险

- 自动化无法直接证明 GUI 黑窗肉眼不可见，仍建议发布后在干净 Windows 用户环境做一次双击、协议唤起、自更新和开机自启烟测。
- 全量 clippy 历史债未在本任务内清理，避免把本次用户可见故障修复扩大成仓库治理。
- 任务终态可见恢复只是让 UI 收到明确终态和“继续”提示，不是接回同一个 Codex/CLI 进程继续执行。
- 本机事件回放已经保存 stdout/stderr 与 B/C 工具事件；同一节点进程内可查询 live handle、Route A pid、pending approval waiter 和 Codex session 元数据，但仍不是完整 TTY attach，节点重启后的进程控制和跨节点恢复仍是缺口。
