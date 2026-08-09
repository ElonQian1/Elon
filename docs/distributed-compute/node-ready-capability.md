---
title: 节点 ReadyCapability 健康证明边界
status: current
reviewed_at: 2026-08-10
owners: node, compute
---

# 节点 ReadyCapability 健康证明边界

## 1. 目标

`ReadyCapability` 只表示某个节点插件在一个很短的时间窗口内具备技术执行条件。它不是安装完成标记、市场报价、可预留容量、账户授权或商业 `ComputeOffer`。

当前代码状态为“v216 增量已编译并完成 schema/VFS 定向回归、生产路径未接线”：staged 候选健康与 quarantine/cleanup 基线之后，源码又增加同时消费 retained content、staging 与未过期健康回执的本机 installed/promotion 双回执闭包。它仍不启动 Sidecar、不声明 runtime ready，也不生成 work-admission、`ReadyCapability`、Offer 或 Attempt 权威；真实生产数据库、NodeRuntime/Host 接线及控制面上报仍未完成。

## 2. 已关闭的错误入口

旧接口由调用方传入 `health_is_fresh: bool`，本机任意代码都可能在没有可信时钟证明的情况下声称健康记录仍然有效。该入口已经删除。

新入口必须同时取得：

- 完整 `ComputePluginInventorySnapshot`，而不是拆散的共享开关和插件记录；
- 规范安装身份，用于阻止另一安装实例的时间证明被重放；
- 经挑战、签名和安装身份绑定后产生的 `ComputePluginTrustedTimeObservation`；
- 要发布技术就绪事实的精确 `plugin_id`。

## 3. 验证闭包

入口首先复用库存整体校验，再要求共享已开启、插件期望存在且启用、准入状态为 allowed、活动槽为 installed、运行态为 ready，并核对正数的安装、激活和运行代次。

健康记录必须精确绑定当前活动槽、Runtime generation 和 Runner SHA-256；权限授权、Manifest、package 和 Runner 摘要都必须是规范 SHA-256。健康观察时间不得早于当前记录状态变更时间，不得晚于可信现在，失效时间必须严格晚于可信现在，且单次健康有效期最多五分钟。

`observation_digest` 使用 JCS/SHA-256 覆盖插件 ID、安装与激活代次、健康状态、运行代次、槽、Runner、规范排序的原因码及观察/失效时间。调用方不能只替换 `expires_at` 或 Runner 后继续复用旧摘要。

## 4. 线性输出

校验成功只产生不可 `Clone`、不可序列化、字段私有的 `ValidatedComputeReadyPublication`。该对象封存已经核对的插件记录、库存和策略修订号，并拥有原始可信时间观察；普通 DTO、墙钟时间或裸布尔值不能构造它。

后续短 TTL capability 构建器必须消费该对象，并在本地权威 Store 中重新核对库存 revision、authority epoch、共享状态和活动运行代次。Store 变化、排水、停用、撤销、重启或健康过期均应阻止发布或使既有技术就绪事实失效。

## 5. 候选预热评估合同

staged 候选现可进入独立的进程内健康评估状态机。入口消费 `StagedComputePluginCandidateArchive`，把原文件句柄保管权与签名 Manifest、安装身份、staging receipt、候选 token、staging run、release、可执行入口和 Runner SHA-256 固定在同一个不可 Clone、不可序列化的对象内。入口不接受普通路径、裸候选 ID 或调用方拼装的 staged 布尔值。

每次 Host 侧探针观察必须提供成功或失败终态、耗时、响应摘要和失败原因码。状态机限制单次超时、探针总数、原因码数量以及 Manifest 声明的连续成功/失败阈值，并用前一摘要、严格序号和本次观察形成追加式 JCS/SHA-256 transcript。达到失败阈值后不能继续伪造成功；达到连续成功阈值后也不能继续改写历史。

健康评估只有在 authenticated trusted-time observation 的单调时间严格晚于最后一次探针，且安装身份和取消门卫仍然有效时才能封存。输出绑定完整探针摘要、staging receipt、release、Runner、健康参数、规范原因码、可信时间证明和最长五分钟有效期，并继续持有原 staged custody。`ValidatedCandidateHealthPublication` 是健康 Store 事务的线性输入，本身仍不是耐久健康回执、installed 状态、promotion permit、ReadyCapability 或商业算力证明。

