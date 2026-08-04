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
| Provider / Offer / Job / Reservation / Lease / Receipt 统一领域合同 | 基础代码、v169-v176 注册表、Broker Reserve/未执行任务终态、v185 首次 Attempt 激活、v186 Lease 状态/续租、v187 staging 无用量中止及 v188 running 累计声明用量回执已写，状态为 `implementation_uncompiled`，尚未迁移、完整接线和运行验证 |
| Provider 本人控制面 | HTTP/MCP 已可登记、读取和列出本人 `user_node` 或 `managed_cluster` 的脱敏视图；只生成 `registering/self_declared` 记录，不接受路由、凭据、适配器或验证证据，尚未编译和运行验证 |
| CapacityPool 本人控制面 | HTTP/MCP 已可在本人 Provider 下登记、读取、列出和审计 `registering` Pool，并按稳定序号分页读取脱敏账本历史；审计健康不等于硬件 verified，历史省略消费者和业务因果字段；尚未编译和运行验证 |
| CapacityBucket 本人控制面 | HTTP/MCP 已可在本人当前 Pool 版本下创建 open、零发行余额 Bucket，并读取当前余额；窗口和 Bucket 摘要由服务端生成，不发行容量、不预留、不交易，尚未编译和运行验证 |
| Capacity Supply 本人控制面 | HTTP/MCP 已可显式确认后向同一窗口的多个 open Bucket 原子追加 self-declared 供给，或把尚在 available 的供给原子撤入 retired；服务端固定首次时间并复用现有双分录账本，available 不等于 verified 或可交易，尚未编译和运行验证 |
| 激活证据申请与计划控制面 | v177-v181、本人 HTTP/MCP、管理员审核/废止、申请/计划预检、不可变计划、原子应用与紧急隔离回执已写；应用单事务激活内部 Provider/Pool，隔离单事务把其当前 active 状态转为 quarantined。两者均不发送节点命令、不直接改写 Offer、不移动资金，恢复尚未实现，状态为 `implementation_uncompiled` |
| Offer 草稿、发布与生命周期控制面 | HTTP/MCP 已可创建、精确修订和撤销本人 draft Offer；管理员 HTTP 可原子发布 active、转为 draining，并在无 pending/active Reservation 时终结。v182-v184 保存追加式回执和依赖索引，所有写入口均不移动资金，状态为 `implementation_uncompiled` |
| 节点插件治理合同 | Signed Manifest、InstallPlan、双槽 lifecycle 与短期 ReadyCapability 合同已写；双 keyring 校验、精确 binding/time resolver 以及独立 SQLite v1 schema/私有事务 seam 已形成代码，均未编译接线或执行建库；生产 root pin、防回滚 keyring 安装、库存 CAS、计划应用、候选所有权、三段式下载认领与运行接线尚未完成 |
| 通用 Attempt 执行合同 | Start / RenewLease / Cancel 命令、Runner typed events 与 Host 盖章事件合同已写，尚未编译或接入云端协议 |
| 节点按需插件下载与通用任务执行 | 旧 LLM 已接入内部 Host seam，尚未编译；真实下载器、Sidecar/IPC、动态健康上报、通用任务派发和协议接线仍未实现 |
| 共享 CapacityPool 与追加式容量账本 | 领域合同、v165-v168 schema、隔离 Store、Store-canonical Supply/Claim 请求摘要、事务内 Claim kernel、只读审计、到期批处理、状态门卫和 epoch 轮换已写；v173 追加 Claim 完整历史，Hold V2 固定 causal binding，Reservation Claim 强制绑定 Offer/Job/Reservation，Finish 继承原 held 绑定；尚未编译、执行迁移或接线 |
| Provider 与 Offer 版本注册表 | v169/v170 schema、Provider/Offer 当前投影、追加式历史版本、规范摘要和容量引用审计已写；尚未编译、执行迁移或接线 |
| Price Snapshot 锁价控制面 | v171 不可变 Registry 及本人 HTTP/MCP 已写，可发布、读取和稳定列出 active Offer 的规范化 fallback_curve 快照；不预留容量、不冻结余额，也不代表真实市场价格，状态为 `implementation_uncompiled` |
| ComputeJob 版本注册表 | v172 schema、Workload/范围/预算合同校验、当前投影、不可变历史、幂等、CAS、状态机和依赖审计已写；项目级 HTTP/MCP 可创建 Job、发现并绑定 Offer/Price Snapshot，v175/v176 Broker 已组合写入；尚未编译、执行迁移或接入自动撮合 |
| ComputeReservation 版本注册表 | v174 schema、Job/Offer/Price Snapshot/Claim 精确版本绑定、当前投影、不可变历史、消费者幂等、CAS、状态机、完整依赖审计及事务内登记入口已写；HTTP/MCP 可读取本人或当前项目的最新列表与详情，独立写入口不移动容量或资金，v175/v176 Broker 已组合调用 |
| 消费者余额预授权 | v175 Broker 将显式到期预授权与 Job/Claim/Reservation 在同一事务内编排，并要求结果为 `reserved` 且含余额结果；v176 可在 Attempt 尚未激活时按精确预授权 ID 严格退款。仅支持 `platform_balance_cny`，不覆盖运行中任务或实际用量结算 |
| Broker 原子 Reserve 与未执行任务终态 | v175/v176 schema、不可变回执、严格请求重放与历史绑定审计已写；Reserve 单事务完成预算、容量、Reservation 和 Job，Finish 单事务完成退款、held Claim Release/Expire 与 Job/Reservation 终态。Reserve 首次回执到期后仍可按历史语义重放；项目级 Job 创建/锁价、登录用户查询及 Reserve/Release/Expire HTTP 与 MCP 控制面已接线，状态为 `implementation_uncompiled`，尚未执行迁移或运行验证 |
| Attempt 已接受激活回执 | v185、Store 与 Provider HTTP 已写；外部执行器接受后可显式登记首个 staging Lease，并在单事务内推进 held Claim、reserved Job 和 active Reservation。它不发送节点命令、不验证接受证明、不新增扣款，尚未编译、执行迁移或运行验证 |
| Attempt Lease 状态与续租 | v186、Store 与 Provider HTTP 已写；精确 revision/digest/fencing 栅栏下可登记外部心跳声明、延长软期限并追加续租回执。它不验证心跳签名、不发送节点命令、不改变容量或资金，尚未编译、执行迁移或运行验证 |
| staging Attempt 无用量安全中止 | v187、Store 与 Provider HTTP 已写；仅当前 revision 1、无心跳的 staging Lease 可在显式无执行声明下单事务全额退款、归还 active Claim、终结 Job/Reservation/Lease 并保存不可变回执。它不发送取消命令、不验证外部中止证明，尚未编译、执行迁移或运行验证 |
| running Attempt 累计声明用量 | v188、Store 与 Provider HTTP 已写；只接受精确 running Lease、完整 meter 集合、递增序号和不回退累计值，保存 `provider_declared` 与超额标记。它不改变状态、容量或资金，也不等于 verified usage，尚未编译、执行迁移或运行验证 |
| 外部算力池适配器与统一报价 | 已接受设计，尚未实现 |
| 多源验证、期货曲线与真实结算 | 已接受设计，尚未实现 |
| 二级容量市场与自动清算 | 目标架构，尚未实现 |

