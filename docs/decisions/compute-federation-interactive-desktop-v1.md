---
title: 算力联邦交互式云电脑扩展 V1
status: accepted
date: 2026-09-02
owners: backend, node, pc, security, ai-economy
implementation_status: source_contract_only_uncompiled_unrun
---

# 算力联邦交互式云电脑扩展 V1

## 背景

现有任务级分布式算力联邦面向可独立执行、重试、验证并返回结果的 AI 任务，核心链为 Provider、Offer、Job、Reservation、Attempt Lease、Execution Receipt 与 Settlement Receipt。新的产品方向允许用户把自己的 PC 作为云端电脑供给方，把游戏或桌面画面和系统音频实时传给付费消费者，并在授权时接收键鼠输入。

两类供给在身份、报价、容量、租约、计量和结算上高度相似，但执行语义不同。AI 任务有明确输入、终态和结果工件；云电脑是长生命周期、双向、低延迟、可重连的交互会话。仅把 `output.streaming=true` 或新增一个批处理 `task_kind` 无法表达媒体连接、输入控制、用户同意和断线接管。

## 决定

### 1. 一套联邦控制面，两个执行平面

Provider 身份、Offer 版本、CapacityPool、PriceSnapshot、容量 Claim、消费者预算、分层计量、争议和双价格腿结算继续作为共享控制面。

执行面分为：

- `batch_compute`：现有 ComputeJob、Reservation、Attempt 与结果回执；
- `interactive_desktop`：新的 SessionRequest、SessionReservation、HostLease、ViewerGrant、MediaEpoch、ControlEpoch 与会话回执。

云电脑不是把远端 GPU 虚拟成一张本地 GPU，也不是把实时画面当作 Job 结果工件。

### 2. 使用独立 service class

新能力固定 `service_class=interactive_desktop` 和 `interactive_desktop.*.v1` schema。首批使用独立 Offer Profile 和 Session 合同，不修改现有 `compute_federation.workload.v1` 的任务闭集，不让旧 Broker、节点或 Adapter 误以为自己支持云电脑。

未来若建立联邦通用 Demand 外壳，现有 ComputeJob 可投影为 `batch_compute` Demand，新 SessionRequest 可投影为 `interactive_desktop` Demand；历史 Job 和 Receipt 不迁移、不改写。

### 3. 复用商业对象，不复用 Attempt 实体

交互 Session 必须精确绑定 Provider、Consumer、Offer 版本与摘要、PriceSnapshot、Reservation/Claim 和整数预算。HostLease 复用租约到期、硬截止和 `fencing_generation` 原则，但拥有独立实体和状态机。

一次 Session 可经历多个 MediaEpoch；断线重连增加 epoch，不创建重复收费 Session。控制权由单调递增的 ControlEpoch 决定，旧连接和旧输入不能在重连后恢复权限。

### 4. 画面、音频和输入不走现有控制 WebSocket

现有节点 WSS 只承载身份、Offer/Answer 与 ICE 信令引用、短期 ticket、租约、心跳和有界统计。媒体采用 WebRTC：ICE/STUN 直连优先，TURN 中继兜底；画面和音频通过加密媒体通道，输入通过授权的数据通道。

不得把视频帧、音频帧或高频输入编码进当前 JSON 消息、数据库事件或业务日志。多人观看需要独立 SFU 决定，不在 V1 中隐式增加服务器转码或录制。

### 5. 云电脑共享拥有独立同意和紧急停止

AI 算力共享同意不能授权桌面捕获或远程输入。Provider 必须分别开启云电脑共享、选择捕获范围，并看到会话指示、消费者角色、资源与价格。每次控制会话需要 ViewerGrant 和本地策略共同允许。

Provider 本地终止、锁屏、注销、UAC/安全桌面、权限撤销、HostLease 过期或非当前 fencing generation 立即使远端控制失效。剪贴板、文件、麦克风、摄像头、USB、手柄和提权默认关闭；扩展必须逐项建立新权限合同。

### 6. 资源池统一防止重复出售

同一 GPU、硬件编码器、网络出口和交互式登录会话形成一个共享物理资源边界。AI Offer 与云电脑 Offer 若会争用同一资源，必须绑定同一 CapacityPool 或可原子关联的子池，不能分别发布完整额度。

会话槽是 reusable capacity；会话毫秒、出口字节和 TURN 中继字节可作为 consumable meter。输入事件、画面内容和按键内容不作为收费单位。

### 7. 会话计量继续分层

Provider 声明、传输层观测、Consumer 播放观测、verified usage 和 compensable usage 分开保存。有效收费区间必须位于当前 HostLease、当前 MediaEpoch、媒体实际发送和 Consumer 播放确认的交集中；连接等待、无媒体、过期或被本地拒绝的时间不能自动计费。

