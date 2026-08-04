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
| Provider / Offer / Job / Reservation / Lease / Receipt 统一领域合同 | 基础代码、v169-v176 注册表、首版 Broker Reserve 与未执行任务终态已写，状态为 `implementation_uncompiled`，尚未迁移、接线和运行验证 |
| Provider 本人控制面 | HTTP/MCP 已可登记、读取和列出本人 `user_node` 或 `managed_cluster` 的脱敏视图；只生成 `registering/self_declared` 记录，不接受路由、凭据、适配器或验证证据，尚未编译和运行验证 |
| CapacityPool 本人控制面 | HTTP/MCP 已可在本人 Provider 下登记、读取、列出和审计 `registering` Pool，并按稳定序号分页读取脱敏账本历史；审计健康不等于硬件 verified，历史省略消费者和业务因果字段；尚未编译和运行验证 |
| CapacityBucket 本人控制面 | HTTP/MCP 已可在本人当前 Pool 版本下创建 open、零发行余额 Bucket，并读取当前余额；窗口和 Bucket 摘要由服务端生成，不发行容量、不预留、不交易，尚未编译和运行验证 |
| Capacity Supply 本人控制面 | HTTP/MCP 已可显式确认后向同一窗口的多个 open Bucket 原子追加 self-declared 供给，或把尚在 available 的供给原子撤入 retired；服务端固定首次时间并复用现有双分录账本，available 不等于 verified 或可交易，尚未编译和运行验证 |
| 激活证据申请控制面 | v177、本人 HTTP/MCP、管理员 HTTP 审核队列及双方只读就绪预检已写；申请锁定 Provider/Pool 精确版本和稳定账本审计摘要，预检解释路由、verified 硬件、信任层、版本与账本阻断，但 `activation_effect=none`，状态为 `implementation_uncompiled` |
| 节点插件治理合同 | Signed Manifest、InstallPlan、双槽安装/切换/回滚 lifecycle 与短期 ReadyCapability 合同已写，尚未编译或接线 |
| 通用 Attempt 执行合同 | Start / RenewLease / Cancel 命令、Runner typed events 与 Host 盖章事件合同已写，尚未编译或接入云端协议 |
| 节点按需插件下载与通用任务执行 | 旧 LLM 已接入内部 Host seam，尚未编译；真实下载器、Sidecar/IPC、动态健康上报、通用任务派发和协议接线仍未实现 |
| 共享 CapacityPool 与追加式容量账本 | 领域合同、v165-v168 schema、隔离 Store、Store-canonical Supply/Claim 请求摘要、事务内 Claim kernel、只读审计、到期批处理、状态门卫和 epoch 轮换已写；v173 追加 Claim 完整历史，Hold V2 固定 causal binding，Reservation Claim 强制绑定 Offer/Job/Reservation，Finish 继承原 held 绑定；尚未编译、执行迁移或接线 |
| Provider 与 Offer 版本注册表 | v169/v170 schema、Provider/Offer 当前投影、追加式历史版本、规范摘要和容量引用审计已写；尚未编译、执行迁移或接线 |
| Price Snapshot 锁价注册表 | v171 schema、active Offer/单一窗口/双价格腿/费用/来源绑定、不可变登记读取、精确重放和历史审计已写；尚未编译、执行迁移或接入报价/Broker |
| ComputeJob 版本注册表 | v172 schema、Workload/范围/预算合同校验、当前投影、不可变历史、幂等、CAS、状态机、历史依赖审计及事务内登记入口已写；项目级 HTTP/MCP 可创建本人 submitted Job、发现并绑定既有 Offer/Price Snapshot，HTTP/MCP 也可读取本人或当前项目的最新 Job 列表与详情，v175/v176 Broker 已组合写入；尚未编译、执行迁移或接入新报价生成与自动撮合 |
| ComputeReservation 版本注册表 | v174 schema、Job/Offer/Price Snapshot/Claim 精确版本绑定、当前投影、不可变历史、消费者幂等、CAS、状态机、完整依赖审计及事务内登记入口已写；HTTP/MCP 可读取本人或当前项目的最新列表与详情，独立写入口不移动容量或资金，v175/v176 Broker 已组合调用 |
| 消费者余额预授权 | v175 Broker 将显式到期预授权与 Job/Claim/Reservation 在同一事务内编排，并要求结果为 `reserved` 且含余额结果；v176 可在 Attempt 尚未激活时按精确预授权 ID 严格退款。仅支持 `platform_balance_cny`，不覆盖运行中任务或实际用量结算 |
| Broker 原子 Reserve 与未执行任务终态 | v175/v176 schema、不可变回执、严格请求重放与历史绑定审计已写；Reserve 单事务完成预算、容量、Reservation 和 Job，Finish 单事务完成退款、held Claim Release/Expire 与 Job/Reservation 终态。Reserve 首次回执到期后仍可按历史语义重放；项目级 Job 创建/锁价、登录用户查询及 Reserve/Release/Expire HTTP 与 MCP 控制面已接线，状态为 `implementation_uncompiled`，尚未执行迁移或运行验证 |
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
6. `docs/distributed-compute/provider-api.md`：Provider 本人登记、查询和信任边界。
7. `docs/distributed-compute/capacity-pool-api.md`：本人共享物理资源边界及摘要隐私合同。
8. `docs/distributed-compute/capacity-bucket-api.md`：交付窗口 Bucket 登记、余额读取和窗口不变量。
9. `docs/distributed-compute/capacity-supply-api.md`：本人供给追加、撤回、幂等和信任边界。
10. `docs/distributed-compute/activation-evidence-api.md`：证据申请、人工审核、版本复核和“批准不等于激活”边界。
11. `docs/distributed-compute/broker-api.md`：Job、报价与预留 HTTP/MCP 控制面。
12. 现有兼容实现：`docs/decisions/node-compute-sharing-supply-v1.md`。

