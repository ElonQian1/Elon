---
title: 一龙任务级分布式算力联邦
status: current
reviewed_at: 2026-08-11
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

> v218 合同修正：历史“v11 Planning Snapshot V2”现以协议阈值 12 解释；work-admission `None` 允许新晋升槽首次重授权，`Some` 才承诺 current head。当前 A1 只有未编译的同事务 projector；A2 总合同已冻结，未编译、未运行的 A2b2 静态源码已覆盖 route→WAL-main、map/lock、barrier、完整 unmap、联合 close、route/registration typed custody/count inventory，但逐 case Windows 动态证据仍缺。无 producer，v14 永久 blocked-only，Runtime/Ready/派发不可达。

| 能力 | 2026-08-11 状态 |
|---|---|
| 节点模型白名单、最大并发、每日 Token 预算与执行租约 | 已实现，是兼容供给入口 |
| Provider / Offer / Job / Reservation / Lease / Receipt 统一领域合同 | 基础代码与 v169-v201 分段实现已写；Provider/Supply、Offer、Price Snapshot 和 Broker 部分链已有分层验证，其余 Attempt/结算仍有大量 `implementation_uncompiled` 或未接线入口。不能把局部通过描述为整条交易链可用 |
| Provider 本人控制面 | 服务/Store 及进程内 HTTP/MCP 已验证登记、读取、幂等和本人隔离；PC 自助登记已通过严格类型、lint 和生产构建。只生成 `registering/self_declared`，真实 TCP 与浏览器尚未验证 |
| CapacityPool 本人控制面 | 服务/Store 及进程内 HTTP/MCP 已验证登记、重放、审计和磁盘重开；PC `/compute-supply` 已通过严格类型、lint 和生产构建。审计健康不等于硬件 verified，真实 TCP 与浏览器尚未验证 |
| CapacityBucket 本人控制面 | 服务/Store 及进程内 HTTP/MCP 已验证同窗口双 meter Bucket、余额读取和磁盘重开；PC 已通过严格类型、lint 和生产构建。窗口和 Bucket 摘要由服务端生成，不发行可交易资产 |
| Capacity Supply 本人控制面 | 服务/Store 及进程内 HTTP/MCP 已验证多 Bucket 原子追加、撤回、幂等重放、显式确认和本人隔离；PC 已通过严格类型、lint 和生产构建。available 仍是 self-declared，不等于 verified 或可交易 |
| 激活证据申请与计划控制面 | v177-v181、v203-v205 已通过 Store/Service、本人/管理员 HTTP/MCP、角色隔离、幂等和文件重开专项；PC `/compute-supply`、`/compute-activation` 已静态生产构建。并发压力、真实 TCP、浏览器和生产库副本仍未验证；流程不发节点命令、不退款、不付款或移动资金，详见 `activation-control-plane-acceptance.md` |
| Offer 草稿、发布与生命周期控制面 | v170、v182-v184 已通过 Store/Service、本人和管理员 HTTP/MCP、角色隔离、幂等重放及文件重开专项；PC `/compute-supply`、`/compute-offers` 已静态生产构建。并发压力、真实 TCP、浏览器和生产库副本仍未验证；写入口不生成 Snapshot、不取消预留或移动资金，详见 `offer-control-plane-acceptance.md` |
| 节点插件治理合同 | Signed Manifest/InstallPlan、双 keyring、authority、受管取数/验证/staging/cleanup 与 v216 install/promotion 双回执已形成；v216 已随节点编译并通过 11 项版本链及既有 69 项 SQLite 回归，其中 3 项为单 Connection 成功路径。v217/v218 work-admission、Planning A1 尚未编译；A2b2 已形成未编译、未运行的 exact bridge、map/lock/barrier/unmap/联合 close/route/registration 静态源码和 typed custody/count inventory，但它不是逐 case Windows 动态证据。生产 VFS/register/live `sqlite3_file`/process owner/open、A1 producer、root/keyring/time/rollback、Signer、Host、downloader、Sidecar、Runtime、Ready 与 Attempt 接线不可达；v14 永久 blocked-only。精确边界见阅读顺序第 4 项 |
| 节点端点凭据与认证会话权威 | 服务端 v216-v218 已铺 endpoint credential/session、目标绑定 owner 重认证与单次消费；direct-TLS owner API、Windows DPAPI bootstrap、legacy no-downgrade 及 v13 auth-only WSS 均默认关闭。当前批次新增独立 v14 Planning bootstrap profile：固定单一 capability，在同一 exact endpoint generation 内只允许 sharing→preparation→Planning 六消息摘要链；每段观察与下一 intent 同事务重验耐久 session/credential并写 v219 provenance。NodeAgent 始终 `connected=false`，Planning 终点仍 `snapshot_ready=false`；无 signed Plan、work-admission、Runtime、Ready、route、outbox、Lease 或派发。源码尚未编译、测试、运行或执行迁移，状态仍为 `implementation_unwired`。见 `node-endpoint-session-authority.md` |
| Attempt Execution Plan / Gateway | v211-v214 已铺 sealed Plan、Provider-neutral Start、route/credential/actor authority、outbox 与 no-start recovery；v215 又在源码中形成 accepted observation→ACK→v185→actor/LeaseAuthority/commit→application 的 Store-local 原子闭包，并把既有 ACK-null cleanup pair 固定送往 quarantine。服务端及测试源码已编译；内存与临时文件 SQLite 的完整迁移、重复迁移、两次重开、关键对象及冲突 backfill 门卫已通过 4 项专项测试，但 accepted 成功闭包、生产数据库原位升级和生产链路未运行。旧人工激活/Renew/Abort 仍固定失败，可信输入、发送和远端证据 capability 无构造器，也无网络、worker、ACK ingress 或真实派发。见 `attempt-execution-plan-v1.md`、`attempt-execution-gateway-v1.md`、`attempt-delivery-outbox-v1.md` 与 `attempt-gateway-acceptance.md` |
| 节点本机 Attempt 执行合同 | Start / RenewLease / Cancel、Runner typed events 与 Host 盖章事件合同已写；它不是 Provider-neutral wire，尚未编译或接入云端协议 |
| 节点按需插件下载与通用任务执行 | 旧 LLM 已接入内部 Host seam，尚未编译；真实下载器、Sidecar/IPC、动态健康上报、通用任务派发和协议接线仍未实现 |
| 共享 CapacityPool 与追加式容量账本 | v165-v168 Supply/Claim 与 v173 Claim 历史已在 Supply、Offer 和 Broker 定向链中执行临时 SQLite 全量迁移；Broker 测试验证 held 与 release 回流。并发、到期批处理、生产磁盘和真实节点仍未验证 |
| Provider 与 Offer 版本注册表 | v169/v170 当前投影和追加式历史已随 Provider、Offer、Price Snapshot 与 Broker 定向链验证；进程内 HTTP/MCP 与 Offer 文件重开已验证，并发和生产磁盘仍未验证 |
| Price Snapshot 锁价控制面 | v171 Store/Service 已通过临时 SQLite 发布、幂等与审计专项；平台四眼 reference fallback v223/v224 已通过管理员 HTTP/MCP、原子 v171 Snapshot、拒绝零副作用、旧 TTL 触发器升级和文件重开专项，PC `/compute-reference-curves` 已通过跨层契约、严格类型、lint、生产构建和 bundle budget。两条来源都固定为 fallback_curve，不预留容量、不冻结余额，也不代表真实市场价格。真实 TCP、浏览器、并发压力、生产数据库副本与部署仍未验证，状态为 `implementation_partially_verified` |
| Provider Capacity Commitment v225 | `implementation_partially_verified`：immutable `committed` 主事实、唯一 `canceled|expired` terminal receipt、Store 原子 create/read/cancel/expire、generic bypass 与 owner/admin HTTP 已接线；数量/余额复用同一 Claim/ledger。生产目标、临时 SQLite 全量迁移、Store/Service/进程内 HTTP 和磁盘重开定向测试已通过；真实 TCP、生产升级、跨连接并发及交付结算未验证 |
| ComputeJob 版本注册表 | v172 Job 创建、候选发现、锁价、幂等、CAS 和依赖审计已随 Broker 组合链通过临时 SQLite 测试；项目级 HTTP/MCP、并发、生产磁盘和自动撮合仍未验证 |
| ComputeReservation 版本注册表 | v174 schema、Job/Offer/Price Snapshot/Claim 精确版本绑定、当前投影、不可变历史、消费者幂等、CAS、状态机、完整依赖审计及事务内登记入口已写；HTTP/MCP 可读取本人或当前项目的最新列表与详情，独立写入口不移动容量或资金，v175/v176 Broker 已组合调用 |
| 消费者余额预授权 | v175 Broker 将显式到期预授权与 Job/Claim/Reservation 在同一事务内编排，并要求结果为 `reserved` 且含余额结果；v176 可在 Attempt 尚未激活时按精确预授权 ID 严格退款。仅支持 `platform_balance_cny`，不覆盖运行中任务或实际用量结算 |
| Broker 原子 Reserve 与未执行任务终态 | v172-v176 已通过 2 项成功链/重放/回滚、1 项进程内 HTTP/MCP 身份与确认、2 项独立连接竞争及 1 项两次临时磁盘重开测试；服务端时间不再破坏终态重放。PC `/compute-market` 已静态生产构建；真实 TCP、高并发压力、生产数据库升级、异常断电恢复、派发与结算未验证，状态为 `implementation_partially_verified`。详见 `broker-control-plane-acceptance.md` |
| Attempt 已接受激活回执 | v185 状态推进内核仍负责原子激活 held Claim、reserved Job、active Reservation 与 staging Lease；v215 源码只在 sealed verified ACK 可用时同事务形成 exact application、actor receipt、Lease authority 与 commit outbox。服务端及测试源码已编译，内存/临时文件迁移、两次重开与冲突 backfill 门卫已验证；PC 已静态生产构建并关闭人工 Start。accepted 成功闭包、生产数据库原位升级和生产链路仍未运行，也无可信 ACK producer、生产 Adapter、节点执行或新增扣款 |
| Attempt Lease 状态与续租 | v186 的状态、续租 kernel 与历史读取仍在；Provider HTTP 人工续租写入口现固定 `COMPUTE_ATTEMPT_RENEW_GATEWAY_NOT_READY`，PC 已静态生产构建并禁用人工续租，避免未认证心跳绕过 durable Renew/fencing/recovery。读取仍可保留，真实 Renew 尚未实现 |
| staging Attempt 无用量安全中止 | v187 kernel 与历史读取仍在；Provider HTTP 人工“未执行”中止写入口现固定 `COMPUTE_ATTEMPT_ABORT_GATEWAY_NOT_READY`，PC 已静态生产构建并禁用人工中止。v214 cancel observation 只解锁 reconcile，仍不等于 no-start；只有 rejected 或 final reconcile+tombstone 的 exact authenticated proof 可解 v176，真实零用量补偿/Abort service-actor kernel 尚未实现 |
| running Attempt 累计声明用量 | v188、Provider HTTP 与 PC `/compute-execution` 已写；只读模板从当前合同返回 meter、上一累计值和下一序号，写入口只接受精确 running Lease、完整 meter 集合和不回退累计值，保存 `provider_declared` 与超额标记。v226 源码已补候选后封口与精确重放门卫，尚未编译、执行迁移或运行。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。它不改变状态、容量或资金，也不等于 verified usage |
| Attempt Provider 终态候选 | v189、追加式 Store、Provider HTTP 与 PC `/compute-execution` 已写；第一份候选必须绑定当前 running Lease、最新 v188 快照和服务端返回的 Workload 输出合同。v226 已线性化“当前流头→唯一候选→声明流封口”并令 v190-v195 重审 final usage currentness，源码尚未编译、执行迁移或运行。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。候选不推进状态、不消费容量、不移动资金，也不等于 Execution Receipt |
| Attempt 消费者终态审核 | v190、追加式 Store、消费者 HTTP 与 PC `/compute-reviews` 已写；本人待审核队列按消费者过滤并排除已有审核，第一份 `accepted/rejected/disputed` 必须绑定精确 v189 候选。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。审核只登记消费者证据，不等于平台验证或结算 |
| Attempt 平台终态观测 | v191、追加式 Store、管理员 HTTP 与 PC `/compute-observations` 已写；待观测队列返回已审计候选和最终 Provider meter，管理员可登记完整平台 meter、结果与证据引用并保存差异。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。观测不等于 verified usage、可信终态或结算 |
| Attempt Verification 决定 | v192、追加式 Store、管理员 HTTP 与 PC `/compute-verification` 已写；待验证队列返回重新审计的 v189-v191 证据链，管理员按保守策略登记 verified/compensable usage。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。决定不生成 Execution Receipt、不改状态和资金 |
| Attempt Execution Receipt | v193、追加式 Store、管理员 HTTP 与 PC `/compute-receipts` 已写；待签发队列只返回重新审计的 accepted Verification 与候选，管理员确认后固定执行身份、工件、用量和证明。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。回执不改状态、容量和资金 |
| Attempt 可信终态与容量收口 | v194、追加式 Store、管理员 HTTP 与 PC `/compute-finalization` 已写；待收口队列重新审计 v193 及当前状态并返回精确提交模板。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器、生产库和发布仍未验证。应用终态会推进状态和容量，但预授权和 Provider 收益不变 |
| Attempt 待结算回执 | v195、追加式 Store、独立 pending 账本、管理员 HTTP 与 PC `/compute-settlement-issuance` 已写；待办队列重审计资金来源并返回金额预览。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器、生产库和发布仍未验证。应用只结清平台内预授权并登记 pending，不是外部资金转移 |
| Attempt 结算挑战 | v196、追加式 Store、消费者待申诉队列、消费者/管理员 HTTP 与 PC `/compute-challenges` 已写；队列重审计本人 72 小时窗口、ledger 腿、pending 与释放门卫。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器、生产库和发布仍未验证。挑战只阻断未来 pending 释放，不退款、不裁决、不移动余额 |
| Attempt 结算生命周期历史 | v195-v199 角色 HTTP 与共享 PC 视图已写；消费者、Provider 和管理员分别读取本人、当前 Provider 与全局全部结算。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器、生产库和发布仍未验证。available 仅指内部余额且不证明外部付款 |
| Attempt 待结算原子释放 | v198、追加式 Store、Release Posting/账本腿与消费者/管理员 HTTP 已写；满 72 小时且挑战允许时，管理员可把内部 pending 原子转入 available。PC `/compute-settlement` 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。open/accepted 阻断，available 不等于提现或外部付款 |
| 到期结算释放队列与管理员批处理 | 管理员 HTTP 可按不透明 keyset 游标读取到期候选，再逐笔复用 v198；v202 保存批次意图和完成回执，PC 展示分页、当前页处理和批次历史。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。批次不是整批原子事务或后台定时器，不提现、不外部付款 |
| Attempt accepted 挑战纠正 | v199、追加式 Store、accepted 待纠正队列、Correction Posting/账本腿、角色 HTTP 与 PC `/compute-corrections` 已写；管理员以整数 fen/micros、守恒预览和双重确认提交平台内向下纠正。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器、生产库和发布仍未验证。纠正不等于外部退款到账 |
| Provider 提款申请与内部冻结 | v200、追加式 Store、Withdrawal Request Posting/账本腿与 Provider 本人 HTTP 已写；把 CNY available 原子转入 withdrawn 保留区。PC `/my-compute-settlement` 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。它只冻结内部余额，不执行或证明外部付款 |
| Provider 提款唯一终态 | v201、追加式 Store、Terminal Posting/账本腿与 Provider/管理员 HTTP 已写；取消或拒绝会全额返还 withdrawn，外部已付款声明只保存证据引用和摘要且不移动余额。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。它不发起或验证外部付款 |
| 结算账户审计视图与提款队列 | Provider 本人 HTTP 可从 v195、v198-v201 不可变账本重建账户和提款生命周期；管理员 HTTP 可重建固定平台账户并读取全局队列。PC 本人收益与管理员结算页面已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。视图不提供平台提款、不移动资金 |
| 外部算力池适配器与统一报价 | Provider onboarding v221 与 Adapter release v222 已编译迁移；10 个 onboarding、6 个 release HTTP 操作及 22 个分角色 MCP 工具已通过治理链和 Store 重开专项。PC `/compute-external-pools` 已通过跨层静态合同、严格类型、lint、生产构建和 bundle budget；真实 TCP、浏览器与生产部署仍未运行。它们只保存候选来源，不验证 artifact/verifier、不写 v213、不建 Pool/Offer 或派发。见 [`compute-management-mcp-acceptance.md`](compute-management-mcp-acceptance.md)、[`external-pool-adapter-authority.md`](external-pool-adapter-authority.md)、[`external-pool-onboarding-api-acceptance.md`](external-pool-onboarding-api-acceptance.md) 与 [`external-pool-adapter-release-api-acceptance.md`](external-pool-adapter-release-api-acceptance.md) |
| 平台参考回退曲线、真实价格源与多源验证 | reference fallback 的四眼 batch→review→atomic application 已通过 v223/v224 Store、管理员 Service/HTTP/MCP、旧库升级与文件重开专项，限定 `fallback_curve/sample_count=0` 且直接复用 v171。PC、真实 TCP 和生产部署未验证；index/mark/trade、真实市场样本、多源验证和自动撮合仍未实现 |
| 二级容量市场与自动清算 | 目标架构，尚未实现 |

