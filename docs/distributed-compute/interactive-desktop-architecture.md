---
title: 分布式算力联邦交互式云电脑架构
status: current
reviewed_at: 2026-09-02
owners: backend, node, pc, security, ai-economy
implementation_status: source_contract_only_uncompiled_unrun
---

# 分布式算力联邦交互式云电脑架构

## 1. 产品定义

交互式云电脑把 Provider PC 上用户明确选择的桌面或窗口变成一段付费、可撤销的实时会话。Provider 发送画面和可选系统音频；Consumer 接收并显示，在 ViewerGrant 允许时发送键盘或鼠标输入。游戏串流是该能力的首要场景，但合同保持为 `interactive_desktop`，以便同一安全边界承载远程应用或工作站。

它与 AI 算力共享的关系是“共享交易控制面、分离执行数据面”。两者都出售有界资源，但 AI Job 交付结果，云电脑 Session 交付持续的交互体验。

### 1.1 产品模式

| 模式 | 允许范围 | 市场与结算 |
|---|---|---|
| `same_owner_remote_access` | 同一账号在自己的 Host 与 Viewer 间远程访问 | 第一阶段技术闭环；不产生 Provider 分成 |
| `friend_co_play` | Host 主动邀请的受信 Viewer；不公开挂牌 | 不自动获得商业出租权，仍受具体游戏条款约束 |
| `licensed_cloud_seat` | 已审核标题、版本、地区、年龄、DRM、反作弊和商业串流授权 | 唯一允许进入陌生用户付费市场的模式 |

个人游戏平台账号或游戏库不是默认可出售资产。未知权利、未知标题版本或未知反作弊兼容性均失败关闭；系统不索取或转交游戏平台密码。通用任意桌面、浏览器、终端或宿主局域网访问会引入 VPS/出口网络与恶意软件托管风险，必须作为另一产品治理。

## 2. 共享与独立边界

| 领域 | 与 AI 算力共用 | 云电脑独立对象 |
|---|---|---|
| 身份 | Account、Device、ComputeProvider、所有权与信任 | Host 当前交互式用户、Viewer 设备、一次性 Grant |
| 商品 | Offer 版本、PriceSnapshot、授权范围 | InteractiveOfferProfile、codec/QoS/输入与捕获声明 |
| 容量 | CapacityPool、Bucket、Claim、原子防超卖 | session slot、encoder slot、session_ms、egress/relay bytes |
| 调度 | Broker 的硬过滤与价格排序原则 | SessionBroker、网络/QoS/交互式登录状态过滤 |
| 租约 | 到期、硬截止、fencing generation | HostLease、ViewerGrant、MediaEpoch、ControlEpoch |
| 数据面 | 内容摘要和短控制事件原则 | WebRTC 媒体与 input data channel |
| 计量 | declared/observed/verified/compensable 分层 | 媒体有效区间、播放确认、QoS、direct/TURN 路径 |
| 结算 | 不可变快照、双价格腿、挑战与纠正 | 会话专用 Usage/Terminal/Settlement Receipt |

现有 `ComputeWorkloadSpec`、`ComputeJob`、`ComputeAttemptLease` 和 `ComputeExecutionReceipt` 继续只描述批处理。新模块不能被当前 batch Broker 解析或派发。

## 3. 总体架构

```text
Provider PC                                      Consumer
┌──────────────────────────┐                    ┌──────────────────────┐
│ Compute Bootstrap        │                    │ PC Web / Tauri       │
│ Remote Desktop Host      │── WebRTC media ──▶│ video/audio decoder  │
│ capture/audio/encoder    │◀─ data channel ───│ keyboard/mouse input │
│ local consent + kill     │                    │ local input release  │
└────────────┬─────────────┘                    └──────────┬───────────┘
             │ WSS control / signaling                       │ HTTPS/WSS
             ▼                                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 一龙控制面                                                         │
│ Provider / Offer / Capacity / Price / SessionBroker / HostLease     │
│ ViewerGrant / signaling ticket / meter / verification / settlement │
└──────────────────────┬──────────────────────────────────────────────┘
                       │ short-lived credentials
                       ▼
             STUN discovery / TURN encrypted relay
```

控制面从不承载视频或输入正文。P2P 成功时媒体不经过一龙服务器；TURN 只转发加密包，仍需独立记录中继流量和成本。

`licensed_cloud_seat` 与其他陌生用户模式首期强制 TURN `relay_only`，不向交易双方公开对端候选地址。同账号或受信好友的直连能力只能由独立网络策略显式开放，不能从付费 Offer 继承。