## 分阶段落地

### F0：统一语言和合同

版本化的 Provider、Offer、Workload、Job、Reservation、Attempt Lease、Execution Receipt、Settlement Receipt 和 Price Snapshot 基础合同已经写入代码。节点侧还形成了带 `fencing_generation` 的 Start / RenewLease / Cancel Attempt 命令、Runner typed events 和 Host 盖章事件合同；这些代码均尚未编译、接线或运行验证。现有 `LlmStreamRequest` 继续工作，不在首批协议变更中制造强制升级。

### F1：用户节点成为可插拔 Provider

节点内部已经形成 Plugin Host 兼容 seam，以及 Signed Manifest、InstallPlan、双槽安装/切换/回滚 lifecycle 和 ReadyCapability 合同骨架；云端还形成 v169 版本化 Provider Registry、Offer 规范合同校验和 v170 追加式 Offer Registry。本人 HTTP/MCP 控制面可登记和查询 `user_node` 或 `managed_cluster`，但只产生服务端固定的 `registering/self_declared` 记录，响应不暴露路由、凭据、适配器或结算账户；还可在本人 Provider 下登记和查询 `registering` CapacityPool，服务端生成 epoch、revision 和摘要，响应不返回原始资源档案。它们均尚未编译、执行迁移或运行验证。ReadyCapability 只是有明确过期时间的本机技术就绪证据，不包含市场价格、可预留容量或账户授权，**不等于 Compute Offer**；只有控制面结合 Provider、策略、容量和价格后才能登记版本化 Offer。

目标流程仍是：共享关闭时不下载重型组件；开启后按硬件和任务选择签名插件、运行时与模型工件。真实下载器、Sidecar 进程与 IPC、动态健康状态、云端 capability gate、通用 Attempt 协议接线和 Offer 发布目前都未实现。

### F2：Broker、验证和真实结算