“已接受设计”不等于“已上线”。任何代理都必须保留实现状态，不得把文档中的目标合同描述成当前生产能力。

节点失败候选清理的 schema v3 已包含 expected-object topology、candidate-parent identity anchor、plan seal 与四阶段 hash-chain step journal，并要求 completion 同时绑定 plan digest 和 terminal namespace-durability digest。当前代码已形成 topology 及首对象四阶段 typed Store、handle-derived identity/parent/name binding、Windows authority-domain parent namespace barrier 和进程内 exact recovery；mutation fence 已具备 exact scope/authority binding、线性 custody、固定 v1 wire codec/descriptor 与独立首方驱动供应链 shape。当前 canonical 已通过 sequence 1–4 对应的 32 项候选清理测试、9 项 wire 合同测试和 5 项特权组件 non-authorizing shape/binding 测试；这些证据不包含真实 WDK driver、Filter Manager transport、Bootstrap 首方验签、Windows catalog 验签、安装或安全构造器。evidence v2、显式 release、后续对象、terminal journal、跨重启物理恢复、旧执行器适配与 Host 仍未实现，不能理解为节点已经会自动清理生产失败候选。

## 并行开发与远程去重协议

仓库级同步、提交与 rebase 时点严格复用 `WF-DEDUP`、`WF-PUSH`、`WF-REBASE`，操作细节见 `docs/ai-agent-workflow.md`，本目录不复制另一套 Git 流程。算力域只追加以下语义查重规则：