## 4. 核心合同

### 4.1 InteractiveOfferProfile

Profile 是不可变 Offer 扩展，至少包含：

- `service_class=interactive_desktop`、schema、profile ID/digest；
- 捕获类型：用户选择的 display 或 window；
- 最大宽高、FPS、bitrate、SDR/HDR、光标策略；
- video codec/profile、audio codec/channel/sample-rate；
- encoder family 与经验证硬件能力摘要；
- view/control 角色、键盘/鼠标和未来 gamepad 能力；
- direct/TURN transport policy、区域和 SLA 档；
- 本地同意、指示、终止和 fail-closed policy revision；
- 规范 meter 名称。

Profile 声明最大能力，不是当前 Ready 证明。节点必须另行提供短 TTL `NodeInteractiveDesktopReadyCapability`，绑定当前登录会话、显示器、编码器、音频、输入和网络探测。

### 4.2 InteractiveSessionRequest

Consumer 请求期望的分辨率、FPS、audio/input、最大时长、区域、SLA、数据等级和最大消费者收费。Request 不携带主机地址、SDP、ICE 或凭据。

### 4.3 InteractiveSessionReservation

Broker 选择一个 active Offer/Profile 和 PriceSnapshot，在单一容量边界中冻结：

- 一个 `interactive_session_slot`；
- 允许的最大 `session_ms`；
- 允许的 `egress_bytes`，以及需要时的 `relay_bytes` 预算；
- 消费者最大费用。

Reservation 精确绑定 SessionRequest、Offer 版本/摘要、Profile digest、PriceSnapshot、Claim revision/digest 和到期时间。

### 4.4 InteractiveHostLease

HostLease 授权一台精确 Provider/Executor 提供一段 Session。它包含 `lease_id`、`session_id`、`fencing_generation`、soft expiry 和 hard deadline。续租只能在 hard deadline 内延长；过期或旧 generation 不可复活。

HostLease 不包含可重放认证材料。节点身份、一次性 ticket 与短期 TURN credential 由私密信道传递，业务 Store 只保存不可逆引用或摘要。

### 4.5 ViewerGrant

ViewerGrant 精确绑定 Consumer account、Consumer device、Session、角色和过期时间：

- `view`：仅接收媒体；
- `control`：在 view 基础上允许合同列出的输入；
- 首期最多一个 Controller。

长期设备登录、项目读写权、AI Provider 凭据或节点 bearer 均不能替代 ViewerGrant。

### 4.6 MediaEpoch 与 ControlEpoch

Session ID 在有限重连期间稳定。每次 WebRTC connection takeover 或重连创建递增 `media_epoch`；旧 epoch 的媒体、统计和信令失效。每次控制权授予、撤销或重新连接创建递增 `control_epoch`；输入消息必须同时匹配当前 HostLease fencing、MediaEpoch 和 ControlEpoch。

Provider 本地按键、鼠标或紧急热键优先。断开时 Host 必须合成完整 key-up/button-up 恢复，避免远端按键卡住。

### 4.7 Usage 与 Terminal Receipt

会话期间按序追加累计快照：

- `provider_declared`：Host 捕获/编码/发送计数；
- `transport_observed`：信令/TURN/transport gateway 的连接和字节计数；
- `consumer_observed`：解码、播放、掉帧和输入 ACK；
- `verified`：Verification 接受的有效区间与 meter；
- `compensable`：受 Offer、Reservation 和 SLA 上限约束的 Provider 可补偿用量。

终态回执绑定上述证据摘要、终止原因、最终 HostLease/MediaEpoch/ControlEpoch、Offer、Profile、PriceSnapshot 和容量因果链。原始画面、音频、输入、SDP、ICE 和密钥不进入回执。

## 5. 状态机

```text
requested -> quoted -> reserved -> awaiting_owner_consent
  -> signaling -> connecting -> active <-> reconnecting
  -> draining -> ended -> verification_pending -> settled
```

任意未开始阶段可进入 `canceled` 或 `expired`；连接/活动阶段可进入 `failed`。状态含义：

- `awaiting_owner_consent`：Provider 本地尚未批准捕获目标和权限；
- `signaling`：只交换短期信令，不计有效媒体时长；
- `connecting`：ICE/DTLS/codec 协商，不计有效媒体时长；
- `active`：当前租约、epoch 和媒体证明都有效；
- `reconnecting`：有限宽限期，不自动计费；
- `draining`：拒绝新控制与重连，释放按键并停止媒体；
- `ended`：Host 已停止捕获、音频、编码和输入；
- `verification_pending`：证据待验证，不代表可结算；
- `settled`：会话专用 Settlement Receipt 已生成。

