---
title: 分布式算力节点客户端与按需插件
status: current
reviewed_at: 2026-08-04
owners: node, pc
---

# 分布式算力节点客户端与按需插件

## 1. 产品原则

普通用户安装一龙客户端时，只携带轻量 `Compute Bootstrap`。共享算力默认关闭；关闭时不启动算力守护进程，也不新增下载插件包、CUDA/DirectML 运行时或模型工件。已缓存内容是否保留由用户的暂停、清缓存或完全移除选择决定。

用户主动开启“贡献闲置算力”后，客户端才探测硬件、展示预计任务和收益、生成安装计划，并在用户确认后下载所需 Node Core、插件、运行时和模型工件。

## 2. 组件分层

| 层 | 默认携带 | 职责 |
|---|---|---|
| Compute Bootstrap | 是 | 开关、硬件探测、磁盘预算、签名下载器、恢复与卸载 |
| Node Core | 否 | 与控制面维持会话、策略同步、任务队列、租约、审计事件 |
| Plugin Host | 否 | 统一启动/停止插件，限制资源并转换通用任务合同 |
| Runtime Plugin | 否 | Ollama、LM Studio、llama.cpp、CUDA、DirectML 等适配 |
| Model Artifact | 否 | 内容寻址的模型、Tokenizer、LoRA 和辅助工件 |

现有 Windows PC 节点可以复用身份、升级器和 WebSocket 会话，但“远程开发节点”与“贡献算力节点”仍是两个独立权限。

## 3. 目标开启流程

1. 用户打开贡献开关；Bootstrap 只读探测 CPU、GPU、显存、内存、磁盘、驱动和现有本地模型服务。
2. 控制面根据硬件、用户允许的任务种类和磁盘预算生成带授权、库存与策略修订的 signed `InstallPlan`。
3. 客户端展示下载体积、预期磁盘、请求与实际授予的权限、运行时和资源上限，用户确认后才可执行。
4. 执行器在每次取回工件前重新核对共享授权；共享关闭时计划只能保留、停用或移除，下载列表和下载字节必须为零。
5. 下载器校验 signed Manifest、包和文件摘要、精确目标平台、Host API 与 Sidecar 约束，再把候选版本写入独立槽。
6. Plugin Host 对候选槽启动 Sidecar、完成健康检查和预热，成功后生成绑定安装、激活和运行代次的短 TTL `ReadyCapability`。
7. 控制面可以结合新鲜 `ReadyCapability`、Provider 策略和市场条款创建商业 Offer；节点客户端不直接发布 Offer。

“开关打开”不等于“立刻可调度”；只有插件和工件验证完成、策略允许且健康检查通过，节点才上报短期技术就绪事实。Offer、价格、可预留容量和消费者授权始终属于服务端商业域。

## 4. 关闭与卸载

关闭共享时先进入 `draining`，停止领取新任务，允许在资源上限内完成或检查点化已有任务；到达截止时间后取消剩余 Attempt、释放 Reservation 并停止 Node Core。

用户可以分别选择：

- 仅暂停：保留插件和模型，后续快速恢复；
- 停止并清缓存：删除可重下的模型和运行时缓存；
- 完全移除：删除 Node Core、插件、模型和算力专用状态，但保留必要的结算回执与用户账户记录。

## 5. 插件生命周期

```text
install:    absent -> downloading -> verifying -> staged -> installed -> removing -> absent
activation: disabled <-> enabled
admission:  allowed | quarantined | revoked
runtime:    stopped -> starting -> ready -> draining -> stopped
                         \-> crashed
```

- 安装、启用意图、准入信任和运行状态是四个正交事实；`planned` 属于安装计划，`ready` 不是安装态或商业 Offer；
- 每个插件记录独立的 `active` 与 `candidate` 槽。旧版本继续服务时，新版本只能在候选槽下载、验证和预热；切换必须增加激活代次并原子更新活动槽；
- `ready` 必须绑定活动槽、插件与 Runner 摘要、运行代次、健康观察摘要、模型摘要和硬件档案，并在 TTL 到期、重启、排水、停用或撤销时失效；
- `quarantined` 和 `revoked` 禁止生成 `ReadyCapability`；回滚复用仍然完整且已验证的旧槽，不能原地覆盖执行中的版本。

