# PC 节点黑窗风暴修复进度

## 当前状态

- 工作目录：`D:\一龙\一龙参考库-task-20260621-205803`
- 分支：`codex/task-20260621-205803`
- 基线：已 rebase 到 `origin/main` 的 `13e06854`
- 目标：修复 PC 端 exe 启动/更新/协议唤起/CLI 执行时几十到上百个黑色 cmd 窗口的问题。
- 追加阶段工作目录：`D:\rust\active-projects\elon-win-task-recovery-20260621`
- 追加阶段目标：补齐服务重启/PC 节点超时后的频道 AI 任务终态可见恢复，避免 PC 任务卡永久停在运行中。

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
- 通过：`node scripts\test-pc-dev-assets.js`
- 通过：`cargo check --manifest-path server\Cargo.toml --bin elon-server --bin elon-pc-node`
- 通过：`cargo clippy --manifest-path server\Cargo.toml --bin elon-pc-node -- -D warnings`
- 通过：`cargo test --manifest-path server\Cargo.toml --all-features -- --test-threads=1`，`elon-pc-node` 100 passed，`elon-server` 518 passed
- 通过：`git diff --check`，仅有 Git 的 CRLF 工作区提示
- 未通过：`cargo clippy --manifest-path server\Cargo.toml --bin elon-server -- -D warnings`，退出 101；剩余为服务端历史 lint（例如 `billing_pay.rs`、`agent.rs`、`store.rs`、`tools.rs`、`project_ws_job.rs` 等），不属于本次恢复补丁。
- 未通过后复核：`cargo test --manifest-path server\Cargo.toml --all-features` 默认并发模式曾出现 `tools_patch::tests::apply_patch_changes_file` 偶发失败；该用例单独重跑通过，单线程全量测试通过。

## 剩余任务

- 只 stage 本任务文件，commit、push。
- 按发布脚本发布服务端；本追加阶段未改 Windows 节点包和 PC 静态资源，原则上不需要重新发布节点包。
- 下一阶段实现真正任务恢复：定义持久 run handle，绑定 `task_id/pc_req_id/node_id/route/cwd/codex_session_id/lease/last_event_seq/resume_strategy`。
- 下一阶段给 PC 节点增加本地任务 registry/jsonl journal，并扩展 homecli attach/snapshot/since 协议，避免把 `codex resume` 误认为同进程恢复。

## 剩余风险

- 自动化无法直接证明 GUI 黑窗肉眼不可见，仍建议发布后在干净 Windows 用户环境做一次双击、协议唤起、自更新和开机自启烟测。
- 全量 clippy 历史债未在本任务内清理，避免把本次用户可见故障修复扩大成仓库治理。
- 任务终态可见恢复只是让 UI 收到明确终态和“继续”提示，不是接回同一个 Codex/CLI 进程继续执行。