“已接受设计”不等于“已上线”。任何代理都必须保留实现状态，不得把文档中的目标合同描述成当前生产能力。

## 并行开发与远程去重协议

仓库级同步、提交与 rebase 时点严格复用 `WF-DEDUP`、`WF-PUSH`、`WF-REBASE`，操作细节见 `docs/ai-agent-workflow.md`，本目录不复制另一套 Git 流程。算力域只追加以下语义查重规则：

- 查重必须沿 Provider/Pool/Offer/Price Snapshot/Job/Reservation/Claim/Attempt/Lease/Receipt 整条能力链检查领域合同、迁移、Store、service、HTTP/MCP 与权威文档；文件名不同但推进同一状态或生成同一回执仍算重叠。
- 上游已有完整能力时转为复用和审查，只补不相交的层；命中同一符号、迁移版本、状态转换或事务边界时停止平行实现，不用改名制造第二套类型。
- 算力代理任务包记录 `TASK_BASE_SHA`、最近观察的远程 SHA、负责的能力/路径和禁止重叠区；代码进入远程后更新本页“当前事实”，继续区分已写、未编译、未接线和未实现。

## 阅读顺序

1. `docs/decisions/distributed-compute-federation-v1.md`：不可随意改变的架构决定。
2. `docs/distributed-compute/architecture.md`：Provider、控制面、数据面和任务状态。
3. `docs/distributed-compute/node-client-and-plugins.md`：客户端按需启用与插件边界。
4. `docs/distributed-compute/node-plugin-local-authority.md`：节点 SQLite 真源、根签名 keyring、计划应用、候选所有权与下载栅栏。
5. `docs/decisions/distributed-compute-capacity-ledger-v1.md` 与 `docs/distributed-compute/capacity-ledger.md`：共享容量池、跨 Offer 防超卖和追加式容量账本。
6. `docs/distributed-compute/market-and-settlement.md`：标准化 SKU、期货锁价和结算回执。
7. `docs/distributed-compute/provider-api.md`：Provider 本人登记、查询和信任边界。
8. `docs/distributed-compute/capacity-pool-api.md`：本人共享物理资源边界及摘要隐私合同。
9. `docs/distributed-compute/capacity-bucket-api.md`：交付窗口 Bucket 登记、余额读取和窗口不变量。
10. `docs/distributed-compute/capacity-supply-api.md`：本人供给追加、撤回、幂等和信任边界。
11. `docs/distributed-compute/activation-evidence-api.md`：证据申请、人工审核、版本复核和“批准不等于激活”边界。
12. `docs/distributed-compute/offer-api.md`：Offer 本人规范化草稿、管理员发布、安全退场与资金边界。
13. `docs/distributed-compute/price-snapshot-api.md`：Offer 派生 fallback_curve 报价、候选效果与无资金效果边界。
14. `docs/distributed-compute/broker-api.md`：Job、报价与预留 HTTP/MCP 控制面。
15. `docs/distributed-compute/attempt-activation-api.md`：首次 Attempt 已接受登记、fencing、原子状态变化与无节点命令效果边界。
16. `docs/distributed-compute/attempt-lease-api.md`：Lease 状态投影、受控续租、过期不可复活与无执行效果边界。
17. `docs/distributed-compute/attempt-abort-api.md`：staging 无用量中止、容量归还、退款和外部声明边界。
18. `docs/distributed-compute/attempt-usage-api.md`：running Attempt 累计声明用量、单调性、超额标记与无结算效果边界。
19. 现有兼容实现：`docs/decisions/node-compute-sharing-supply-v1.md`。

