---
title: 一龙任务级分布式算力联邦
status: current
reviewed_at: 2026-08-04
owners: backend, node, ai-economy
---

# 一龙任务级分布式算力联邦

本目录是“一龙成为 AI 算力矿池与联邦入口”的权威设计入口。目标不是只共享一个本地模型的推理接口，而是把用户闲置节点、平台集群和外部算力池统一成可发现、可报价、可预留、可执行、可验证、可结算的任务级算力网络。

## 北极星

- 一龙是需求聚合器、调度与验证网络、算力市场及统一结算层。
- 用户节点是自主供给方；只有主动开启共享后才下载节点核心、运行时插件和模型工件。
- 外部公司、云 GPU 和其他矿池通过 Provider Adapter 接入，对上暴露统一 Offer、Lease 和 Receipt。
- 用户下载的开源本地模型既可以在自己的设备运行，也可以把可拆分任务调度到联邦节点执行。
- 公网异构节点优先聚合“任务”，不假装组成一张低延迟虚拟 GPU；真正的张量并行或流水线并行由受管集群内部完成，并作为一个逻辑 Provider 接入。
- 供需双方先使用不可变的期货/远期价格快照结算，随后演进到标准化容量合约、订单簿、持仓和清算。

## 当前事实

| 能力 | 2026-08-04 状态 |
|---|---|
| 节点模型白名单、最大并发、每日 Token 预算与执行租约 | 已实现，是兼容供给入口 |
| Provider / Offer / Job / Reservation / Lease / Receipt 统一领域合同 | 基础代码、v169 Provider、v170 Offer、v171 Price Snapshot 与 v172 Job Registry 已写，状态为 `implementation_uncompiled`，尚未迁移、接线和运行验证 |
| 节点插件治理合同 | Signed Manifest、InstallPlan、双槽安装/切换/回滚 lifecycle 与短期 ReadyCapability 合同已写，尚未编译或接线 |
| 通用 Attempt 执行合同 | Start / RenewLease / Cancel 命令、Runner typed events 与 Host 盖章事件合同已写，尚未编译或接入云端协议 |
| 节点按需插件下载与通用任务执行 | 旧 LLM 已接入内部 Host seam，尚未编译；真实下载器、Sidecar/IPC、动态健康上报、通用任务派发和协议接线仍未实现 |
| 共享 CapacityPool 与追加式容量账本 | 领域合同、v165-v168 schema、隔离 Store、Store-canonical Supply/Claim 请求摘要、只读审计、到期批处理、状态门卫和 epoch 轮换已写；Hold 已收紧窗口/TTL，Release/Expire 只接受 Claim 自有 held 容量；尚未编译、执行迁移或接线 |
| Provider 与 Offer 版本注册表 | v169/v170 schema、Provider/Offer 当前投影、追加式历史版本、规范摘要和容量引用审计已写；尚未编译、执行迁移或接线 |
| Price Snapshot 锁价注册表 | v171 schema、active Offer/单一窗口/双价格腿/费用/来源绑定、不可变登记读取、精确重放和历史审计已写；尚未编译、执行迁移或接入报价/Broker |
| ComputeJob 版本注册表 | v172 schema、Workload/范围/预算合同校验、当前投影、不可变历史、幂等、CAS、状态机和历史依赖审计已写；尚未编译、执行迁移或接入 HTTP/Reservation |
| 外部算力池适配器与统一报价 | 已接受设计，尚未实现 |
| 多源验证、期货曲线与真实结算 | 已接受设计，尚未实现 |
| 二级容量市场与自动清算 | 目标架构，尚未实现 |

“已接受设计”不等于“已上线”。任何代理都必须保留实现状态，不得把文档中的目标合同描述成当前生产能力。

## 阅读顺序

1. `docs/decisions/distributed-compute-federation-v1.md`：不可随意改变的架构决定。
2. `docs/distributed-compute/architecture.md`：Provider、控制面、数据面和任务状态。
3. `docs/distributed-compute/node-client-and-plugins.md`：客户端按需启用与插件边界。
4. `docs/decisions/distributed-compute-capacity-ledger-v1.md` 与 `docs/distributed-compute/capacity-ledger.md`：共享容量池、跨 Offer 防超卖和追加式容量账本。
5. `docs/distributed-compute/market-and-settlement.md`：标准化 SKU、期货锁价和结算回执。
6. 现有兼容实现：`docs/decisions/node-compute-sharing-supply-v1.md`。

## 分阶段落地

### F0：统一语言和合同

版本化的 Provider、Offer、Workload、Job、Reservation、Attempt Lease、Execution Receipt、Settlement Receipt 和 Price Snapshot 基础合同已经写入代码。节点侧还形成了带 `fencing_generation` 的 Start / RenewLease / Cancel Attempt 命令、Runner typed events 和 Host 盖章事件合同；这些代码均尚未编译、接线或运行验证。现有 `LlmStreamRequest` 继续工作，不在首批协议变更中制造强制升级。

### F1：用户节点成为可插拔 Provider

节点内部已经形成 Plugin Host 兼容 seam，以及 Signed Manifest、InstallPlan、双槽安装/切换/回滚 lifecycle 和 ReadyCapability 合同骨架；云端还形成 v169 版本化 Provider Registry、Offer 规范合同校验和 v170 追加式 Offer Registry。它们均尚未编译、执行迁移或接线。ReadyCapability 只是有明确过期时间的本机技术就绪证据，不包含市场价格、可预留容量或账户授权，**不等于 Compute Offer**；只有控制面结合 Provider、策略、容量和价格后才能登记版本化 Offer。

