---
version_status: current
reviewed_at: 2026-08-16
implementation_status: tested
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
2. `navigate`：只接受已登记的一龙 PC 相对路由，不接受完整 URL、协议、host、query secret 或任意脚本；Tauri 执行时固定补齐 `/pc` 浏览器 basename，回执成功后必须仍由 PC Router 接管，不能落到站点根的空白页。
3. `reload_page`：刷新主工作台。
4. `open_devtools`、`close_devtools`：只由 Tauri 壳执行并写回回执；生产是否允许由壳能力明确报告。
5. `capture_state`：读取当前路由、标题、可见/聚焦状态和版本等非秘密状态，不截图、不读输入框正文。
6. `list_ai_windows`：列出 `chatgpt` 与 `google-ai-mode` 两个固定厂商的生产官方网页窗口状态；每项同时包含同一生产会话的 `official_session` 结构诊断，不返回 Tauri label、owner 指纹、窗口句柄或 URL。
7. `capture_ai_window_state`：按固定 `provider_id` 读取生产官方窗口阶段、是否打开/聚焦、语义快照健康，以及适配器、上下文、缓存、目录和流式就绪度；只公开稳定错误码、固定枚举、布尔值与有界计数。
8. `focus_ai_window`：按固定 `provider_id` 恢复、显示并聚焦已存在的生产官方网页窗口；找不到窗口时失败关闭，不隐式创建网页会话。
9. `update_and_restart`：只接受 Codex MCP 提交的精确 `version+40–64 位 Git SHA` 发布身份。Tauri 仅在正式安装目录中启动外部更新守护器，先写回“已安排”回执，再主动退出桌面壳；守护器复用既有签名、健康检查、任务终态等待和回滚流程，确认更新锁释放后自动重开正式工作台。HTTP/PC UI、模糊版本、任意 URL、任意程序路径和任意进程号均被拒绝。

动作由节点 loopback API 排队，Win 页面先原子领取为 `executing`，再调用 Tauri 白名单 command 并写回成功或失败回执。刷新等会中断页面的动作会延迟到回执发起后执行；领取后页面崩溃时动作只会过期，不会被新页面重复执行。Codex MCP 只调用同一领域服务，不直接操作进程、窗口句柄或 WebView2 profile。

动作提交成功只代表进入队列。Codex 必须调用 MCP `win_control_action_status` 或 HTTP
`GET /api/codex-control/actions/:action_id`，一直查询到 `succeeded`、`failed`、
`host_unavailable`、`rejected` 或 `expired`。AI 窗口回执的 `window_state` 在节点再次按固定
schema 清洗并限制为 16 KiB；即使被篡改的 Tauri 页面提交 label、URL、Cookie 或 token，
节点也不会把这些字段返回给 Codex。

独立测试聊天窗已经退役。AI 窗口状态中的 `phase` 直接投影生产 `/pc/ai` 使用的官方
WebView2 生命周期；`phase=not_created` 表示该厂商生产会话尚未创建，`closed` 表示曾有运行时
记录但窗口当前已关闭。同一 provider 项下的 `official_session` 返回更完整的结构诊断，仅包含窗口与加载
状态、适配器/语义快照/输入框/上下文就绪度、导航与目录完整度、缓存状态、消息/会话/项目/
置顶有界计数、流式状态、稳定动作名和稳定错误码。Tauri 从运行时生成一次安全投影，节点再按
固定白名单重建一次；消息正文、标题、草稿、引用、会话 ID、项目名、URL、host、owner、账号、
Cookie、token、Authorization 和网页异常详情不会进入控制响应。

`update_and_restart` 的 `succeeded` 回执只证明退出与更新守护器已经成功安排，不证明新版本已激活。Codex 必须等待节点重新连接，并从 `win_control_capabilities.release_identity` 回读与请求完全相同的发布身份，才能宣告更新完成；若正式发布正在等待不可安全中断的本机任务，守护器会持续等待，而不是绕过终态门禁。

## 统一时间线

节点维护有界内存事件环并按需合并 task journal：