## 分阶段落地

### F0：统一语言和合同

版本化的 Provider、Offer、Workload、Job、Reservation、Attempt Lease、Execution Receipt、Settlement Receipt 和 Price Snapshot 基础合同已经写入代码。节点侧还形成了带 `fencing_generation` 的 Start / RenewLease / Cancel Attempt 命令、Runner typed events 和 Host 盖章事件合同；这些代码均尚未编译、接线或运行验证。现有 `LlmStreamRequest` 继续工作，不在首批协议变更中制造强制升级。

### F1：用户节点成为可插拔 Provider

节点内部已经形成 Plugin Host 兼容 seam、Signed Manifest、InstallPlan、双槽安装/切换/回滚 lifecycle 和 ReadyCapability 合同骨架，并写入真实 Ed25519/JCS 验证、Manifest 语义校验及本机 InstallPlan 准入内核；云端还形成 v169 版本化 Provider Registry、v170 追加式 Offer Registry及 v182-v184 发布/生命周期控制。本人可创建规范化 draft Offer；平台管理员可发布 active、转为 draining，并在无活动预留时终结为 expired/revoked。这些入口均尚未编译、执行迁移或运行验证。ReadyCapability 只是有明确过期时间的本机技术就绪证据，不包含市场价格、可预留容量或账户授权，**不等于 Compute Offer**。