共享 CapacityPool 与追加式容量账本已经形成领域合同、checked-i128 reducer、v165-v168 SQLite schema 和隔离的本地 Store。Store 可登记池版本与零余额 bucket，原子追加多 meter 发行/撤出双分录，并通过稳定 Claim 完成 hold、revision 栅栏释放和到期归还。Supply Add/Withdraw 与 Claim Hold/Finish 不再接收调用方摘要；Hold V2 固定完整 causal binding，Reservation Claim 强制绑定 Offer、Job 与同主体 Reservation。Finish 摘要绑定 claim ID、expected revision、终态 action 和发生时间，并从原始 held 事务继承业务绑定。普通重放仍返回当前 Claim/余额，Reservation Hold 只重放未到期的初始 held 版本，尚未保存通用不可变首次响应。公开 standalone 方法继续拥有 `BEGIN IMMEDIATE` 与 commit，但拒绝单独创建或终结 Reservation Claim；事务内 kernel 不自行提交。Hold 必须显式到期、不允许在窗口结束后创建或晚于窗口结束；Release/Expire 只允许 `held -> available`，并用 checked `i128` 证明 Claim 自有归属，不能释放 active Attempt 容量。通用恢复器按 held 账本真实 Reservation binding 跳过 Broker Claim。v169-v174 还形成 Provider、Offer、Price Snapshot、Job、Claim 历史与 Reservation Registry；v175 第一版 Broker 已在一个 `BEGIN IMMEDIATE` 事务中组合余额预授权、Reservation Claim Hold、pending/active Reservation 与 quoted/reserved Job，并保存不可变回执。预算结果不是 `reserved`、缺少余额结果、任何依赖过期或任一步写入失败时，整笔事务回滚；同 Reservation ID 或消费者幂等键重放时必须匹配规范请求摘要并重新审计历史绑定。只有首次创建要求未来到期；首次回执在合同到期或预算后来进入终态后仍按历史语义重放，不依赖余额表的可变 `expires_at`。v176 为尚未激活 Attempt 的 Reservation 增加 Release/Expire 编排：严格核对 v175 原始绑定后，在一个事务中退款、归还 held Claim、把 Job 推进为 canceled/failed、把 Reservation 推进为 released/expired，并保存不可变终态回执；Claim 的持久化 `recorded_at` 是三个终态的共同时间边界。通用余额释放与到期器排除 Broker 预算，v176 通过精确预授权 ID 的严格入口终结，避免单腿退款。登录用户 HTTP 与项目 MCP 均可列出或读取本人最新 Job/Reservation，并发起 Reserve、Release、Expire；MCP 对 Reserve、Release 要求显式确认。该路径只支持 `platform_balance_cny`，按 Price Snapshot 的消费者最高微单位金额向上取整到人民币分；它仍为 `implementation_uncompiled`，尚未执行迁移、HTTP/MCP 运行验证、编排 Attempt 或完成运行中任务和实际用量结算，因此不构成完整算力交易运行系统。
项目级 HTTP 与 MCP 已可创建归属当前用户的 submitted Job，按完整 Job 合同发现当前有效的既有 Offer/Price Snapshot，再把当前 revision/digest 锁定到所选候选。候选返回价格合同和最小 Provider 摘要，不返回节点路由、凭据、适配器配置或授权名单。用户、项目、状态和时间由服务端固定，商户身份必须属于当前项目，相同消费者幂等键只能重放相同需求。候选发现和锁价均不移动资金或容量；新报价生成和自动撮合仍未实现。

Provider 本人控制面由 `docs/distributed-compute/provider-api.md` 维护，Pool、Bucket 和 Supply 控制面分别由 `docs/distributed-compute/capacity-pool-api.md`、`docs/distributed-compute/capacity-bucket-api.md`、`docs/distributed-compute/capacity-supply-api.md` 维护；激活证据申请与“批准不等于激活”边界由 `docs/distributed-compute/activation-evidence-api.md` 维护；Job、报价、预留与未执行任务终态控制面由 `docs/distributed-compute/broker-api.md` 维护。

v172 ComputeJob Registry 已把需求身份、所选 Offer 历史版本、不可变 Price Snapshot、消费者预算上限和生命周期状态写入版本化 Store。新 Job 必须从 `submitted` 创建；项目控制面已接入该创建路径。进入 `quoted` 时只接受当前 active Offer、active Provider 与未过期快照，项目控制面可显式绑定或刷新既有锁价选择；进入 reserved 或后续状态后锁价合同不能更换。消费者幂等键、`expected_revision` 与 revision/digest CAS 防止重复或并发覆盖；当前和历史读取都会重新审计 Workload 合同及 Provider/Offer/Snapshot 依赖。该路径仍为 `implementation_uncompiled`，不代表预算已冻结、容量已预留或任务已派发。

v173 为 Capacity Claim 的每次 revision 保存完整不可变 JSON、状态和规范摘要，数据库拒绝修改或删除历史版本，历史 Reservation 不必依赖后来已变化的 Claim 当前投影。v174 ComputeReservation Registry 在此基础上精确绑定 Job、Offer、Price Snapshot 和 Claim 历史版本；创建和更新使用消费者级幂等、`expected_revision`、revision/digest CAS 与受限状态机，当前和历史读取都会重新审计全部依赖。注册表单独调用时只登记已存在的合同，不创建 Claim、不冻结预算或移动容量；v175/v176 Broker 才负责组合这些事务内入口。它们同样为 `implementation_uncompiled`。

后续实现可选择 Offer 查询与撮合、价格源/期货曲线和报价生成、容量自动调度与受控修复、带 `fencing_generation` 的尝试租约、运行中任务与最终用量结算、多源计量、挑战任务、争议状态和可提取收益账本。

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