## 6. 插件清单

正式发布物使用 signed Manifest：签名位于规范化 Manifest 载荷之外，摘要只覆盖载荷，避免自签名循环。每份 Manifest 对应一个精确目标平台，至少包含：

- `plugin_id`、插件版本、规范目标标识、Manifest 与包摘要；
- Publisher、签名算法和签名 key 引用，以及规范化与摘要算法；
- 支持的 `task_kinds`、Sidecar 协议和 Host API 修订区间；
- 操作系统、架构、GPU 后端、ABI、驱动及内容寻址的系统/下载依赖；
- 无 shell 的入口进程、健康检查、请求的资源上限和逻辑权限范围；
- 可选状态兼容声明。保留旧版本数量、排水、权限授予和回滚动作由 signed `InstallPlan` 决定。

Manifest 只能声明请求，不能授予权限、证明本机已安装或健康，也不能携带下载凭据、本机路径、价格或 Offer。插件能力必须由实际健康检查生成，禁止根据“已安装”推断为“可执行”。

## 7. 本机 InstallPlan 准入合同

客户端必须在产生任何网络下载或安装副作用前完成一次失败关闭的本机准入，结果只是一份短期、可撤销的执行授权，不等于插件已经安装。准入固定以下边界：

1. Publisher Manifest 与控制面 InstallPlan 使用彼此独立的 Ed25519 keyring。Manifest 不能使用控制面 key，InstallPlan 也不能借用 Publisher key；除按用途、状态和有效期解析 key 外，本机还拒绝本次 Manifest 与 InstallPlan 复用同一份原始公钥指纹。未知、重复、撤销、跨用途或格式错误的 key 一律拒绝。
2. 两类载荷都使用 RFC 8785 JCS 规范化并计算 SHA-256，对外摘要固定为 64 位小写十六进制。签名消息固定为 `DOMAIN || 0x00 || digest-raw-32-bytes`，不能把十六进制文本当作签名字节；签名字段永远位于被签载荷之外。签名载荷中的整数限制在 I-JSON/IEEE-754 安全整数范围，超出时改用版本化十进制字符串字段，不能让不同语言各自舍入。
3. signed Manifest 必须精确绑定插件、版本、目标、包摘要、Host API、无 shell Sidecar 入口、文件清单、依赖和请求权限。相对路径必须规范且留在插件槽内；包、文件与下载依赖必须形成无重复、无额外工件的闭包。
4. InstallPlan 必须同时绑定当前库存 revision 与 JCS SHA-256 摘要、策略 revision、共享授权 revision/digest、节点硬件档案摘要、Manifest catalog revision、Publisher/Control 两类 keyring 的 revision/digest 和有效期。计划寿命最多 24 小时；首次准入只容忍可信时间相对 `generated_at` 最多提前 5 分钟，真实取数不得早于 `generated_at`。任何事实漂移都要求重新取计划，客户端不得猜测或自动放宽。
5. 本地 grant 只能是 Manifest 请求权限与资源上限的严格子集。计划签名不能把 Publisher 未请求的网络、文件、设备、子进程、CPU、内存、显存、磁盘或运行时长权限变成合法授权。
6. 每个 install/upgrade 项必须绑定一份已验签且目标精确匹配的 Manifest；插件包工件 ID 固定为 `sha256:<package_digest>`，依赖使用 Manifest 的显式 ID，全部摘要、长度和顺序必须形成无重复、无额外工件的精确闭包。`source_ref` 只是有界 ASCII 查找引用，不能是 URL、本机路径、token 或签名下载地址。计划声明的磁盘需求不得低于下载压缩字节与包解压字节之和。
7. 首次准入只冻结“允许应用这份计划”的事实。真实取数使用 `begin_claim -> fsync .part -> commit_segment` 三段式事务，在认领和提交时都重新读取当前共享开关、授权 binding、策略 revision、节点档案、catalog、两类 keyring binding/签名 key、应用计划、候选 owner 和有效期；offset 必须等于耐久 committed cursor，claim 携带 authority epoch 与 generation fence。单段最多 16 MiB，range 不得越过工件长度，redirect 严格递增且不推进游标，可信时间持久保持单调高水位。下载循环还在每次 socket read/write buffer 前检查 cancellation epoch，使任一变化在下一字节前停止。
8. 共享关闭时，计划与本机重读都必须同时证明关闭；总下载字节、所需新增磁盘和所有下载列表精确为零，只允许 keep/disable/remove/cancel_candidate，应用后不得留下仍期望启用的插件。`cancel_candidate` 精确绑定候选 release、活动 release（可为空）和安装代次，用于清理首次安装或升级中断后的候选槽，不能借机下载或激活新版本。
9. 准入与计划应用必须分离。独立 SQLite 权威库在单一 `BEGIN IMMEDIATE` 内以 `expected_inventory_revision` CAS，同事务保存不可变应用、候选所有权、下载游标和库存；提交顺序固定为“克隆旧状态、全量校验、原子提交成功、最后发布内存”。写入失败保留旧数据库和旧内存，不启动下载。
10. 候选授权不能依赖审计字段 `last_plan_id`。candidate 必须绑定本机随机 token、owner plan ID/digest、应用 revision 和 generation；每个未完成槽恰好由 candidate 指针引用，普通新计划不能接管，取消也必须精确绑定所有权。
11. 重启恢复必须持久化计划 ID/digest、已应用 inventory revision、候选槽归属、三段式 claim/cursor、authority/owner epoch 和可信时间高水位；恢复时重新验签并重读授权，不能因进程曾经准入就跳过撤销、过期或 keyring 变化。恢复器只能继续仍被当前计划拥有的候选，或等待显式 `cancel_candidate` 清理。完整权威合同见 `docs/distributed-compute/node-plugin-local-authority.md`。