当前 `CandidateHealthProbeObservation` 仍是 Host 进程内部观察，不证明真实 Sidecar 已经运行，也不替代尚未实现的认证 IPC、进程隔离、响应大小限制和探针调度器。调用方不能据此宣称插件已经可执行。

## 6. 候选健康回执 Store

存储前必须进行无副作用的 fresh authority read，并精确绑定当前 staged 候选、staging receipt、inventory revision/digest、authority epoch、process owner epoch 与 trusted-time high-water。只有这些事实未漂移时，Store 才能在同一 `BEGIN IMMEDIATE` 事务中写入不可变 `candidate_health_receipts` 并推进可信时间高水位。schema trigger 禁止回执更新或删除，并拒绝同一候选上有效期重叠的健康回执。

Store 返回成功后产生继续持有 staged 文件句柄的 `DurableCandidateHealthPublication`。如果写入结果不确定，调用方只能使用进程内 recovery key 读出稳定的 `NotCreated` 或 exact `Recorded`；身份碰撞、fence 漂移或时间回退均失败关闭。exact `Recorded` adoption 会再做 fresh read，并从保留句柄重新哈希 staging 文件和 seal；`NotCreated` 只是结果证明，不恢复写入许可。该耐久结果仍不会修改槽位、安装或激活代次，也不会生成 `ReadyCapability` 或商业回执。

## 7. 终止失败与 quarantine Store

达到 Manifest 声明的连续失败阈值后，评估不能再走健康成功封存入口。独立失败入口要求终止不健康状态、足量连续失败、至少一个规范原因码、完整 transcript、严格晚于最后探针和 staging 的 authenticated trusted time，并继续检查安装身份和取消门卫。输出为不可 Clone、不可序列化且继续持有 staged 文件句柄的 `ValidatedCandidateHealthFailurePublication`；它不是调用方可拼装的失败布尔值。

quarantine 授权先做无副作用 fresh authority read，精确核对 staged 槽、staging receipt、candidate owner、无未过期健康回执、inventory/state/authority/process fence 与 trusted-time high-water。Store 在单一 `BEGIN IMMEDIATE` 中推进可信时间，把槽从 `staged` 改为 `failed`，令 state、inventory 与 authority fence 各精确加一，并插入不可更新、不可删除的 `candidate_health_quarantine_receipts`。它保留 candidate owner、candidate pointer 和原 staged 文件 custody，不删除目录，也不授予下载、重试、安装或推广。

提交结果不确定时，进程内 recovery key 只能得到稳定 `NotCreated` 或 exact `Quarantined`。前者不恢复写许可；后者必须 fresh read 精确的 failed inventory/fence，再从保留句柄重新哈希 staging 文件和 seal，才可恢复 `DurableCandidateHealthQuarantine`。底层同句柄删除、cleanup authorization、旧线性物理执行器及 completion Store 内核已形成私有代码；completion 还必须消费 sealed topology 和全部 namespace-durability journal 的不透明终态能力，而该能力的 topology/journal Store 与目录 durability 生产者尚未实现。只有 durable completion receipt 才能证明 owner/pointer/inventory 已原子释放；整条流水线尚不可达且未接入生产 Host，quarantine、cleanup authorization 或内存物理证据都不能替代完成回执。精确边界见 `node-plugin-candidate-cleanup.md`。

## 8. Installed/promotion 双回执边界

v216 把内容安装与 active 指针提升固定为同一个本机 `BEGIN IMMEDIATE` 事务。入口只能消费继续持有原 staging 文件和 seal 句柄的 `DurableCandidateHealthPublication`；在新的 authenticated trusted-time observation 之后重新校验 retained content，再 fresh-read candidate owner、staged 槽、staging/health 回执、签名 Manifest、permission grant、inventory/state/authority/process fence 和健康 TTL。普通路径、裸摘要、调用方布尔值或已析构的健康 DTO 均不能构造 promotion permit。

