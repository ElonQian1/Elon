# Realtime 运维 Runbook

本文档固定实时 WebSocket 链路的巡检、告警判断和故障处置方法。目标是让 `/ws/app`、`/ws/notify`、项目任务 WS、语音 WS、HomeCLI agent 和手机 P2P relay 出现断开异常时，可以从同一套指标和 close reason 语义快速定位问题。

模块 owner、源码入口和 close reason 来源由 `docs/realtime-channel-ownership.md` 固定；改实时通道前先对照 owner 表，避免 channel、reason、告警和后台展示漂移。

## 覆盖范围

| Channel | 典型入口 | 主要影响 |
| --- | --- | --- |
| `app_notify` | `/ws/notify` | 旧版移动端通知、升级提示 |
| `global_app` | `/ws/app` | 用户在线、好友状态、通用推送 |
| `project_ws` | 项目任务 WebSocket | 项目任务状态、审批、执行流 |
| `voice_transcribe` | 实时转写 WS | 语音转文字 |
| `voice_realtime_chat` | 实时语音聊天 WS | AI 语音对话 |
| `voice_virtual_mic` | 虚拟麦克风 WS | 语音输入链路 |
| `homecli_agent` | HomeCLI agent 反向长连接 | PC CLI 调度、节点在线 |
| `peer_relay` | 手机 P2P APK relay | APK 局域网/中继传输 |

## 日常巡检

1. 打开管理后台 Realtime 标签页，查看 Realtime Health。
2. 优先看 `last_1h`，再对比 `last_24h` 和 `all_time`，判断是短时尖峰还是长期趋势。
3. 若页面不可用，直接请求管理接口：

```powershell
Invoke-RestMethod `
  -Headers @{ Authorization = "Bearer <ADMIN_TOKEN>" } `
  -Uri "https://<server>/api/admin/realtime/close-metrics"
```

返回结构应包含：

- `metrics`：兼容字段，等同 `windows.all_time`。
- `windows.last_1h`：最近 1 小时持久化断开事件。
- `windows.last_24h`：最近 24 小时持久化断开事件。
- `windows.all_time`：保留期内全部持久化断开事件。
- `windows.process`：当前进程内存计数，适合判断重启后的即时趋势。
- `alerts`：当前打开的 Realtime 告警。

需要机器可读诊断字典时请求：

```powershell
Invoke-RestMethod `
  -Headers @{ Authorization = "Bearer <ADMIN_TOKEN>" } `
  -Uri "https://<server>/api/admin/realtime/diagnostics"
```

该接口返回 `channels`、`close_reasons`、`alert_bucket`、入口模块和变更规则，是后台、runbook 和 AI 运维说明共用的诊断定义源。

## Close Reason 语义

| close_reason | 含义 | 首查方向 |
| --- | --- | --- |
| `peer_closed` | 客户端正常关闭 | 客户端切后台、页面离开、版本行为 |
| `client_control_close` | 客户端发控制消息主动关闭 | 客户端逻辑、用户主动停止语音/任务 |
| `reader_ended` / `peer_reader_ended` | 读取流自然结束 | 网络断流、代理超时、客户端进程退出 |
| `read_error` / `peer_read_error` | 服务端读取 WS 帧失败 | 网络质量、反代、协议兼容、客户端异常退出 |
| `write_failed` / `peer_write_error` | 服务端向客户端写入失败 | 客户端半断开、下游背压、代理断开 |
| `pong_write_failed` | 服务端回应 Ping 失败 | 连接已半断开、网络抖动、代理保活问题 |
| `reader_timeout` | HomeCLI agent 读超时 | PC 节点假在线、心跳丢失、本机休眠或代理问题 |
| `writer_closed` | HomeCLI agent 写侧关闭 | PC 节点写循环退出、发送队列关闭 |

## 告警阈值

阈值通过管理后台配置卡和配置弹窗维护，底层配置键为：

| Key | 默认值 | 告警含义 |
| --- | ---: | --- |
| `realtime_close_read_error_alert_threshold_1h` | `20` | 最近 1 小时读错误超过阈值 |
| `realtime_close_write_failure_alert_threshold_1h` | `20` | 最近 1 小时写失败超过阈值 |
| `realtime_close_timeout_alert_threshold_1h` | `5` | 最近 1 小时读超时超过阈值 |

阈值最小有效值为 `1`。环境变量同名大写形式会优先于数据库配置，例如 `REALTIME_CLOSE_TIMEOUT_ALERT_THRESHOLD_1H`。

## 处置流程

### 读错误升高

1. 在 Realtime Health 中确认主要 channel。
2. 如果集中在 `global_app`、`app_notify` 或语音 channel，先看客户端版本、网络切换、反向代理 WS timeout。
3. 如果集中在 `project_ws`，同时检查任务流是否有异常重连或长时间无消费。
4. 若读错误伴随 `reader_ended` 同步升高，优先判断客户端/代理主动断流；若只有 `read_error` 升高，优先查协议、帧格式或连接中断。

### 写失败升高

1. 确认是否有大量客户端切后台、刷新页面、移动网络切换。
2. 检查服务端是否在向已断开的连接持续发送消息。
3. 对 `project_ws`，重点查任务事件广播和审批推送是否有慢消费者。
4. 对语音 channel，重点查音频发送节奏和客户端停止录音后的连接关闭顺序。

### 超时升高

1. 优先检查 `homecli_agent`，确认 PC 节点是否休眠、网络断开或代理失效。
2. 查看节点管理页面，确认节点在线状态是否和实际连接一致。
3. 对反复超时的 agent，触发节点重连或让客户端刷新本地运行时。
4. 如果超时伴随 pending CLI 请求失败，优先保护用户任务状态，避免继续等待假在线节点。

### P2P Relay 失败

1. 如果 `peer_read_error` 或 `peer_reader_ended` 升高，先看种子端是否退出或手机/PC 是否不在同一网络。
2. 如果 `peer_write_error` 升高，检查下载端是否中途断开、APK 文件是否过大、网络是否不稳定。
3. 如果注册确认发送失败，清理对应 registry 后让种子端重新注册。

## 发布前回归

涉及实时通道、管理后台 Realtime 面板、告警阈值或 close reason 的变更，至少运行：

```powershell
cargo fmt --manifest-path server\Cargo.toml -- --check
cargo check --manifest-path server\Cargo.toml
cargo test --manifest-path server\Cargo.toml ws_transport --quiet
cargo test --manifest-path server\Cargo.toml realtime_metrics --quiet
cargo test --manifest-path server\Cargo.toml realtime_close_events --quiet
```

如变更 `server/src/assets/admin.html`，还需要检查内联脚本语法：

```powershell
@'
const fs = require('fs');
const html = fs.readFileSync('server/src/assets/admin.html', 'utf8');
const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/gi)].map(m => m[1]).join('\n');
new Function(scripts);
console.log('admin inline scripts syntax ok');
'@ | node -
```

## 边界原则

- `ws_transport.rs` 负责 Axum WebSocket 帧、Ping/Pong、Close 和 JSON 文本帧基础语义。
- `ws_client_transport.rs` 负责 `tokio-tungstenite` 客户端侧发送语义，不能和 Axum `Message` 混用。
- `realtime_metrics.rs` 负责统一 `(channel, close_reason)` 计数、管理 API payload 和未来指标后端适配点。
- `store/realtime_close_events.rs` 负责持久化窗口、30 天保留、Realtime 告警写入和告警列表隔离。
- 各业务入口只负责把自己的断开原因映射为稳定 close reason，不在业务代码里重复实现指标窗口、阈值或告警表逻辑。