- 查重必须沿 Provider/Pool/Offer/Price Snapshot/Job/Reservation/Claim/Attempt/Lease/Receipt 整条能力链检查领域合同、迁移、Store、service、HTTP/MCP 与权威文档；文件名不同但推进同一状态或生成同一回执仍算重叠。
- 上游已有完整能力时转为复用和审查，只补不相交的层；命中同一符号、迁移版本、状态转换或事务边界时停止平行实现，不用改名制造第二套类型。
- 算力代理任务包记录 `TASK_BASE_SHA`、最近观察的远程 SHA、负责的能力/路径和禁止重叠区；代码进入远程后更新本页“当前事实”，继续区分已写、未编译、未接线和未实现。

## 阅读顺序

1. `docs/decisions/distributed-compute-federation-v1.md`：不可随意改变的架构决定。
2. `docs/distributed-compute/architecture.md`：Provider、控制面、数据面和任务状态。
3. `docs/distributed-compute/node-client-and-plugins.md`：客户端按需启用与插件边界。
4. `docs/distributed-compute/node-endpoint-session-authority.md`、`docs/distributed-compute/node-plugin-local-authority.md`、`docs/distributed-compute/node-plugin-manifest-catalog-authority.md`、`docs/distributed-compute/node-plugin-vfs-fault-authority.md`、`docs/distributed-compute/node-plugin-planning-snapshot-authority.md`、`docs/distributed-compute/node-ready-capability.md`、`docs/distributed-compute/node-plugin-candidate-cleanup.md` 与 `docs/distributed-compute/windows-compute-namespace-fence-wire-v1.json`：端点 currentness、SQLite 真源、目录/回滚、测试 VFS 故障、Planning 投影、短期就绪、失败清理与 Windows hard-fence ABI。
5. `docs/decisions/distributed-compute-capacity-ledger-v1.md` 与 `docs/distributed-compute/capacity-ledger.md`：共享容量池、跨 Offer 防超卖和追加式容量账本。
6. `docs/distributed-compute/market-and-settlement.md`、`docs/distributed-compute/platform-reference-price-curve-authority.md` 与 `docs/distributed-compute/capacity-commitment-authority.md`：标准化 SKU、期货锁价、平台参考回退批次、v225 Provider 容量承诺和结算边界。
7. `docs/distributed-compute/provider-api.md`：Provider 本人登记、查询和信任边界。
8. `docs/distributed-compute/capacity-pool-api.md`：本人共享物理资源边界及摘要隐私合同。
9. `docs/distributed-compute/capacity-bucket-api.md`：交付窗口 Bucket 登记、余额读取和窗口不变量。
10. `docs/distributed-compute/capacity-supply-api.md`：本人供给追加、撤回、幂等和信任边界。
11. `docs/distributed-compute/activation-evidence-api.md`：证据申请、人工审核、版本复核和“批准不等于激活”边界。
12. `docs/distributed-compute/activation-recovery-api.md`：隔离恢复计划、第二人复核、显式废止重做、旧 Offer 退场门卫和追加式恢复边界。
13. `docs/distributed-compute/offer-api.md`：Offer 本人规范化草稿、管理员发布、安全退场与资金边界。
14. `docs/distributed-compute/price-snapshot-api.md`：Offer 派生 fallback_curve、计划中的平台 reference fallback producer、候选效果与无资金效果边界。
15. `docs/distributed-compute/broker-api.md`：Job、报价与预留 HTTP/MCP 控制面。
16. `docs/distributed-compute/attempt-execution-plan-v1.md`：可信 capability、ArtifactAccess、数值 ResourceGrant、不可变 Plan 与 v211 exact 门。
17. `docs/distributed-compute/attempt-execution-gateway-v1.md`：Provider-neutral Start、Adapter ACK、本地原子激活与 provisional 远端边界。
18. `docs/distributed-compute/attempt-delivery-outbox-v1.md`：Route authority、耐久投递、at-least-once 恢复与 no-start proof。
19. `docs/distributed-compute/attempt-activation-api.md`：v185 激活内核、fencing、原子状态变化与无节点命令效果边界。
20. `docs/distributed-compute/attempt-lease-api.md`：Lease 状态投影、受控续租、过期不可复活与无执行效果边界。
21. `docs/distributed-compute/attempt-abort-api.md`：staging 无用量中止、容量归还、退款和外部声明边界。
22. `docs/distributed-compute/attempt-usage-api.md`：running Attempt 累计声明用量、单调性、超额标记与无结算效果边界。
23. `docs/distributed-compute/attempt-terminal-candidate-api.md`：Provider 首次终态候选、输出合同、不可覆盖与无状态/无资金效果边界。
    - `docs/distributed-compute/attempt-final-usage-fence-authority.md`：v188 流头、v189 原子封口、精确重放和 v190-v195 currentness 继承。
