---
version_status: current
reviewed_at: 2026-09-23
---

# Win WebView AI 接管四项能力 · 2026-09-23

本文只记录实现与验证证据；运行合同以 [Win 浏览器研究运行说明](../win-browser-research-mcp.md) 为准。

## 交付内容

| 能力 | 实现 | 自动验证 | 现场验收 |
|---|---|---|---|
| 研究宿主挂载交易所登录窗口（`host_mode=exchange_window`） | `browser_research::host::attach`、`ResearchRuntime.execute(attach_label)`、`exchange_webview::ensure_exchange_session_for_site` | desktop 45→218 项 | 待真实币安测试账户 |
| Win 交易所窗口注入共享 Binance 只读适配器 | `ProviderAdapter::Binance`、`binance_win_bridge.js`、`ExchangeObservationRuntime` 与两条只读命令；`DESKTOP_RUNTIME_VERSION` 14 | desktop 215 项、`test-exchange-webview.cjs`、`test-local-ai-browser-contract.cjs` | 待现场 |
| 研究 MCP `export` 会话快照 | `browser_research::export`，写 `<会话>/exports/snapshot-<ms>.json` | desktop 218 项、harness 20 项、前端 24 项 | 待现场 |
| 研究 MCP 开发门控 `evaluate` | `browser_research::dev_eval` + 固定 CDP `Runtime.evaluate`，仅 `ELON_BROWSER_RESEARCH_DEV_EVAL=1` 或 debug 构建 | 同上 | 待现场 |

VS Code 侧已把 `project-memory-mcp-proxy.mjs browser_research` 登记为用户级 MCP server，`initialize` 与 `hosts` 在本机节点 7799 与桌面宿主实例上通过。

## 提交

`3101e43f3`（attach）、`6d8fd1e9d`（Binance 观察器）、`07f132a2d`（export/evaluate）。

## 与本批无关的既有阻塞

- `scripts/validate-win-web-ai.ps1` 的 `test-chatgpt-win-attachment-transport.js` 仍钉在 ChatGPT adapter 207，源码自 `fec86533f` 起为 208。
- `pc-frontend/scripts/test-local-ai-browser-contract.cjs` 与上游 `fcded4224` 的 `chatgpt_web_adapter.js` 改动不匹配（`authenticationPolicy.isAuthenticated` 断言）。
- 节点 `elon-pc-node` test profile 存在 `compute_plugin_host` 测试模块私有再导出编译错误；bin check 通过。