目标流程仍是：共享关闭时不下载重型组件；开启后按硬件和任务选择签名插件、运行时与模型工件。真实下载器、Sidecar 进程与 IPC、动态健康状态、云端 capability gate、通用 Attempt 协议接线和 Offer 发布目前都未实现。

### F2：Broker、验证和真实结算

共享 CapacityPool 与追加式容量账本已经形成领域合同、checked-i128 reducer、v165-v168 SQLite schema 和隔离的本地 Store。Store 可登记池版本与零余额 bucket，原子追加多 meter 发行/撤出双分录，并通过稳定 Claim 完成 hold、revision 栅栏释放和到期归还。Supply Add/Withdraw 与 Claim Hold/Finish 不再接收调用方摘要。Supply/Hold 摘要绑定完整资源、主体、排序 bucket 数量和规范 UTC；Finish 摘要绑定 claim ID、expected revision、终态 action 和发生时间，资源与主体经 claim ID 间接固定。幂等重放仍返回当前 Claim/余额，尚未保存不可变首次响应。Hold 必须显式到期、不允许在窗口结束后创建或晚于窗口结束；这些边界和 Expire 授权以 Store 当前记录时间为准，不信任调用方回填/未来时间。Release/Expire 只允许 `held -> available`，并用 checked `i128` 从 Claim 自己的 ledger legs 证明归属，不能释放 `active` Attempt 容量。Hold/Finish 的 causal binding 仍硬编码为空，standalone Reservation/Commitment 也尚未与预算、Price Snapshot 或真实 Reservation 原子绑定；不可变首次响应、业务绑定与事务内 Broker API 仍是待补缺口。只读审计、有界到期恢复、状态门卫、追加式生命周期和排空后的 epoch 轮换也已形成。v169/v170 又增加版本化 Provider 与 Offer 注册表：Provider 的当前投影和不可变历史分离；Offer 绑定发布时的 Provider 版本、SKU、容量池、窗口和 bucket，并验证静态容量上限不超过已发行容量，但不保存实时剩余量。v171 Price Snapshot Registry 固定当前 active Offer、单一交付窗口、双价格腿、费用和来源证据，可持久化完整快照，按快照 ID 精确重放，约束 quote ID 唯一，读取时复核历史 Offer，且数据库触发器拒绝更新和删除。上述代码尚未编译、执行迁移、调度或接入运行路由；Registry 接收已构造快照，不负责价格源、期货曲线、报价生成或 Broker 原子锁定，也不保存实时容量或移动资金。Offer 发布、复制或续期都不会铸造容量；只有 Pool bucket 的 `supply_added` 进入账本后才形成可用余额。V1 最终仍要求一份 Reservation 只绑定一个 Pool 和一个精确 UTC 半开窗口 `[starts_at, ends_at)`，并在同一事务中完成多 meter 容量、消费者预算、Price Snapshot 与 Reservation 的原子预留。

v172 ComputeJob Registry 已把需求身份、所选 Offer 历史版本、不可变 Price Snapshot、消费者预算上限和生命周期状态写入版本化 Store。新 Job 必须从 `submitted` 创建；进入 `quoted` 时只接受当前 active Offer、active Provider 与未过期快照，quoted 可显式刷新选择，进入 reserved 或后续状态后锁价合同不能更换。消费者幂等键、`expected_revision` 与 revision/digest CAS 防止重复或并发覆盖；当前和历史读取都会重新审计 Workload 合同及 Provider/Offer/Snapshot 依赖。该路径仍为 `implementation_uncompiled`，不代表预算已冻结、容量已预留或任务已派发。

后续实现可选择 Offer 查询与撮合、价格源/期货曲线和报价生成、容量自动调度与受控修复、容量/预算/Price Snapshot/Reservation 统一 Reserve、带 `fencing_generation` 的尝试租约、多源计量、挑战任务、争议状态和可提取收益账本。

### F3：外部矿池与企业集群

服务端 Provider Adapter 统一接入公司集群、云 GPU 和其他算力池；每个 Provider 保留自己的内部调度，只向一龙提交标准回执。

### F4：容量期货市场

以标准化 Compute SKU 和交付窗口发行容量合约，引入订单、持仓、指数价、标记价、保证资源和到期交割；任务结算消费已锁定的价格快照。

## 当前工程指令

产品负责人要求先把架构和代码骨架铺到主线，后置统一编译与真实运行验证。因此本阶段每个提交都必须明确标注“未编译/未运行”，只允许做格式化、静态审阅和仓库强制守卫；进入集成检查阶段后再一次性补齐编译、迁移、协议兼容和端到端验收。

## 代理接力规则

- 核心合同使用显式 `schema_version`，新增字段优先保持向后兼容。
- 金额、价格和用量禁止使用浮点数；统一使用整数微单位、基点或有理数比例。
- Offer 和 Price Snapshot 一经被 Job 引用便不可修改，只能创建新版本。
- 一次 Job 可以有多个 Attempt，但任何时刻只有拥有最新 `fencing_generation` 的 Attempt 能提交候选结果。
- 节点自报用量、平台观测用量和验证后用量必须分开保存。
- Provider 特有字段放扩展区或 Adapter 内，不污染核心调度语义。
- 新功能按模块拆分，禁止继续扩大协议和服务端巨型入口文件。