事务写入互相精确引用且不可单独提交的 `candidate_install_receipts` 与 `candidate_promotion_receipts`，随后一次性把槽从 `staged` 推进为 `installed`、清除 candidate pointer、切换 active slot、推进 install/activation 与 authority/inventory fence，并把 candidate owner 从 `owned` 终结为 `promoted`。旧 active provenance 在升级时由前一对双回执精确承接；首次安装则保持完整空组。结果继续持有原受管文件句柄，提交不确定时只能通过 exact recovery 判定 `NotCreated` 或已存在的双回执，再经 fresh head 与同句柄重哈希恢复 custody。

promotion 刻意保持 runtime 为 `stopped`、runtime generation 不变、active health 为空。候选预热健康只证明安装前内容检查，不会被复制成活动 Runtime 健康；双回执也不替代 work-admission receipt。因此 v11 Planning Snapshot、ReadyCapability V2 与商业调度仍继续失败关闭。本批已随 `elon-pc-node` 编译，v7 authority 全新安装、重开及 v3-v6 原子迁移等 11 项测试和 69 项 SQLite VFS 回归通过；尚未执行生产磁盘迁移或 install/promotion 完整事务夹具。

## 9. Work-admission 仍不是 Ready

v217 源码新增 `reauthorize_existing`：Control-signed InstallPlan 只能精确引用当前 active installed release、当前 install generation 和一个新的 grant，不得携带 candidate、target release 或下载；Plan admission 会重新验证该 release 的 Publisher-signed Manifest、Host API/target 兼容性、grant 子集和 JCS 摘要。PlanApply 只把当前记录投影为 `present/enabled/allowed` 并更新 grant 与计划引用，不改变 install、activation、runtime generation、active slot、健康或 active attempts。

新的线性 work-admission 合同先消费 `DurableInstalledPluginSlot`，在旧 candidate source 尚可验证时对 retained handles 做全量重哈希；随后取得严格晚于该重哈希 barrier 且仍 live 的 authenticated trusted-time observation P，并用同一个 P 与当前 process fence 应用签名 `reauthorize_existing`。只有 fresh `Applied` 结果会封存私有 PlanApply commit barrier，历史 `Replayed` 或其他 action 都不会获得它；再取得 monotonic 严格晚于 commit、可信时间也严格晚于 P 的 observation S 后，才可绑定 local authority session。v8 本地账本以不可变 source/receipt 和单调 current head 封存 v7 install/promotion 双回执、sealed reauthorization application、签名 Plan/Manifest、policy/catalog/keyring fence、完整 launch profile、授权的 CPU/内存/显存/磁盘/进程/Sidecar 时长及权限上限；单一事务只推进 work-admission head、authority state/epoch 与可信时间，inventory 和 runtime 事实保持不变。提交不确定只能 exact recovery；只有 current exact receipt 且 retained content 再次 fresh rehash 成功，才恢复 `DurableWorkAdmittedPluginSlot`。

这些数值是签名 grant 的授权上限，不是测得容量、设备分配、OS 限制或 v212 `ResourceCeiling`。合同不生成 accelerator count、输出上限、并发上限、runtime/model 摘要或 enforcement receipt，也不启动 Sidecar、不复制 candidate health、不构造 Ready。CPU-only 节点可诚实保留 VRAM=0；服务端现有 accelerator>0 合同必须在后续显式解决，不能用虚构 accelerator 填平。v217 增量同样尚未编译、迁移、运行或接入生产 Host。

## 10. 尚未实现

- Sidecar 启动、预热和动态健康探针；
- Sidecar 到 Host 的认证 IPC、响应验证和真实探针调度；
- 失败候选的清理完成 Store、目录耐久闭包、重试授权与跨重启 custody 治理；
- Host enforcement profile、Runtime 启动/激活和活动健康回执；
- 发布 Ready 前的 fresh Store read、CAS/fencing 和主动失效链；
- `ComputeReadyCapability` 的规范短 TTL 构建、认证上报和服务端验证；
- 共享关闭、排水、崩溃和撤销后的主动失效通知；
- 从技术就绪事实到 Provider、CapacityPool、Offer 和 Attempt 的生产接线。

在上述链路完成前，代码中存在健康证明类型不代表节点已经可被消费者 AI 调度或产生算力收入。