目标流程仍是：共享关闭时不下载重型组件；开启后按硬件和任务选择签名插件、运行时与模型工件。本机耐久真源已经选定为独立 SQLite，根签名双 keyring、原子计划应用、候选所有权、三段式下载认领与可信时间边界见 `docs/distributed-compute/node-plugin-local-authority.md`；对应 Store、真实下载器、Sidecar 进程与 IPC、动态健康状态、云端 capability gate 和通用 Attempt 协议接线目前都未实现。

### F2：Broker、验证和真实结算

共享 CapacityPool 与追加式容量账本已经形成领域合同、checked-i128 reducer、v165-v168 SQLite schema 和隔离的本地 Store。Store 可登记池版本与零余额 bucket，原子追加多 meter 发行/撤出双分录，并通过稳定 Claim 完成 hold、revision 栅栏释放和到期归还。Supply Add/Withdraw 与 Claim Hold/Finish 不再接收调用方摘要；Hold V2 固定完整 causal binding，Reservation Claim 强制绑定 Offer、Job 与同主体 Reservation。Finish 摘要绑定 claim ID、expected revision、终态 action 和发生时间，并从原始 held 事务继承业务绑定。普通重放仍返回当前 Claim/余额，Reservation Hold 只重放未到期的初始 held 版本，尚未保存通用不可变首次响应。公开 standalone 方法继续拥有 `BEGIN IMMEDIATE` 与 commit，但拒绝单独创建或终结 Reservation Claim；事务内 kernel 不自行提交。Hold 必须显式到期、不允许在窗口结束后创建或晚于窗口结束；Release/Expire 只允许 `held -> available`，并用 checked `i128` 证明 Claim 自有归属，不能释放 active Attempt 容量。通用恢复器按 held 账本真实 Reservation binding 跳过 Broker Claim。v169-v174 还形成 Provider、Offer、Price Snapshot、Job、Claim 历史与 Reservation Registry；v175 第一版 Broker 已在一个 `BEGIN IMMEDIATE` 事务中组合余额预授权、Reservation Claim Hold、pending/active Reservation 与 quoted/reserved Job，并保存不可变回执。预算结果不是 `reserved`、缺少余额结果、任何依赖过期或任一步写入失败时，整笔事务回滚；同 Reservation ID 或消费者幂等键重放时必须匹配规范请求摘要并重新审计历史绑定。只有首次创建要求未来到期；首次回执在合同到期或预算后来进入终态后仍按历史语义重放，不依赖余额表的可变 `expires_at`。v176 为尚未激活 Attempt 的 Reservation 增加 Release/Expire 编排：严格核对 v175 原始绑定后，在一个事务中退款、归还 held Claim、把 Job 推进为 canceled/failed、把 Reservation 推进为 released/expired，并保存不可变终态回执；Claim 的持久化 `recorded_at` 是三个终态的共同时间边界。v185 增加首个 Attempt 已接受激活：Provider 所有者在外部执行器已接受后显式确认，单事务把 Claim `held -> active`、Job `reserved -> running`，更新 active Reservation 绑定并保存 staging Lease/不可变回执。v186 增加 Lease 当前状态投影和追加式续租回执；Provider 所有者可在精确 revision/digest/fencing 栅栏下提交外部心跳引用并延长软期限，已过期 Lease 不可复活。v185/v186 都保留原预算预授权和 active 容量，不发送节点命令，也不验证外部接受或心跳签名。登录用户 HTTP 与项目 MCP 均可列出或读取本人最新 Job/Reservation，并发起 Reserve、Release、Expire；Attempt 激活和续租写入口当前仅为 Provider 所有者 HTTP，参与 Provider 所有者和消费者可读取激活回执与当前 Lease 状态。上述路径只支持 `platform_balance_cny`，仍为 `implementation_uncompiled`，尚未执行迁移、HTTP/MCP 运行验证、真实节点派发、Lease 超时归还或运行中实际用量结算，因此不构成完整算力交易运行系统。