24. `docs/distributed-compute/attempt-consumer-review-api.md`：消费者首次终态审核、证据引用、不可覆盖与非验证/非结算边界。
25. `docs/distributed-compute/attempt-platform-observation-api.md`：平台首次终态观测、累计 meter 差异、不可覆盖与非验证/非结算边界。
26. `docs/distributed-compute/attempt-verification-api.md`：保守 Verification policy、verified/compensable usage、不可覆盖与非状态/非结算边界。
27. `docs/distributed-compute/attempt-execution-receipt-api.md`：accepted Verification 的执行回执、完整源证据重审计与非状态/非结算边界。
28. `docs/distributed-compute/attempt-finalization-api.md`：精确 Execution Receipt 的可信终态、容量消费/归还与资金不变边界。
29. `docs/distributed-compute/attempt-settlement-api.md`：CNY 双价格腿、消费者预授权结清与 Provider pending 收益边界。
30. `docs/distributed-compute/attempt-settlement-challenge-api.md`：72 小时消费者挑战、不可覆盖记录与无余额移动边界。
31. `docs/distributed-compute/attempt-settlement-challenge-resolution-api.md`：消费者撤回、管理员裁决与释放门卫边界。
32. `docs/distributed-compute/attempt-settlement-correction-api.md`：accepted 挑战向下金额纠正、消费者退款和 pending 冲减边界。
33. `docs/distributed-compute/attempt-settlement-release-api.md`：72 小时后纠正净额从 pending 到 available 的原子释放、账本与非提现边界。
34. `docs/distributed-compute/settlement-withdrawal-request-api.md`：Provider available 提款申请、withdrawn 内部冻结与非付款边界。
35. `docs/distributed-compute/settlement-withdrawal-terminal-api.md`：提款取消、拒绝、外部已付款声明和唯一终态边界。
36. `docs/distributed-compute/settlement-account-view-api.md`：Provider 结算账户账本重建与管理员提款队列边界。
37. `docs/distributed-compute/settlement-release-batch-api.md`：到期候选、逐笔 v198 释放、部分失败报告与非后台自动化边界。
38. 现有兼容实现：`docs/decisions/node-compute-sharing-supply-v1.md`。

