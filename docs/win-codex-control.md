---
version_status: current
reviewed_at: 2026-08-08
implementation_status: compiled
---

# Win Codex 语义控制与统一诊断

本文定义 Codex 操控一龙 Win 工作台和读取调试证据的公开合同。它扩展现有 `codex exec --json + pipe_sidecar + task journal`，不替代 Codex CLI，也不把任意桌面点击、任意 JavaScript、任意 Tauri command 或用户浏览器凭据暴露给代理。

## 用户结果

- Codex 通过短期、项目绑定的 MCP profile 查询 Win/Tauri 能力、读取当前页面和节点状态、提交白名单语义动作并读取执行回执。
- Win 工作台提供 `/codex-control` 控制台，以同一时间线查看 `frontend`、`rust`、`cli`、`network`、`tauri` 与 `control` 事件。
- 每个事件包含稳定 `event_id`、`trace_id`、来源、级别、类型、摘要和时间；不记录请求/响应正文、Cookie、Authorization、API key、prompt、CLI 原始秘密或 Tauri 参数正文。
- 每个动作包含稳定 `action_id`、来源、目标、状态和回执；没有 Win 壳在线执行时保持可见的 `queued/expired`，不能伪造成成功。

## 语义动作白名单

第一版只允许：

1. `show_window`、`focus_window`：呼出并聚焦主窗口。
2. `navigate`：只接受已登记的一龙 PC 相对路由，不接受完整 URL、协议、host、query secret 或任意脚本。
3. `reload_page`：刷新主工作台。
4. `open_devtools`、`close_devtools`：只由 Tauri 壳执行并写回回执；生产是否允许由壳能力明确报告。
5. `capture_state`：读取当前路由、标题、可见/聚焦状态和版本等非秘密状态，不截图、不读输入框正文。

动作由节点 loopback API 排队，Win 页面先原子领取为 `executing`，再调用 Tauri 白名单 command 并写回成功或失败回执。刷新等会中断页面的动作会延迟到回执发起后执行；领取后页面崩溃时动作只会过期，不会被新页面重复执行。Codex MCP 只调用同一领域服务，不直接操作进程、窗口句柄或 WebView2 profile。

## 统一时间线

节点维护有界内存事件环并按需合并 task journal：

- `frontend`：页面启动、路由、window error、unhandled rejection；摘要强制截断。
- `network`：method、同源路径、status、duration、失败分类；移除 query、header 和 body。
- `tauri`：命令名、动作结果、窗口/页面生命周期；不保存参数/结果正文。
- `rust`：节点/控制域健康、诊断文件摘要和控制 API 错误。
- `cli`：最近任务的状态、phase、脱敏 current command 和有界 journal 事件。
- `control`：动作排队、领取、回执、过期与 MCP 调用。

默认最多保留 2,000 条控制域事件，查询最多返回 500 条。导出只生成脱敏 JSON；原始 CLI 输出仍由既有 task journal 的授权详情页读取，不复制成第二份日志。

## 权限与失败关闭

- HTTP 写操作只在 `127.0.0.1` 上提供，并复用 `x-elon-local-admin-token` 与可信 Origin 校验。
- MCP descriptor 使用随机短期 token、项目根绑定和 profile 固定，不能在同一会话切换权限。
- 页面领取动作后必须核对 `action_id`，重复回执幂等；过期动作不再执行。
- 事件字段按 key 和内容双重脱敏；Cookie、token、password、secret、authorization、API key 一律替换。
- 没有 Tauri 宿主时，浏览器/PWA 控制台仍能读取日志，但 Tauri 动作返回明确 `host_unavailable`。
- 此能力不恢复已暂停的跨项目自动派发、自动验收、自动续跑或后台自进化。

## 验收

- Rust 单元测试覆盖动作/路由白名单、事件脱敏、分页、回执幂等和 MCP profile 隔离。
- Tauri 测试覆盖相对路由校验、事件有界化和白名单动作。
- PC 静态测试覆盖路由、全局桥、网络无正文日志、Tauri 回执和控制台五类来源筛选。
- 编译通过只证明合同和类型闭合；真实 Codex App Server、MCP 客户端、WebView2、DevTools、长时间事件量和发布安装节点仍需分别记录现场证据，不能互相冒充。