## 6. Windows Host 数据面

### 6.1 捕获

首选 Windows Graphics Capture，让 Provider 通过系统 UI 或本地可信 UI 选择 display/window。需要全桌面或兼容全屏 DirectX 时可评估 DXGI Desktop Duplication；它提供 GPU surface、脏矩形、移动区域和光标信息。现有 `PrintWindow/GetDIBits` 单帧 PNG 只属于 UI 证据链，不能作为游戏串流实现。

捕获器必须处理 resize、device lost、显示器移除、HDR/SDR 变化和受保护内容。黑屏、受保护帧或系统拒绝不是可收费成功。

### 6.2 编码

首期使用低延迟 H.264，优先 Media Foundation 硬件 MFT 或经签名 sidecar 访问 NVENC/AMF/QSV。软件编码只能作为明确降级，不能隐式突破 CPU/温度上限。编码能力画像包含实现、codec/profile、最大分辨率/FPS、并发会话数和观测摘要。

编码与 AI 推理会争用 GPU、显存、功耗和散热；本机资源仲裁器可以降低码率、暂停 AI 领取或拒绝会话，但不能扩大 Offer。

### 6.3 音频

系统音频使用 WASAPI loopback，并独立征得同意。首期不回传 Consumer 麦克风。音画使用同一单调时钟生成时间戳，重连后重新建立 epoch，不拼接旧时间线。

### 6.4 输入

键盘和鼠标通过独立授权通道传递：

- 可靠有序通道处理按键、按钮和状态快照；
- 高频相对鼠标可用带 sequence 的低延迟通道；
- Host 校验 session/fencing/media/control epoch、允许键集合和速率；
- 周期完整状态与断线 key-up 防止卡键；
- Windows `SendInput` 受 UIPI 限制，不能控制更高完整性应用或安全桌面；这种失败必须显式报告。

手柄通常需要额外驱动、虚拟设备和管理员边界，首期不实现。

## 7. 网络与 WebRTC

### 7.1 信令

一龙 WSS 只接受已认证、未过期、精确绑定 Session/Viewer/device 的一次性 SignalingTicket。信令消息有大小、频率、顺序和状态上限；日志只保存摘要与结果，不保存完整 SDP、ICE candidate、IP 或 TURN secret。

### 7.2 连接

WebRTC 建立 DTLS-SRTP 媒体和加密 data channel。ICE 依次使用 host/server-reflexive/relayed 候选；P2P 不可达时使用 TURN。Consumer 不能向任意地址驱动 Host 发送流量，双方必须完成协议同意与会话鉴权。

### 7.3 自适应

Host 根据 RTT、jitter、loss、available bitrate、encode queue 和 decode/drop feedback 调整 bitrate、resolution 和 FPS。降级不改变 PriceSnapshot；是否满足 SLA 由 Verification 决定。

## 8. 权限与隐私

权限分四层，缺一不可：

1. Provider policy：用户是否开启云电脑、允许哪些捕获与输入；
2. HostLease：哪台 Host 在哪个 generation 提供资源；
3. ViewerGrant：谁可以 view/control；
4. Media/Control epoch：当前哪条连接和控制代次有效。

安全默认值：

- 画面：无捕获目标；
- 音频：关闭；
- 输入：关闭；
- 剪贴板/文件/USB/麦克风/摄像头/提权：关闭且首期不可请求；
- 录制：服务端和客户端均不由 V1 合同授权；
- 日志：仅保存有界状态、摘要、整数统计和错误码。

游戏或应用发行条款、DRM、受保护画面与反作弊兼容必须单独评审。Offer 只能声明经验证的兼容范围，不能承诺绕过系统或第三方保护。

可收费 Offer 还必须绑定不可变 TitlePolicySnapshot：商店/发布商与 App ID、可执行文件身份、地区和年龄范围、商业串流权利证据、DRM/反作弊结论、允许的捕获与输入方式、审核人和有效期。任一字段未知或过期时，Broker 不得创建付费 Reservation。

## 9. 容量与计价

建议规范 meter：