v187 继续补齐最窄的 staging 无用量中止：只有激活回执对应的首版、无心跳 Lease 才能由 Provider 所有者显式声明外部执行器未开始执行，并在一个事务内全额退回预授权、把 active Claim 归还 available、推进 Job/Reservation/Lease 终态并保存追加式回执。它不验证 `executor_abort_ref`、不发送取消命令，也不覆盖已开始执行、部分扣费、自动超时、调度重试或最终结算，状态仍为 `implementation_uncompiled`。

v188 再补齐 running Attempt 的累计声明用量证据：Provider 所有者只能在当前 Lease 精确 revision/digest/fencing 下追加完整 meter 快照，序号严格递增、累计值不得回退；高于预留合同的 meter 被保留并标记为 overage。回执明确为 `unverified_provider_declaration`，不更新 Lease/Job/Reservation/Claim，不消费容量、不扣款，也不产生 Provider 收益。真实 Host 事件接线、`observed_usage`、`verified_usage`、Execution Receipt 和结算仍未实现。
Offer 所有者 HTTP/MCP 可发布服务端规范化的 fallback_curve Price Snapshot；项目级 HTTP/MCP 可创建 submitted Job、发现当前有效候选，再把当前 revision/digest 锁定到所选报价。候选返回价格合同和最小 Provider 摘要，不返回节点路由、凭据或适配器配置。报价发布、候选发现和锁价均不移动资金或容量；真实价格源、批量报价和自动撮合仍未实现。

Provider 本人控制面由 `docs/distributed-compute/provider-api.md` 维护，Pool、Bucket 和 Supply 控制面分别由 `docs/distributed-compute/capacity-pool-api.md`、`docs/distributed-compute/capacity-bucket-api.md`、`docs/distributed-compute/capacity-supply-api.md` 维护；激活证据申请、计划与“内部激活不等于市场可交易”边界由 `docs/distributed-compute/activation-evidence-api.md` 维护；Offer 规范化草稿与无市场效果边界由 `docs/distributed-compute/offer-api.md` 维护；Job、报价、预留与未执行任务终态控制面由 `docs/distributed-compute/broker-api.md` 维护；首次 Attempt 已接受登记见 `docs/distributed-compute/attempt-activation-api.md`，Lease 状态与续租见 `docs/distributed-compute/attempt-lease-api.md`，staging 无用量中止见 `docs/distributed-compute/attempt-abort-api.md`，running 累计声明用量见 `docs/distributed-compute/attempt-usage-api.md`。

v172 ComputeJob Registry 已把需求身份、所选 Offer 历史版本、不可变 Price Snapshot、消费者预算上限和生命周期状态写入版本化 Store。新 Job 必须从 `submitted` 创建；项目控制面已接入该创建路径。进入 `quoted` 时只接受当前 active Offer、active Provider 与未过期快照，项目控制面可显式绑定或刷新既有锁价选择；进入 reserved 或后续状态后锁价合同不能更换。消费者幂等键、`expected_revision` 与 revision/digest CAS 防止重复或并发覆盖；当前和历史读取都会重新审计 Workload 合同及 Provider/Offer/Snapshot 依赖。该路径仍为 `implementation_uncompiled`，不代表预算已冻结、容量已预留或任务已派发。

v173 为 Capacity Claim 的每次 revision 保存完整不可变 JSON、状态和规范摘要，数据库拒绝修改或删除历史版本，历史 Reservation 不必依赖后来已变化的 Claim 当前投影。v174 ComputeReservation Registry 在此基础上精确绑定 Job、Offer、Price Snapshot 和 Claim 历史版本；创建和更新使用消费者级幂等、`expected_revision`、revision/digest CAS 与受限状态机，当前和历史读取都会重新审计全部依赖。注册表单独调用时只登记已存在的合同，不创建 Claim、不冻结预算或移动容量；v175/v176 Broker 才负责组合这些事务内入口。它们同样为 `implementation_uncompiled`。

后续实现可选择真实价格源/期货曲线、批量报价与撮合、容量自动调度与受控修复、Attempt 真实派发/续租/归还、重试时递增 `fencing_generation`、运行中任务与最终用量结算、多源计量、挑战任务、争议状态和可提取收益账本。

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
