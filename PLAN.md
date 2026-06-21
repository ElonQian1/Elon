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

## 风险

- GUI 黑窗是否可见最终依赖 Windows 桌面烟测，自动化主要验证启动条件、隐藏 flags 和防重复逻辑。
- 全量 `cargo clippy --all-targets --all-features -- -D warnings` 当前被服务端/测试历史 lint 阻塞，未在本次黑窗任务中大范围清理。
- 原主工作区存在未提交改动，本任务只在隔离 worktree 修改并只 stage 本任务文件。

## 验证命令

- `cargo fmt --manifest-path server\Cargo.toml --all --check`
- `cargo clippy --manifest-path server\Cargo.toml --bin elon-pc-node -- -D warnings`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_client_launcher`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_cli_security`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_tool_guard`
- `cargo test --manifest-path server\Cargo.toml --bin elon-pc-node node_agent_server_runtime`
- `cargo test --manifest-path server\Cargo.toml --all-features`
- `git diff --check`

## 回滚策略

- 回滚本次提交即可恢复旧启动器、CLI shim 和隐藏窗口行为。
- 如节点包已经发布，回滚后基于上一稳定 SHA 重新运行 `scripts\publish-node-agent.ps1`。
- 不 reset 原主工作区，不覆盖用户或其他 AI 的未提交改动。
