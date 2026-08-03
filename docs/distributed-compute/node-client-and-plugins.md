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

## 7. 第一批插件路线

1. `bridge.ollama.v1`：复用用户已经运行的 Ollama；
2. `bridge.lmstudio.v1`：复用 LM Studio OpenAI 兼容端点；
3. `runner.llama-cpp.cpu.v1`：通用 CPU 低门槛供给；
4. `runner.llama-cpp.cuda12.v1`：NVIDIA GPU；
5. `runner.llama-cpp.directml.v1`：Windows 多品牌 GPU。

首批代码已经把现有 `LlmStreamRequest` 经 NodeAgent 内部 Host seam 委托给旧本地推理实现，输出仍由原实现产生 Chunk/End/Error。内部登记使用稳定 `runner_id` 和本机 revision，不序列化成 Plugin Manifest，也不发布新 capability。

本批又写入了 release identity、signed Manifest、满足共享关闭零下载约束的 signed `InstallPlan`、active/candidate 槽、本机正交生命周期、hashed 短 TTL `ReadyCapability`、Attempt `Start/RenewLease/Cancel` 命令和 typed Runner/Host 事件合同。它们均是尚未编译、尚未接线的 Rust 合同，不能据此宣称插件系统已经可下载安装或可调度。

## 8. Host 与 Sidecar 边界

插件目标形态是独立 Sidecar 进程，通过 Windows Named Pipe 或等价本机 IPC 与 Plugin Host 通信，不暴露公网监听端口。Host 负责：

- 验证任务合同和工件摘要；
- 施加 CPU、内存、显存、磁盘、时长和并发限制；
- 转发取消、心跳、进度、检查点和终态；
- 在进程退出或租约过期时生成失败事实；
- 只允许最新 `fencing_generation` 提交候选终态；秘密租约凭据与代次字段分开处理。

本批合同把 Attempt 控制固定为 `Start`、`RenewLease` 和 `Cancel`，秘密租约引用只留在 Host，不转发给 Runner。Runner 只产生 typed `Started/Heartbeat/Progress/StreamChunk/CheckpointReady/UsageSnapshot/Terminal` 事件；Host 校验大小、顺序和摘要，补写 Attempt 身份与 fencing generation，再转换成 Host 侧 typed 事件。Runner 自报用量只是声明，终态事件也不是执行回执或结算决定。

第一批内部兼容 seam 仍直接调用现有 Rust 本地推理函数。下载器、Manifest/InstallPlan 验证器、原子槽切换、Sidecar 进程、IPC framing、资源沙箱、事件背压、`ReadyCapability` 上报与真实 Attempt 运行接线仍未实现。

## 9. 模型与缓存

模型采用内容寻址标识，至少绑定 SHA-256、文件长度、格式、Tokenizer 摘要、许可元数据和来源。下载支持断点续传、分块校验、镜像切换和去重。

缓存分为 pinned、active、warm 和 evictable。正在运行或已承诺交付窗口的工件不能被回收；其余根据用户磁盘配额和最近使用顺序回收。插件升级不能隐式下载任意模型，模型计划必须单独可见。

## 10. 策略与资源保护

控制面下发带 `policy_revision` 的期望策略，节点返回 `applied_revision` 和本机有效策略。每次任务开始前仍需本地重新检查：

- 用户开关、允许任务和允许模型；
- 使用时段、最大功耗、温度、CPU/GPU/内存占用；
- 最大并发、每日/周期容量和网络上行；
- 前台交互优先级、游戏/会议/电池状态；
- 插件、运行时与模型的实际健康状态。

本地策略比云端期望更严格时，以本地为准。节点拒绝任务必须返回稳定原因码，使 Broker 能立即改派。

## 11. 任务与数据

V1 只运行平台批准的任务种类和签名 Runner，不接受请求方上传任意可执行代码。任务输入使用摘要和短期引用；日志默认不复制完整 Prompt 或模型输出。技术信任通过验证策略逐步提高，而不是把节点自报结果直接当成结算事实。

## 12. 当前未验证声明

本文是已接受的客户端目标设计。NodeAgent 内部 legacy Host seam 与上述版本化合同已形成代码，但本批尚未编译或启动。真实 downloader、签名与摘要验证器、安装事务、Sidecar/IPC/沙箱、typed 事件运行链路、短 TTL `ReadyCapability` 上报和调度接线仍未实现；节点不直接发布商业 Offer，服务端 Offer 生成也未因本批合同自动接通。
