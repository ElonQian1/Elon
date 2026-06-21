# PC 节点黑窗风暴修复进度

## 当前状态

- 工作目录：`D:\一龙\一龙参考库-task-20260621-205803`
- 分支：`codex/task-20260621-205803`
- 基线：已 rebase 到 `origin/main` 的 `13e06854`
- 目标：修复 PC 端 exe 启动/更新/协议唤起/CLI 执行时几十到上百个黑色 cmd 窗口的问题。

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

## 验证结果

- 通过：`cargo fmt --manifest-path server\Cargo.toml --all --check`
- 通过：`cargo clippy --manifest-path server\Cargo.toml --bin elon-pc-node -- -D warnings`
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_client_launcher`，11 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_cli_security`，7 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_tool_guard`，16 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_server_runtime`，5 passed
- 通过：`cargo test --manifest-path server\Cargo.toml --all-features`，503 passed
- 通过：`git diff --check`，仅有 Git 的 CRLF 工作区提示
- 未通过：`cargo clippy --manifest-path server\Cargo.toml --all-targets --all-features -- -D warnings`，退出 101；剩余为服务端/测试历史 lint（例如 `billing_pay.rs`、`store.rs`、`tools.rs`、`project_membership.rs`、`project_mobile.rs` 等），不属于本次黑窗链路。

## 剩余任务

- 只 stage 本任务文件，commit、push。
- 按发布脚本尝试发布 PC 节点包；如脚本/权限阻塞，记录阻塞原因。

## 剩余风险

- 自动化无法直接证明 GUI 黑窗肉眼不可见，仍建议发布后在干净 Windows 用户环境做一次双击、协议唤起、自更新和开机自启烟测。
- 全量 clippy 历史债未在本任务内清理，避免把本次用户可见故障修复扩大成仓库治理。