| meter | 类型 | 用途 |
|---|---|---|
| `interactive_session_slot` | reusable | 同时活动 Session 数 |
| `session_ms` | consumable | verified 有效媒体时长 |
| `egress_bytes` | consumable | Provider 实际媒体出口 |
| `relay_bytes` | consumable | TURN 中继成本，独立于 Provider 收益 |
| `encode_gpu_ms` | optional consumable | 只有硬件与 Verification 能可靠证明时才启用 |

帧数、解码帧、掉帧、RTT、jitter、loss、码率和输入 ACK 是 SLA/验证证据，不默认直接收费。PriceSnapshot 仍使用整数微单位和明确舍入规则。

同一物理 GPU 发布 AI 与云电脑 Offer 时，Pool 必须覆盖 GPU、encoder session、内存、网络和登录会话的共同约束。不能把一个 `max_concurrent_runs=1` 同时复制成一个 AI slot 和一个云电脑 slot。

## 10. 失败与恢复

- Provider 本地拒绝：释放 Reservation/Claim 和消费者预授权，零会话用量；
- ICE/TURN 失败：在连接期限内重试，未 active 不计 session_ms；
- 网络短断：进入 reconnecting，使用新 MediaEpoch；超过宽限期结束；
- HostLease 过期：立即停止输入和媒体，旧心跳不可复活；
- Consumer 离开：释放所有输入状态，等待短重连或进入 draining；
- Host 锁屏/UAC/注销：停止控制和媒体并报告结构化原因；
- Encoder/device lost：有界重建；改变 codec/profile 需要新 epoch，超出 Offer 则终止；
- 双方统计不一致：进入 Verification/争议，资金保持 pending；
- 更新或 watchdog：活动 Session 是 drain blocker，不得静默杀死后自动当作成功。

## 11. 模块边界

目标结构：

```text
server/src/compute_federation/interactive_desktop/  # 领域合同
server/src/interactive_desktop/                     # 后续 Store/Service/Broker/信令
server/src/node_agent_interactive_desktop/          # 后续节点控制协议
node sidecar: remote-desktop-host                    # 捕获/音频/编码/WebRTC/输入
pc-frontend/src/features/cloud-pc/                   # Provider 与 Consumer 页面
android feature/cloudpc/                             # 后续消费者播放器
```

`node_agent_android_live`、CDP 截图、ChatGPT 语音 WebRTC 和旧 `LlmStreamRequest` 不迁移为此模块；可复用其会话恢复或 UI 模式，但不能复用权限或媒体事实。

## 12. 分阶段验收

| 阶段 | 用户能力 | 验证要求 | 生产状态 |
|---|---|---|---|
| C0 合同 | 架构与类型可审查 | 静态合同和兼容保护 | closed |
| C1 本机/LAN | 自己的 Host 与 Viewer 连续串流 | 真帧、音画同步、键鼠、断线、kill switch | test-only |
| C2 受信远端 | 单一受信账号走互联网 | ICE/TURN、安全、QoS、恢复 | allowlist |
| C3 联邦控制面 | Offer/Reservation/HostLease/计量 | migration、Store、API、并发、磁盘恢复 | no billing |
| C4 付费灰度 | 授权 `licensed_cloud_seat` 的 verified usage 与退款/结算 | 权利快照、争议、账本、真实网络、监控 | limited |
| C5 产品化 | PC/Android/TV、兼容清单 | 设备矩阵、游戏条款、长期稳定性 | gated |

每一阶段的实现、验证、交付和验收状态独立记录。前一阶段的源码存在不能替代后一阶段的真实证据。

## 13. 当前实现状态

2026-09-02 当前仓库只有外围能力：节点 WSS、Provider/Offer/Capacity/Price/Receipt 控制面、签名插件设计、Tauri 生命周期和单帧 GDI 证据截图。没有游戏级连续捕获、硬件编码、WASAPI loopback、Windows 输入注入、通用 WebRTC/ICE/TURN、Session Store 或付费会话接线。

本批新增内部源合同仍是 `source_contract_only_uncompiled_unrun`。没有 migration、公开 route、节点 capability、媒体、输入、计量写入、结算或生产效果。

## 14. 相关文档

- [`需求`](../requirements/interactive-desktop-federation-v1.md)
- [`接受决定`](../decisions/compute-federation-interactive-desktop-v1.md)
- [`任务级联邦原决定`](../decisions/distributed-compute-federation-v1.md)
- [`联邦架构`](architecture.md)
- [`节点与插件`](node-client-and-plugins.md)
- [`市场与结算`](market-and-settlement.md)