## 分阶段落地

### F0：统一语言和合同

版本化的 Provider、Offer、Workload、Job、Reservation、Attempt Lease、Execution Receipt、Settlement Receipt 和 Price Snapshot 基础合同已经写入代码。节点侧还形成了带 `fencing_generation` 的 Start / RenewLease / Cancel Attempt 命令、Runner typed events 和 Host 盖章事件合同；这些代码均尚未编译、接线或运行验证。现有 `LlmStreamRequest` 继续工作，不在首批协议变更中制造强制升级。

### F1：用户节点成为可插拔 Provider

节点内部已经形成 Plugin Host 兼容 seam、Signed Manifest、InstallPlan、双槽安装/切换/回滚 lifecycle 和 ReadyCapability 合同骨架，并写入真实 Ed25519/JCS 验证、Manifest 语义校验及本机 InstallPlan 准入内核；节点供应链源码已随 `elon-pc-node` 编译通过，但未启动或生产接线。云端还形成 v169 版本化 Provider Registry、v170 追加式 Offer Registry及 v182-v184 发布/生命周期控制；本人可创建规范化 draft Offer，平台管理员可发布 active、转为 draining，并在无活动预留时终结为 expired/revoked。Offer 已通过 Store/Service、进程内 HTTP/MCP 与文件重开专项，但生产数据库和真实 TCP 仍未验证。ReadyCapability 只是有明确过期时间的本机技术就绪证据，不包含市场价格、可预留容量或账户授权，**不等于 Compute Offer**。

