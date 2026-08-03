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
| Provider / Offer / Job / Reservation / Lease / Receipt 统一领域合同 | 基础代码已写，尚未编译、接线和运行验证 |
| 节点插件治理合同 | Signed Manifest、InstallPlan、双槽安装/切换/回滚 lifecycle 与短期 ReadyCapability 合同已写，尚未编译或接线 |
| 通用 Attempt 执行合同 | Start / RenewLease / Cancel 命令、Runner typed events 与 Host 盖章事件合同已写，尚未编译或接入云端协议 |
| 节点按需插件下载与通用任务执行 | 旧 LLM 已接入内部 Host seam，尚未编译；真实下载器、Sidecar/IPC、动态健康上报、通用任务派发和协议接线仍未实现 |
| 共享 CapacityPool 与追加式容量账本 | 领域合同、checked-i128 reducer 与 v165 SQLite schema 骨架已写；尚未编译或执行迁移，Store、恢复器和 Broker 接线仍未实现 |
| 外部算力池适配器与统一报价 | 已接受设计，尚未实现 |
| 多源验证、标准化 SKU 与期货锁价结算 | 已接受设计，尚未实现 |
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

节点内部已经形成 Plugin Host 兼容 seam，以及 Signed Manifest、InstallPlan、双槽安装/切换/回滚 lifecycle 和 ReadyCapability 合同骨架，均尚未编译或接线。ReadyCapability 只是有明确过期时间的本机技术就绪证据，不包含市场价格、可预留容量或账户授权，**不等于 Compute Offer**；只有控制面结合策略、容量和价格后才能发布版本化 Offer。

目标流程仍是：共享关闭时不下载重型组件；开启后按硬件和任务选择签名插件、运行时与模型工件。真实下载器、Sidecar 进程与 IPC、动态健康状态、云端 capability gate、通用 Attempt 协议接线和 Offer 发布目前都未实现。

### F2：Broker、验证和真实结算

共享 CapacityPool 与追加式容量账本已经形成领域合同、checked-i128 reducer、v165 SQLite schema 和隔离的本地 Store 写入路径。Store 可登记池版本与零余额 bucket，原子追加多 meter 发行/撤出双分录，并通过稳定 Claim 完成 hold、revision 栅栏释放和到期归还；幂等重放返回当前余额。上述代码尚未编译、执行迁移或接入运行路由。Offer 发布、复制或续期都不会铸造容量；只有 Pool bucket 的 `supply_added` 进入账本后才形成可用余额。V1 最终仍要求一份 Reservation 只绑定一个 Pool 和一个精确 UTC 半开窗口 `[starts_at, ends_at)`，并在同一事务中完成多 meter 容量、消费者预算、Price Snapshot 与 Reservation 的原子预留。

后续实现 Offer Registry、报价锁定、容量审计与恢复器、容量和预算统一 Reserve、带 `fencing_generation` 的尝试租约、多源计量、挑战任务、争议状态和可提取收益账本。

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