准入输出只包含已验证的不可变绑定、稳定原因码和按计划顺序排列的下载描述，不包含下载凭据。真实下载器仍须自行执行 HTTPS/重定向边界、分块摘要、临时文件、包解压防穿越、文件摘要和槽提交门禁。

## 8. 第一批插件路线

1. `bridge.ollama.v1`：复用用户已经运行的 Ollama；
2. `bridge.lmstudio.v1`：复用 LM Studio OpenAI 兼容端点；
3. `runner.llama-cpp.cpu.v1`：通用 CPU 低门槛供给；
4. `runner.llama-cpp.cuda12.v1`：NVIDIA GPU；
5. `runner.llama-cpp.directml.v1`：Windows 多品牌 GPU。

首批代码已经把现有 `LlmStreamRequest` 经 NodeAgent 内部 Host seam 委托给旧本地推理实现，输出仍由原实现产生 Chunk/End/Error。内部登记使用稳定 `runner_id` 和本机 revision，不序列化成 Plugin Manifest，也不发布新 capability。

本批又写入了 release identity、signed Manifest、满足共享关闭零下载约束的 signed `InstallPlan`、active/candidate 槽、本机正交生命周期、hashed 短 TTL `ReadyCapability`、Attempt `Start/RenewLease/Cancel` 命令和 typed Runner/Host 事件合同。Manifest/InstallPlan 已具备 RFC 8785 JCS、SHA-256、Ed25519 验签、Manifest 语义校验及失败关闭的本机计划准入内核；这些代码仍未编译、未接线，也没有产生下载或安装副作用，不能据此宣称插件系统已经可下载安装或可调度。

## 9. Host 与 Sidecar 边界