目标流程仍是：共享关闭时不下载重型组件；开启后按硬件和任务选择签名插件、运行时与模型工件；只有用户开启并确需 cleanup hard-fence 时，才按独立首方特权组件合同下载并经 UAC 安装驱动，普通插件签名无权授权内核代码。本机耐久真源已选定为独立 SQLite，根签名双 keyring、原子计划应用、候选所有权、三段式下载认领、候选级全工件验证和可信时间边界见 `docs/distributed-compute/node-plugin-local-authority.md`。raw verified 只把本机槽推进到 `verifying`；staged 候选可经健康失败 quarantine 进入 `failed`，再进入 cleanup authorization。固定句柄旧执行器和 completion Store 内核已编译；中间路线已能封存 topology 并写入首对象 intent、disposition、absence 与 namespace durability，sequence 1–4 的 32 项清理测试均已通过，但 sequence 3/4 只覆盖 builder 链和 SQLite exact-row。mutation fence 是 exact scope/authority-bound 的线性租约；其签名 minifilter wire/供应链 shape 已通过 9+5 项独立合同测试，但真实驱动、首方信任/fingerprint、Windows catalog、安装、transport、safe constructor、evidence v2 与显式 release 仍未实现，后续对象/terminal producer 也缺。只有完整 journal exact readback 后返回的不透明终态能力才能进入 completion，并恢复为 `NotCreated` 或 exact `Completed`。因此当前完整清理链仍不可达，不会因内存物理证据或首对象完成直接释放 owner。候选观察值仍由 Host 调用方提供，尚未证明真实 Sidecar 已运行；v216 retained-handle 再验与本机 install/promotion 双回执已随节点编译，v7 schema 全新安装、重开及 v3-v6 原子迁移等 11 项测试通过。生产磁盘迁移、install/promotion 完整事务夹具、生产时间权威、真实下载器、Sidecar/IPC/探针调度、完整清理事务夹具、跨重启恢复、Host 接线、work-admission、云端 capability gate 和通用 Attempt 协议仍未实现。

NodeRuntime 已挂载默认关闭的 Compute Bootstrap，只派生 installation/data-root/authority 身份与路径并绑定节点状态目录实例锁 weak witness；该 witness 只作为进程存活前提，不能替代工件根锁。现有 `pin_compute_plugin_root()` 已能返回同时持有 canonical pinned root 与 share-none 锁句柄的非 Clone capability，但 Bootstrap 尚无 sharing-on transition，因此管理状态仍如实显示根锁未取得，生产 trusted-time、生产回滚见证、root pin、authority open 和 process fence 也不可用。数据根变化要求重启；默认关闭不会打开数据库、执行下载或启动 Sidecar。后续 Runner/节点 Attempt 持久化将使用 schema v4 的独立执行 fence 域，不把高频 run 状态塞入 inventory，也不复用下载 cancellation。

F1 的下一硬门槛是实际执行 [`node-plugin-vfs-fault-authority.md`](node-plugin-vfs-fault-authority.md) 的 Windows SHM、联合关闭、route/registration 与多 Connection 逐 case 验证，并把动态观察逐条对齐 A2b2 静态 key。A2b2 仍未编译、未运行，不能据此进入生产 VFS/open、Planning producer、v15、Runtime 或 Ready。

### F2：Broker、验证和真实结算

共享 CapacityPool 与追加式容量账本已形成 checked-i128 reducer、v165-v168 schema 和隔离 Store：多 meter 发行/撤出、Claim hold/release/expire 均保存 causal binding，Reservation Claim 绑定 Offer、Job 与同主体 Reservation；公开 standalone 方法拥有事务，组合 kernel 不自行提交。v169-v174 形成 Provider、Offer、Price Snapshot、Job、Claim 历史与 Reservation Registry；v175 在一个 `BEGIN IMMEDIATE` 中组合余额预授权、Claim Hold、Reservation 与 Job 并保存不可变回执，任何依赖或资金步骤失败均整体回滚。v176 对没有 Start command 的 Reservation 可按原合同 Release/Expire；一旦已有 Start，则只有逐字段复算并写入 finish receipt 的 exact no-start proof 才能退款和归还容量，无 ACK、accepted、quarantined、超时或 cancel ACK 都继续失败关闭。

v185 保留唯一 Attempt 激活状态推进 kernel：单事务把 Claim `held -> active`、Job `reserved -> running`，更新 Reservation 并保存 staging Lease/回执。v211 已关闭 Provider 所有者人工确认激活；v214 形成 rejected/quarantine/reconcile 的 Store-local cleanup/recovery。v215 源码再把 authenticated accepted observation、ACK、v185、application actor、LeaseAuthorityBinding、commit outbox 与 application 放入同一事务；既有 ACK-null cleanup pair 只能 quarantine，历史重放不重新要求当前授权。服务端及测试源码已编译，内存与临时文件 SQLite 完整迁移、重复迁移、两次重开、关键对象和冲突 backfill 门卫已验证；accepted 成功闭包、生产数据库原位升级和生产链路仍未运行。Offer `draining` 继续按 Reservation 历史版本履约，人工 Renew/Abort 写入口固定失败，也没有 sealed 输入构造器、网络或 worker。

