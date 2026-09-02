---
title: 分布式算力联邦交互式云电脑 V1 需求
version_status: current
status: accepted
reviewed_at: 2026-09-02
owners: backend, node, pc, security, ai-economy
implementation_status: authority_kernel_source_written_fail_closed_uncompiled_unrun
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

- 产品模式必须显式区分：`same_owner_remote_access`（同账号远程访问自己的 PC，不产生 Provider 分成）、`friend_co_play`（邀请式、非公开市场）与 `licensed_cloud_seat`（具备发布商、平台及反作弊兼容授权的付费云座位）。
- 只有 `licensed_cloud_seat` 可以进入陌生用户付费市场；个人账号或个人游戏库不得被默认包装成可出租云座位。标题、版本、地区、年龄、DRM、反作弊或商业串流权利任一未知时失败关闭。
- 陌生用户模式首期强制 `relay_only`，避免向交易双方暴露对端网络地址；同账号或受信好友是否允许直连由独立策略决定。
- 一龙不得索取、保存或转交 Steam 等游戏平台密码；通用任意桌面、浏览器、终端和宿主局域网访问不属于本产品首期。
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

## 第二批源码范围：C1 权威内核

第二批在仍不开放生产入口的前提下，把第一批“调用方必须自行解析”的摘要约束下沉为可复用的权威内核源码：

- 使用 RFC 8785/I-JSON 规范 JSON、对象专属摘要域和 SHA-256 重算 Request、Profile、ProductAuthority、SessionReservation、Session、HostLease、ViewerGrant、MediaEpoch、ControlEpoch、HostConsent、RelayAuthority、AuthorityHead 与完整 AuthorityRecord；对象携带的摘要不得作为计算输入或信任来源；
- 一个 `InteractiveDesktopAuthorityRecord` 原子封装同一 Session revision 的完整授权对象，跨对象 ID、digest、generation、sequence、fencing、Consumer、Provider、权限和时间窗必须整体一致；
- Store 保存不可变 AuthorityRecord 版本和每个 Session 唯一当前头；提交在一个 `BEGIN IMMEDIATE` 事务中完成 exact replay、连续 revision、旧 revision/digest 三元组 CAS 与规范 readback；历史版本即使字节完全相同也不重新获得当前权限；
- 第一条解析分支只允许 `same_owner_remote_access`：同一事务内重取当前 Provider owner、V279 user-node 安装绑定、endpoint credential/session、共享 Reservation/Claim/Pool 与 Consumer 登录 session，并要求 Consumer 等于 Provider owner；Claim 的四个交互式 meter、数量、bucket 与 Pool policy 必须精确相等；
- `friend_co_play` 在精确 Session 邀请 Store 完成前失败关闭；`licensed_cloud_seat` 在 entitlement、TitlePolicy、商业串流权利与支付授权 Store 完成前失败关闭；
- Store 产生的当前权威类型必须不可序列化、不可由普通结构体构造，并受事务生命周期约束；原有 `structurally_authorizes` 继续只是内部结构谓词，不能成为运行时旁路。

第二批只写源码和迁移定义：不执行迁移，不接公开 API/Broker/节点运行时，不创建 WebRTC、TURN、捕获、编码、输入或计费效果，也不把 endpoint 在线等同于 Interactive Ready。当前源码显式拒绝产生活动权威，因为 InteractiveOfferProfile、云电脑专用本机 HostConsent、Viewer 握手/一次性 ticket 与 TURN RelayAuthority 的独立 Store 尚不存在；这些来源不能由 AuthorityRecord 自证。Viewer device/transport 摘要接口已要求传入外部观测值，但在专用握手 Store 完成前仍由不可用来源门卫拒绝。非活动 revision 只允许冻结全部权威对象后推进 Session 状态，以便未来即使活动来源失效仍可进入 reconnecting/ending/terminal，而不会继续授予媒体或输入权限。

## 验收标准

1. 文档明确“一套联邦控制面、两个执行平面”，并给出共享和独立合同的精确边界。
2. 新 Rust 合同使用独立 `interactive_desktop.v1` schema、`service_class` 和摘要域，不修改或放宽现有批处理 Workload 闭集。
3. Session 合同同时绑定 Provider、Consumer、精确 Offer/PriceSnapshot/Reservation/Claim、HostLease fencing、ViewerGrant、MediaEpoch 和 ControlEpoch。
4. 权限合同默认拒绝控制、音频、剪贴板、文件、麦克风、摄像头和提权；首期仅允许显式选择的画面及可选键鼠。
5. 计量合同区分 declared/transport_observed/consumer_observed/verified，使用累计整数并记录 direct/TURN 路径，不保存媒体或输入正文。
6. 合同源码明确旧 MediaEpoch、旧 ControlEpoch、过期 Grant、过期 HostLease 和非当前 fencing generation 不具备权限。
7. 现有 `llm_chat`、图像/视频生成、GPU batch、旧节点共享和批处理结算合同保持不变。
8. 生产、媒体、输入和资金效果均保持关闭，未编译/未运行状态被明确记录。
9. 所有交互式权威对象与总 AuthorityRecord 都从去除自身摘要字段后的规范投影重算摘要；调用方自报摘要不能单独通过当前性校验。
10. 每个 Session 只有一个 Store 当前头，版本不可变且 revision 连续；exact historical replay、旧 head CAS、混合 Lease/Grant/Media/Control 代次均失败关闭。
11. `same_owner_remote_access` 必须在同一 Store 事务中证明当前 Provider/V279 binding/endpoint 与 Consumer 登录 session 属于同一账号。
12. 好友邀请与陌生人付费模式在各自权威 Store 缺失时不可通过；结构正确的 invitation、entitlement 或 TitlePolicy 摘要不是当前授权。
13. C1 结果只登记为源码级、未编译、未运行、迁移未执行；不得据此开启 Session、媒体、输入、计费或 Provider 收益。

## C1 静态结论与显式阻断

- 已写入：对象专属规范摘要、原子 AuthorityRecord、不可变版本/单 Session 当前头、三元 CAS、SameOwner 可用来源重取、共享 Claim 四 meter 精确核对、外部 Viewer 摘要比较，以及不授予 action 的 reconnecting/ending/terminal revision 规则。
- 仍失败关闭：InteractiveOfferProfile 权威版本、云电脑专用本机同意及撤销头、Viewer 设备签名/一次性 ticket/transport 握手、TURN allocation/grant 当前来源，以及区分 owner/consumer/expiry-reconciler 的 typed termination authority。任何一项活动来源缺失时，活动 AuthorityRecord 不可提交或读取为当前许可；终止源码只验证冻结 successor，目前仍保持 Store-private，不能直接接 route。
- Consumer `account_auth_epoch` 在 C1 是登录 session 规范摘要导出的非零 I-JSON 安全代次，不冒充账号级全局安全 epoch；密码修改、恢复与风险处置仍依赖现有 session 撤销语义。后续若引入账号级 epoch，必须作为新的独立真源迁移。
- 本批没有编译、测试、执行 V283 或打开 SQLite，所有结论仅为源码静态审查；测试计数固定为 `passed=0, failed=0`。

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
