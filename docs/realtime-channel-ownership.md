# Realtime Channel Ownership

本文档固定实时连接指标的 owner、源码边界和 close reason 来源。它和 `docs/realtime-operations-runbook.md` 的分工是：本文档回答“改哪里、谁负责、语义从哪里来”，runbook 回答“线上告警后怎么查、怎么止血”。

## Ownership Table

| Channel | 业务边界 | 入口/主模块 | Close reason 来源 | 指标写入点 | 改动时必须同步 |
| --- | --- | --- | --- | --- | --- |
| `app_notify` | 旧版移动端通知、升级提示 | `server/src/app_update.rs` | `WsCloseReason` in `server/src/ws_transport.rs` | `RealtimeChannel::AppNotify` | runbook 覆盖范围、后台 Realtime smoke 如影响展示、`check-realtime-ownership.ps1` |
| `global_app` | `/ws/app` 用户在线、好友状态、通用推送 | `server/src/global_ws.rs` | `WsCloseReason` in `server/src/ws_transport.rs` | `RealtimeChannel::GlobalApp` | 在线状态语义、好友/推送回归、runbook |
| `project_ws` | 项目任务状态、审批、执行流 | `server/src/project_ws_session.rs` | `WsCloseReason` in `server/src/ws_transport.rs` | `RealtimeChannel::ProjectWs` | 项目任务流测试、慢消费者/审批推送排查说明、runbook |
| `voice_transcribe` | 实时语音转写 | `server/src/voice_ws_transcribe.rs`、`server/src/voice_ws_transcribe_impl.rs` | `WsCloseReason` in `server/src/ws_transport.rs` | `RealtimeChannel::VoiceTranscribe` | 语音停止顺序、客户端控制关闭语义、runbook |
| `voice_realtime_chat` | AI 实时语音聊天 | `server/src/voice_ws_realtime_chat.rs`、`server/src/voice_ws_realtime_chat_impl.rs` | `WsCloseReason` in `server/src/ws_transport.rs` | `RealtimeChannel::VoiceRealtimeChat` | 语音计费/预授权路径、客户端控制关闭语义、runbook |
| `voice_virtual_mic` | 虚拟麦克风输入链路 | `server/src/voice_ws_virtual_mic.rs` | `WsCloseReason` in `server/src/ws_transport.rs` | `RealtimeChannel::VoiceVirtualMic` | 音频发送节奏、客户端停止录音语义、runbook |
| `homecli_agent` | HomeCLI/PC agent 反向长连接、PC CLI 调度 | `server/src/homecli_agent/agent_session.rs` | `AgentSessionCloseReason` in `server/src/homecli_agent/agent_session.rs` | `RealtimeChannel::HomecliAgent` | pending CLI 失败文案、节点假在线处置、告警阈值分类、runbook |
| `peer_relay` | 手机 P2P APK relay 和种子节点传输 | `server/src/peer_relay.rs` | `PeerWsCloseReason` in `server/src/peer_relay.rs` | `RealtimeChannel::PeerRelay` | registry 清理、APK 传输失败文案、P2P relay 处置说明、runbook |

## Close Reason Sources

### Shared Axum WS

`WsCloseReason` 由 `server/src/ws_transport.rs` 维护，当前覆盖：

- `peer_closed`
- `read_error`
- `reader_ended`
- `pong_write_failed`
- `write_failed`
- `client_control_close`

适用 channel：`app_notify`、`global_app`、`project_ws`、`voice_transcribe`、`voice_realtime_chat`、`voice_virtual_mic`。

### HomeCLI Agent

`AgentSessionCloseReason` 由 `server/src/homecli_agent/agent_session.rs` 维护，当前覆盖：

- `reader_shutdown`
- `reader_closed`
- `reader_timeout`
- `reader_error`
- `writer_closed`

适用 channel：`homecli_agent`。这些 reason 同时影响 pending CLI 请求的用户可见失败文案，不能只改指标标签。

### Peer Relay

`PeerWsCloseReason` 由 `server/src/peer_relay.rs` 维护，当前覆盖：

- `request_channel_closed`
- `peer_closed`
- `peer_read_error`
- `peer_reader_ended`
- `peer_write_error`

适用 channel：`peer_relay`。这些 reason 同时影响 APK relay 传输失败文案和 registry 清理路径。

## Shared Owners