登录用户 HTTP/MCP 可读本人 Job/Reservation 并发起 Reserve、Release、Expire；旧 Attempt 激活 POST 已稳定失败，历史参与方仍可读激活回执与 Lease。上述 Broker 仅支持 `platform_balance_cny`，Store/Service、进程内接口、两连接竞争和两次临时磁盘重开已定向验证；生产数据库升级、异常断电恢复、真实 TCP、高并发压力、真实派发、超时归还和实际用量结算仍未验证，不能视为完整算力交易系统。

v187 保留最窄 staging 无用量中止 kernel 与历史回执，但 Provider 所有者人工写入口已关闭：未认证的 `executor_abort_ref` 或勾选“未执行”不能解 no-start 门。v214 产生的 exact rejected/final-reconcile proof 只可供 v176 重审计，不调用 v187，也不替代尚缺的 service-actor 补偿 kernel；当前没有真实取消/reconcile 网络，更不覆盖已开始执行、部分扣费、自动超时、调度重试或最终结算。

v188 再补齐 running Attempt 的累计声明用量证据：Provider 所有者只能在当前 Lease 精确 revision/digest/fencing 下追加完整 meter 快照，序号严格递增、累计值不得回退；高于预留合同的 meter 被保留并标记为 overage。v226 源码已固定候选前可追加、候选原子绑定当前流头、候选后只可精确重放；当前仍未编译或运行。回执仍为 `unverified_provider_declaration`，不更新 Lease/Job/Reservation/Claim，不消费容量、不扣款，也不产生 Provider 收益。

v189 再保存 Provider 首次终态候选：当前 running Lease 必须已有最新 v188 用量快照，且 Lease、Job、Reservation、Claim 版本和摘要完全一致；v226 源码以 IMMEDIATE 迁移门卫和统一 Store 审计固定 final usage 仍是当前流头，并使 v190-v195 对漂移失败关闭。`succeeded` 结果按 Workload 输出合同校验，`failed/canceled` 不得携带伪最终产物。候选只保存为 `unverified_provider_declaration`，不更新状态、容量或资金，也不等于 Execution Receipt。

v190 再保存消费者第一份终态审核证据：只有 v189 候选绑定的 Job 消费者可提交 `accepted/rejected/disputed`，并固定候选事件摘要和完整因果链；拒绝或争议必须提供证据引用。该记录固定为 `consumer_attestation_only`，不产生 Verification 决定，不更新状态、容量或资金，也不会因消费者接受而自动付款。

v191 再保存平台第一份终态观测证据：平台 `admin/owner` 可登记 control plane、transport gateway 或 server metering 的完整累计 meter，并固定与最终 v188 快照不同的 meter。该记录固定为 `unverified_platform_observation`；差异或一致都不产生 Verification 决定，也不更新状态、容量或资金。

v192 首次保存平台 Verification 决定：管理员精确绑定 v189-v191 后，`conservative_min_v1` 仅在消费者接受且 Provider/平台 outcome 一致时允许 accepted；verified usage 逐 meter 取双方较小值，compensable usage 再受 Reservation 预留上限约束。rejected/disputed 记录零用量。回执不可覆盖，但不生成 Execution Receipt、不推进状态、不消费容量、不移动资金。

v193 再基于 accepted v192 签发 Execution Receipt：回执重新审计 Attempt 激活、Job/Reservation 历史及 v188-v192，固定 executor、Offer、runner/plugin/model、输入输出、四类用量、三方证明和 Verification。回执不可覆盖，但不推进 Lease/Job、不消费 Claim/Reservation、不移动资金。

v194 再基于由 accepted Verification 签发的精确 v193 Execution Receipt 应用可信终态：单一事务把 Lease 推进为 terminal、Job 推进为 `verification_pending`、Reservation/Claim 推进为 consumed；consumable meter 消费 compensable usage 并归还余量，reusable meter 全量归还。回执不可覆盖，预授权与 Provider 收益仍不变。

v195 再基于精确 v194/v193、Broker 预授权和 Price Snapshot 生成不可变 Settlement Receipt：消费者价格腿使用 verified usage 并按快照舍入到人民币分，Provider 价格腿使用 compensable usage；单事务扣结预授权、退回未用余额、登记 Provider/平台 pending 收益并把 Job 推进为 `settled`。首版仅支持 CNY 基础组件，pending 不可提现，不调用真实支付或链上网络。v196 允许消费者在回执创建后的固定 72 小时内提交一份不可覆盖挑战；v197 再把撤回、接受或驳回保存为唯一终态。两者都不改写结算或余额。v199 对 accepted 挑战追加向下金额纠正，原子退款消费者并冲减 Provider/平台 pending；v198 在 72 小时窗口结束且挑战门卫允许时，用独立 Release Receipt 和四条账本腿把原金额或纠正净额从 pending 原子转入 available。管理员现可读取有界到期候选并逐笔复用 v198；这是人工触发的部分成功批处理，不是后台定时清算。v200 再允许 Provider 所有者把本人 available 原子转入 withdrawn 提款保留区；v201 为申请增加取消、拒绝或外部已付款声明的唯一终态。取消/拒绝返还内部余额，付款声明只保存证据，不调用或验证外部资金网络。
Offer 所有者 HTTP/MCP 可发布服务端规范化的 fallback_curve Price Snapshot；项目级 HTTP/MCP 可创建 submitted Job、发现当前有效候选，再把当前 revision/digest 锁定到所选报价。平台 reference fallback 现由管理员 HTTP/MCP 完成 exact batch 提交、独立复核、preflight 和原子 application；v223/v224 直接登记 entry 对应的唯一 v171 Snapshot，并通过临时文件迁移与重开专项。它仍固定为 `fallback_curve/sample_count=0`，不代表 index/mark/trade；平台曲线 PC、真实 TCP 与生产部署未验证。候选不返回节点路由、凭据或适配器配置，任何报价发布和锁价都不自动移动资金或容量。

