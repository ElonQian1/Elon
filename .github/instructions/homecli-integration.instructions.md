---
applyTo: "server/src/homecli_agent.rs,server/src/router.rs,server/src/types.rs,server/src/agent.rs,server/src/tools.rs"
---

# elon-cli ↔ homecli 反向通道集成规则

> AI 代理在云端修改 shell/build 执行链路时必读。
> 共享协议已 vendored 到 `server/homecli-proto/`（crate 名 `homecli-proto`）。

## 角色与边界

- **云端不直接 spawn 用户机器的命令**：所有面向"开发者 PC"的 shell/build/test 必须经过 `AppState::agent_manager.dispatch(...)`，而不是 `std::process::Command` 或 `tokio::process::Command`。
- **AI 工具循环**（`server/src/agent.rs` 的 `execute_tool`）当前仍走 `tools::run_shell`（本机白名单）。把"`run_shell` 改为通过 `agent_manager` 下发"是 Phase 2 的工作，**Phase 1 不要改**。
- **白名单仍在云端**：即使 PC 端 agent 也做了二次校验，云端入口（`tools::run_shell` 白名单）不能被绕过。

## 协议契约

- 改 wire format ⇒ 改 `server/homecli-proto/src/lib.rs` ⇒ 同步 `PROTO_VERSION`，并同步 PC 端 agent。
- 禁止在 `homecli_agent.rs` 中手写 `serde_json::json!` 构造 `ServerToAgent`/`AgentToServer`。
- WS 帧只用 `Message::Text(serde_json::to_string(...))`。二进制走 base64 字符串。

## 鉴权

- 端点 `/agent/ws` 校验 `Authorization: Bearer <secret>`，secret 通过环境变量 `ELON_AGENT_SECRETS=id1:secret1,id2:secret2` 配置。
- 禁止把 secret 写进 `config.toml`、`tauri.conf.json`、commit message。
- `/api/_test_dispatch` 必须要求 admin token；这是 Phase 1 临时调试端点，Phase 2 应改为内部 AI 调用链路并移除。

## 部署提醒

- `server/Cargo.toml` 依赖仓库内 `server/homecli-proto`，临时 worktree 和服务器部署都必须带上这个目录。
- 发布服务端前必须先在干净提交上跑 `cargo check`，确认 vendored 协议 crate 能被解析。

## 与 bb64a 的隔离

- 这是 elon cli 仓库；不要往 homecli/bb64a 私有逻辑里加东西。