| 模块 | Owner 责任 |
| --- | --- |
| `server/src/realtime_metrics.rs` | Realtime 模块门面，稳定导出计数、诊断目录和管理 API，避免业务入口依赖内部文件布局 |
| `server/src/realtime_metrics/counters.rs` | `RealtimeChannel` 标签、进程内关闭计数和持久化账本写入适配点 |
| `server/src/realtime_metrics/catalog.rs` | Channel/close reason 诊断目录、告警 bucket、first check 和变更规则 |
| `server/src/realtime_metrics/admin.rs` | 管理 API payload 组装和管理员鉴权 |
| `server/src/realtime_metrics_tests.rs` | 计数、诊断目录和 snapshot 契约测试，隔离全局计数测试锁 |
| `server/src/realtime_diagnostics_catalog.snapshot.json` | `realtime_diagnostics_catalog()` 的静态 JSON 契约快照，锁住 diagnostics API 字段、数组结构、alert bucket 和 first check |
| `server/src/router/admin_routes.rs` | Realtime 管理 API 路由挂载，包括 `/api/admin/realtime/close-metrics` 和 `/api/admin/realtime/diagnostics` |
| `server/src/store/realtime_close_events.rs` | SQLite close event ledger、30 天保留、窗口统计、Realtime 告警写入、Realtime/Billing 告警隔离，并从 `realtime_diagnostics_catalog()` 派生告警计数分类和 detail 的 first check |
| `server/src/assets/admin.html` | Realtime Health 面板、窗口切换、summary/detail/alerts 渲染，并消费 `/api/admin/realtime/diagnostics` 展示 category、alert bucket 和 first check |
| `/api/admin/realtime/diagnostics` | 机器可读诊断字典，导出 channel、close reason、alert bucket、入口模块和变更规则 |
| `scripts/test-admin-realtime-health.js` | 管理后台 Realtime 面板 smoke，覆盖 close metrics/diagnostics 请求、窗口选择、诊断提示渲染和 alert detail 转义 |
| `scripts/check-realtime-diagnostics-snapshot.ps1` | Realtime diagnostics JSON 快照契约门禁，校验快照结构、源码 `include_str!` 绑定和 snapshot 单测 |
| `docs/realtime-operations-runbook.md` | 线上巡检、告警判断、故障处置和发布前回归命令 |
| `scripts/check-realtime-ownership.ps1` | 静态校验本文档和源码的 channel/reason/owner 入口没有漂移 |

## Change Rules

新增 channel 时：

1. 在 `RealtimeChannel` 中新增稳定标签，并补 `realtime_channel_labels_are_stable`。
2. 更新 `realtime_diagnostics_catalog()`，让 `/api/admin/realtime/diagnostics` 返回新 channel。
3. 在业务入口用 `realtime_metrics::record_close_with_store(...)` 记录 close reason。
4. 更新本文档 Ownership Table 和 `docs/realtime-operations-runbook.md` 覆盖范围。
5. 更新 `server/src/realtime_diagnostics_catalog.snapshot.json`，并确保 `realtime_diagnostics_catalog_matches_snapshot` 通过。
6. 如果后台展示或分类受影响，更新 `server/src/assets/admin.html`、`scripts/test-admin-realtime-health.js`。
7. 更新 `scripts/check-realtime-ownership.ps1` 的静态校验条目。

新增 close reason 时：

1. 先在对应来源枚举中补稳定标签测试：`WsCloseReason`、`AgentSessionCloseReason` 或 `PeerWsCloseReason`。
2. 更新 `realtime_diagnostics_catalog()` 的 close reason 字典，明确 `alert_bucket`、`meaning` 和 `first_check`。
3. 判断是否应进入读错误、写失败或超时告警分类；如需要，更新 `alert_bucket`，`store/realtime_close_events.rs` 会按 bucket 派生计数分类，测试需要确保告警 detail 继续包含诊断字典的 first check。
4. 更新 `server/src/realtime_diagnostics_catalog.snapshot.json`，并确保 `realtime_diagnostics_catalog_matches_snapshot` 通过。
5. 更新后台颜色分类 `realtimeReasonColor(...)` 和前端 smoke。
6. 更新本文档、runbook 和 ownership guard。

新增实时 WS 入口时：

1. 服务端 Axum WS 优先复用 `ws_transport.rs`。
2. 客户端 `tokio-tungstenite` 发送语义复用 `server/src/ws_client_transport.rs`，不能和 Axum `Message` 类型混用。
3. 入口模块只负责业务协议和 close reason 映射，不重复实现窗口统计、阈值或告警表逻辑。