Provider、Pool、Bucket、Supply、激活、Offer、平台参考价格和 Broker 各自的控制面与证据由本页“阅读顺序”中的专题文档维护。PC `/compute-supply`、`/compute-activation`、`/compute-offers`、`/compute-reference-curves`、`/compute-market` 与 12 个 Attempt/结算角色路由已完成静态生产构建；Attempt 后端操作级专项仍未运行。静态构建不代表接口联调、浏览器验收、生产迁移或发布，参考价格、Broker 与 Attempt PC 证据分别见 `platform-reference-price-curve-api-acceptance.md`、`broker-control-plane-acceptance.md` 与 `pc-compute-attempt-workbenches-acceptance.md`。

v172 ComputeJob Registry 已把需求身份、所选 Offer 历史版本、不可变 Price Snapshot、消费者预算上限和生命周期状态写入版本化 Store。新 Job 从 `submitted` 创建并只从当前合格候选进入 `quoted`；消费者幂等键、revision/digest CAS、历史依赖审计及临时磁盘重开已随 Broker 组合链通过定向测试。该结论不代表真实 TCP、生产数据库升级、异常断电恢复、自动撮合或任务派发已经验证。

v173/v174 Claim 与 Reservation Registry 保存不可变历史并精确绑定 Job、Offer、Price Snapshot 和 Claim 版本；v175/v176 Broker 组合入口已通过成功预留、未执行释放、双向幂等重放、余额不足整笔回滚、两连接竞争和两次临时磁盘重开测试。注册表单独调用仍不创建 Claim、不冻结预算或移动容量；真实 TCP、高并发压力、生产数据库升级和异常断电恢复尚未验证。

平台 reference fallback 管理面已形成；后续不再重复实现其 DTO 或五账本，应分别推进真实价格源/期货曲线、批量报价与撮合、容量自动调度与受控修复、Attempt 真实派发/续租/归还、重试时递增 `fencing_generation`、运行中任务与最终用量结算、多源计量、挑战任务、争议状态和可提取收益账本。

### F3：外部矿池与企业集群

服务端 Provider Adapter 统一接入公司集群、云 GPU 和其他算力池；每个 Provider 保留自己的内部调度，只向一龙提交标准回执。Provider 首段来源权威见 [`external-pool-adapter-authority.md`](external-pool-adapter-authority.md)，Adapter release 候选来源见 [`external-pool-adapter-release-authority.md`](external-pool-adapter-release-authority.md)：v221/v222 已编译迁移，onboarding 的 10 个管理操作与 Store 重开共 7 项专项通过，release 的 6 个管理员操作与 Store 重开共 7 项专项通过，5 个本人 MCP 与 17 个平台 MCP 工具也已通过角色隔离和治理链专项；生产部署和 PC 管理入口仍未运行。onboarding application 与 staged admission 均不证明 artifact/verifier、credential、route、容量、派发或结算。

### F4：容量期货市场

以标准化 Compute SKU 和交付窗口发行容量合约，引入订单、持仓、指数价、标记价、保证资源和到期交割；任务结算消费已锁定的价格快照。

首段平台参考价格只走四眼治理的 `fallback_curve/sample_count=0`；v223/v224 application 已在一个事务中直接登记既有 v171 Snapshot，并通过管理员 HTTP、迁移与文件重开专项。它不是指数、标记价、成交、订单簿或持仓，详见 [`platform-reference-price-curve-authority.md`](platform-reference-price-curve-authority.md) 与 [`platform-reference-price-curve-api-acceptance.md`](platform-reference-price-curve-api-acceptance.md)。

v225 Provider Capacity Commitment 详见 [`capacity-commitment-authority.md`](capacity-commitment-authority.md) 与 [`capacity-commitment-acceptance.md`](capacity-commitment-acceptance.md)。它只允许本地 current Provider/Offer/Pool 在 exact `capacity_future` v171 Snapshot 与已批准应用的 v223 binding 下，把同一 Claim/ledger 的完整 meter/window 从 available 锁到 held，再以唯一 canceled/expired receipt 原子归还；不包含 `external_pool`、DeliveryAllocation、资金或结算。当前为 `implementation_partially_verified`：生产目标、临时 SQLite、进程内 HTTP 和重开已有定向证据，真实 TCP、生产升级和并发压力未验证。

## 当前工程指令

当前工程按仓库任务预检、风险匹配验证和统一收尾执行。文档中的 `implementation_uncompiled`、`implementation_unrun` 或目标设计不能自动升级为已完成；只有形成可调用入口并留下对应编译、迁移和定向验收证据后，才能更新当前状态。

## 代理接力规则

- 核心合同使用显式 `schema_version`，新增字段优先保持向后兼容。
- 金额、价格和用量禁止使用浮点数；统一使用整数微单位、基点或有理数比例。
- Offer 和 Price Snapshot 一经被 Job 引用便不可修改，只能创建新版本。
- 一次 Job 可以有多个 Attempt，但任何时刻只有拥有最新 `fencing_generation` 的 Attempt 能提交候选结果。
- 节点自报用量、平台观测用量和验证后用量必须分开保存。
- Provider 特有字段放扩展区或 Adapter 内，不污染核心调度语义。
- 新功能按模块拆分，禁止继续扩大协议和服务端巨型入口文件。