插件目标形态是独立 Sidecar 进程，通过 Windows Named Pipe 或等价本机 IPC 与 Plugin Host 通信，不暴露公网监听端口。Host 负责：

- 验证任务合同和工件摘要；
- 施加 CPU、内存、显存、磁盘、时长和并发限制；
- 转发取消、心跳、进度、检查点和终态；
- 在进程退出或租约过期时生成失败事实；
- 只允许最新 `fencing_generation` 提交候选终态；秘密租约凭据与代次字段分开处理。

本批合同把 Attempt 控制固定为 `Start`、`RenewLease` 和 `Cancel`，秘密租约引用只留在 Host，不转发给 Runner。Runner 只产生 typed `Started/Heartbeat/Progress/StreamChunk/CheckpointReady/UsageSnapshot/Terminal` 事件；Host 校验大小、顺序和摘要，补写 Attempt 身份与 fencing generation，再转换成 Host 侧 typed 事件。云端 v188 已写入累计 `provider_declared` 快照 Store/HTTP，但尚未与本 Host 事件接线。Runner 自报用量始终只是声明，终态事件也不是执行回执或结算决定。

第一批内部兼容 seam 仍直接调用现有 Rust 本地推理函数。本机权威库已确定为独立 SQLite，并固定了 root-signed 双 keyring、库存/计划/候选原子事务和三段式下载认领合同；keyring DTO、Bootstrap root resolver seam、整包校验和按两类精确 binding/可信时间过滤的不可变快照 resolver 已形成代码，InstallPlan 与 live state 也同时绑定两类 ring revision/digest，但没有生产 root pin、耐久 store、防回滚或运行接线。真实下载器、原子槽切换、Sidecar 进程、IPC framing、资源沙箱、事件背压、`ReadyCapability` 上报与真实 Attempt 运行接线仍未实现；旧式单调用取数权威 trait 也还不能被真实下载器使用。

## 10. 模型与缓存

模型采用内容寻址标识，至少绑定 SHA-256、文件长度、格式、Tokenizer 摘要、许可元数据和来源。下载支持断点续传、分块校验、镜像切换和去重。

缓存分为 pinned、active、warm 和 evictable。正在运行或已承诺交付窗口的工件不能被回收；其余根据用户磁盘配额和最近使用顺序回收。插件升级不能隐式下载任意模型，模型计划必须单独可见。

## 11. 策略与资源保护

控制面下发带 `policy_revision` 的期望策略，节点返回 `applied_revision` 和本机有效策略。每次任务开始前仍需本地重新检查：

- 用户开关、允许任务和允许模型；
- 使用时段、最大功耗、温度、CPU/GPU/内存占用；
- 最大并发、每日/周期容量和网络上行；
- 前台交互优先级、游戏/会议/电池状态；
- 插件、运行时与模型的实际健康状态。

本地策略比云端期望更严格时，以本地为准。节点拒绝任务必须返回稳定原因码，使 Broker 能立即改派。

## 12. 任务与数据

V1 只运行平台批准的任务种类和签名 Runner，不接受请求方上传任意可执行代码。任务输入使用摘要和短期引用；日志默认不复制完整 Prompt 或模型输出。技术信任通过验证策略逐步提高，而不是把节点自报结果直接当成结算事实。

## 13. 当前未验证声明

本文是已接受的客户端目标设计。NodeAgent 内部 legacy Host seam、版本化合同、签名与摘要验证器、Manifest 语义校验、InstallPlan 双 keyring binding 准入以及 root-signed keyring bundle/快照 resolver 已形成代码，但本批尚未编译或启动。SQLite 本机权威库、生产 root pin、防回滚 keyring 安装、计划应用与恢复事务、三段式取数权威、真实 downloader、Sidecar/IPC/沙箱、typed 事件运行链路、短 TTL `ReadyCapability` 上报和调度接线仍未实现；节点不直接发布商业 Offer，服务端 Offer 生成也未因本批合同自动接通。