- `frontend`：页面启动、路由、window error、unhandled rejection；摘要强制截断。
- `network`：method、同源路径、status、duration、失败分类；移除 query、header 和 body。
- `tauri`：命令名、动作结果、窗口/页面生命周期；不保存参数/结果正文。
- `rust`：节点/控制域健康、诊断文件摘要和控制 API 错误。
- `cli`：最近任务的状态、phase、脱敏 current command 和有界 journal 事件。
- `control`：动作排队、领取、回执、过期与 MCP 调用。

默认最多保留 2,000 条控制域事件，查询最多返回 500 条。导出只生成脱敏 JSON；原始 CLI 输出仍由既有 task journal 的授权详情页读取，不复制成第二份日志。

Tauri 原生桥还在后台把最近 64 条原生事件写入本机 `desktop-diagnostics-v1` 快照。持久化
时最多保留 4 条高频心跳，窗口创建、焦点变化、关闭请求与销毁等低频生命周期事件不会再被
心跳挤出快照。每次桥事件（包括能力心跳和语义动作回执）都在写入内存环后自动调度同一份
脱敏快照，不能只留在当前桌面进程内存中。节点读取时
限制为 512 KiB、校验 schema 并最多返回 32 条；HTTP 响应和 MCP 不返回快照文件路径。
因此即使主工作台页面暂时无法回传日志，Codex 仍可从节点读取生产官方窗口的创建、
导航、错误码、语义快照健康、焦点和销毁状态。快照遵守同一正文与凭证禁采集边界。
能力心跳还会附带两个固定厂商窗口的安全结构投影；因此不必先执行聚焦动作，也能在后台
判断最后一次运行时究竟停在加载、会话绑定、私有流、富内容计数或官网回退中的哪一步。
投影同时保留富内容恢复结果、实时语音通道计数和附件上传完成态，仍只含固定枚举、布尔值
和有界计数，不包含消息正文、转写正文、文件名、会话 ID、URL、账号或请求材料。

## 权限与失败关闭

- HTTP 写操作只在 `127.0.0.1` 上提供，并复用 `x-elon-local-admin-token` 与可信 Origin 校验。
- MCP descriptor 使用随机短期 token、项目根绑定和 profile 固定，不能在同一会话切换权限。
- 页面领取动作后必须核对 `action_id`，重复回执幂等；过期动作不再执行。
- 事件字段按 key 和内容双重脱敏；Cookie、token、password、secret、authorization、API key 一律替换。
- AI 窗口控制只接受 `chatgpt`、`google-ai-mode` 两个逻辑 provider；不接受窗口 label、任意厂商字符串、任意 URL 或任意 JavaScript。
- AI 窗口回执只公开逻辑 provider、生产官方窗口阶段、`official_session` 结构状态、焦点、语义快照健康、稳定错误码和更新时间；诊断只含布尔值、有界计数、固定枚举和稳定动作/错误码。窗口 label、owner 指纹、账号、会话身份、URL、页面正文、Cookie 与 token 均为明确的 `false` 采集能力。
- 更新重启动作只接受 `requested_by=codex_mcp` 与精确发布身份；能力投影明确报告 `arbitrary_update_target=false`、`update_restart_requires_exact_release=true`。调用方不能指定下载地址、脚本、安装目录、PID 或重开程序。
- 没有 Tauri 宿主时，浏览器/PWA 控制台仍能读取日志，但 Tauri 动作返回明确 `host_unavailable`。
- 此能力不恢复已暂停的跨项目自动派发、自动验收、自动续跑或后台自进化。

## 验收

- Rust 单元测试覆盖动作/路由/provider 白名单、更新来源与精确发布身份、更新清单身份匹配、生产 `official_session` 安全投影、AI 窗口回执二次清洗、恶意正文/凭证字段丢弃、事件脱敏、分页、回执幂等、精确 action 查询和 MCP profile 隔离。
- Tauri 测试覆盖相对路由校验、事件有界化、白名单动作、正式安装路径限制和守护器的安全退出/自动重开标记。
- PC 静态测试覆盖路由、全局桥、网络无正文日志、Tauri 回执和控制台五类来源筛选。
- 编译通过只证明合同和类型闭合；真实 Codex App Server、MCP 客户端、WebView2、DevTools、长时间事件量和发布安装节点仍需分别记录现场证据，不能互相冒充。