TURN 成本独立于 Provider 收益，PriceSnapshot 明确消费者价格腿、Provider 价格腿和平台/中继成本。所有金额、时长、字节和比率使用整数单位。

### 8. 商业模式与游戏授权必须进入合同

Session 和 Offer 必须显式选择 `same_owner_remote_access`、`friend_co_play` 或 `licensed_cloud_seat`。同账号模式先用于技术闭环且不产生 Provider 分成；好友共玩是邀请式、非公开市场；只有具备发布商、平台、地区、年龄、DRM 和反作弊兼容证据的 `licensed_cloud_seat` 才能进入陌生用户付费市场。

个人账号中的游戏订阅或游戏库不能被一龙默认解释为可转租权。标题、版本或权利证据未知时失败关闭；不得收集或转交游戏平台密码，不得通过 Hook、驱动、提权或协议隧道绕过 DRM 与反作弊。陌生用户会话首期固定 `relay_only`，通用任意桌面/VPS 能力另立产品与滥用治理边界。

### 9. 生产默认关闭并分阶段开放

首批只写 accepted 文档和内部 Rust 源合同，不增加 migration、route、公开 API、endpoint capability、TURN credential、真实媒体、输入、账单写入或生产开关。后续能力只有在自己的动态证据和上线门禁通过后才能逐层打开。

## 媒体实现方向

Windows Host 首选 Windows Graphics Capture 或 DXGI Desktop Duplication 获取 GPU surface，Media Foundation 或厂商硬件编码器生成低延迟 H.264；系统音频使用 WASAPI loopback。受保护内容由操作系统和应用策略决定，项目不得绕过保护。

WebRTC 负责浏览器/桌面消费者的媒体、数据通道、加密和 NAT 穿透。W3C WebRTC 定义媒体与通用 data channel，ICE 使用 STUN/TURN 候选；这些标准只确定传输方向，不替代一龙自己的账号、租约、授权和计量合同。

## 被否决的方案

- 把 `video_generation` 或 `ComputeOutputContract.streaming` 改名为云电脑；
- 让现有 ComputeAttempt 同时承担批任务和可重连桌面会话；
- 用节点 WSS/Base64 转发 60 FPS 视频；
- 复用 AI 算力共享开关、长期设备 bearer 或项目写权限授权看屏/控屏；
- 只靠 Provider 自报在线时间计费；
- 默认开放全桌面、剪贴板、文件、麦克风、摄像头、手柄或管理员输入；
- 绕过 DRM、受保护画面、反作弊或游戏发行方限制；
- 把个人 Steam 等游戏平台账号或游戏库默认包装成陌生人付费云座位；
- 在无 TURN、无本地终止、无断线 fencing 的情况下开放陌生付费用户。

## 后果

- 新能力能够复用成熟度较高的联邦商业语言，而不破坏批处理合同；
- Session 域会增加独立状态、权限和回执，但这些复杂性真实存在，不能藏在 `streaming` 布尔值后面；
- 节点插件权限、硬件画像、生命周期、watchdog 和更新 drain 都需要认识活动云电脑会话；
- TURN 带宽、硬件编码器会话数、游戏授权和反作弊兼容成为新的成本与风险；
- 在真实媒体闭环与安全验收前，项目只能宣称“合同与架构已铺设”。

## 分阶段落地

1. 本决定、需求、架构和 `interactive_desktop.v1` 内部合同；
2. 独立策略、能力画像和本机/LAN 720p H.264 技术闭环；
3. 一次性信令 ticket、WebRTC ICE/STUN/TURN、单一受信消费者；
4. Session Store、HostLease fencing、Media/Control epoch、恢复和生命周期接线；
5. 周期计量、Verification、争议、退款和会话结算；
6. 仅对授权 `licensed_cloud_seat` 做小范围付费灰度，再扩展游戏兼容清单、Android/TV 与后续手柄能力。

## 验证状态

本决定已接受。2026-09-02 首批为 `source_contract_only_uncompiled_unrun`：文档和内部类型可以提交，但不编译、不运行、不迁移，也不产生真实画面、输入、网络、计费或资金效果。动态实现状态只由后续专题验收证据提升。

## 标准参考

- [W3C WebRTC Recommendation](https://www.w3.org/TR/webrtc/)
- [IETF RFC 8445：ICE](https://www.rfc-editor.org/info/rfc8445/)
- [IETF RFC 8826：WebRTC 安全考虑](https://www.rfc-editor.org/info/rfc8826/)
- [Microsoft Windows Graphics Capture](https://learn.microsoft.com/en-us/windows/uwp/audio-video-camera/screen-capture)
- [Microsoft Desktop Duplication API](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api)
- [Steam Remote Play 官方文档](https://partner.steamgames.com/doc/features/remoteplay?language=english)
- [Steam Subscriber Agreement](https://store.steampowered.com/subscriber_agreement/)
