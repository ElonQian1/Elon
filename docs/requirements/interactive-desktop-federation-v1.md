---
title: 分布式算力联邦交互式云电脑 V1 需求
version_status: current
status: accepted
reviewed_at: 2026-09-02
owners: backend, node, pc, security, ai-economy
implementation_status: source_contract_planned
---

# 分布式算力联邦交互式云电脑 V1 需求

## 目标

一龙允许用户在独立同意后，把自己的 Windows PC 作为 `user_node` Provider 提供交互式云电脑能力。消费者购买的是一段有界、可撤销、可计量的桌面会话：Provider PC 采集用户明确选择的显示器或窗口，编码并传输画面和系统音频，消费者端显示画面，并在授权范围内回传键盘、鼠标或后续手柄输入。

云电脑与 AI 批处理共享 Provider、Offer、CapacityPool、PriceSnapshot、Reservation/Claim、分层计量、争议和双价格腿结算原则，但使用独立的 Session、HostLease、ViewerGrant、MediaEpoch、ControlEpoch 和会话回执。不得把 `ComputeWorkloadSpec.output.streaming`、`video_generation` 或 `ComputeAttemptLease` 解释成远程桌面能力。

## 用户可观察能力

### Provider

- AI 算力共享与云电脑共享是两个独立、默认关闭的开关；开启其一不能隐式授权另一个。
- 开启前展示捕获范围、音频、键鼠、网络、预计带宽、资源占用、价格和风险；缺少明确同意时不得下载或启动云电脑 Host。
- 会话期间持续显示本地可见指示，提供托盘终止入口和本地紧急热键；Provider 终止优先于远端控制。
- 锁屏、注销、UAC/安全桌面、捕获权限撤销、受保护内容或策略冲突时失败关闭。

### Consumer

- 在报价中看到分辨率、帧率、codec、输入类型、延迟档、P2P/TURN 条件、计费 meter 和兼容性声明。
- 连接前获得一次性、短期、精确绑定会话与设备的 ViewerGrant；默认只有 `view`，`control` 必须显式授予。
- 断线可在有限宽限期内创建新的 MediaEpoch；旧连接、旧输入和旧 ticket 不能复活。
- 只为验证后的有效媒体会话区间与合同内流量付费；连接等待、长时间无媒体、过期租约和 Provider 本地拒绝不得计为有效交付。

## 联邦兼容要求

1. 保留现有 `compute_federation.*.v1` 批处理合同和闭集任务校验，不在本阶段添加可被 Broker 接受的假 `task_kind`。
2. 新增稳定 `service_class=interactive_desktop`，通过独立扩展合同绑定精确 Offer 版本、PriceSnapshot、Capacity Claim 和 Session Lease。
3. 同一物理 GPU、编码器、网络出口和交互式登录会话必须进入同一个 CapacityPool 或明确关联的资源边界，防止 AI Offer 与云电脑 Offer 重复出售。
4. 控制面 WebSocket 只承载认证、信令、租约、状态和有界统计；不得用现有 JSON/WebSocket 通道转发视频帧或输入正文。
5. 媒体面采用 WebRTC，ICE/STUN 直连优先、TURN 中继兜底；媒体、音频和输入通道必须加密，TURN 使用短期凭据。
6. 所有金额、价格、时长、字节和比率使用整数单位，不使用浮点金额。
7. Provider 声明、传输层观测、Consumer 观测与 verified/compensable usage 分开保存；结算不能只信任任一端自报。
8. 屏幕内容、音频内容、按键正文、剪贴板、SDP、ICE candidate、长期 bearer、Cookie 和凭据不得进入持久业务回执或普通日志。

## V1 产品边界

- 单个 Provider PC、单个 Consumer、单个 Controller。
- 首期 Windows Host；消费者优先 PC Web/Tauri，Android/TV 与手柄在后续阶段。
- 首期一个显示器或一个窗口、SDR、H.264、系统音频可选、键盘和鼠标可选。
- 不保证所有游戏、反作弊系统、DRM 或受保护视频可捕获或接受远程输入；兼容性必须由显式 profile 和真实验收证明。
- 不提供任意文件传输、剪贴板同步、USB 重定向、摄像头、麦克风回传、管理员提权或安全桌面控制。
- 不在服务端保存、转码或录制用户画面；多人旁观和 SFU 不属于首期。

## 第一批源码范围

第一批只完成：

- accepted ADR、架构和安全边界；
- `interactive_desktop.v1` Offer、Session、权限、租约、MediaEpoch 与计量回执内部 Rust 合同；
- 明确的状态转换、fencing、整数 meter、敏感载荷排除和批处理兼容保护测试源码；
- Feature Registry 登记和实现证据绑定。

第一批不新增 migration、公开 API、节点 capability、信令入口、TURN 凭据、真实媒体、输入注入、计费写入或生产开关。按当前架构铺设约束，源码只做静态验收，不编译、不运行。

## 验收标准

1. 文档明确“一套联邦控制面、两个执行平面”，并给出共享和独立合同的精确边界。
2. 新 Rust 合同使用独立 `interactive_desktop.v1` schema、`service_class` 和摘要域，不修改或放宽现有批处理 Workload 闭集。
3. Session 合同同时绑定 Provider、Consumer、精确 Offer/PriceSnapshot/Reservation/Claim、HostLease fencing、ViewerGrant、MediaEpoch 和 ControlEpoch。
4. 权限合同默认拒绝控制、音频、剪贴板、文件、麦克风、摄像头和提权；首期仅允许显式选择的画面及可选键鼠。
5. 计量合同区分 declared/transport_observed/consumer_observed/verified，使用累计整数并记录 direct/TURN 路径，不保存媒体或输入正文。
6. 合同源码明确旧 MediaEpoch、旧 ControlEpoch、过期 Grant、过期 HostLease 和非当前 fencing generation 不具备权限。
7. 现有 `llm_chat`、图像/视频生成、GPU batch、旧节点共享和批处理结算合同保持不变。
8. 生产、媒体、输入和资金效果均保持关闭，未编译/未运行状态被明确记录。

## 后续真实完成门槛

源合同完成不等于云电脑可用。真实上线至少还要依次取得：

1. Windows Graphics Capture/DXGI 连续帧与硬件编码能力探测；
2. WASAPI 系统音频、WebRTC H.264/Opus、ICE/TURN 和消费者解码；
3. 本机可见同意、一键终止、键鼠状态恢复、锁屏/UAC fail-closed；
4. 本机/LAN 的连续串流、音画同步、输入闭环和断线恢复证据；
5. 互联网单一受信用户、TURN、QoS 与滥用防护验收；
6. Session Store、容量预留、周期计量、验证、争议和退款；
7. 付费灰度、游戏授权/条款审核、生产监控和真实用户验收。

在这些门槛完成前，任何 UI、API 或文档都不得宣称“付费云电脑已上线”或“所有 3A 游戏均可远程运行”。
