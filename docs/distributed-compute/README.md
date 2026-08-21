---
title: 一龙任务级分布式算力联邦
status: current
reviewed_at: 2026-08-21
owners: backend, node, ai-economy
---

# 一龙任务级分布式算力联邦

本目录是“一龙成为 AI 算力矿池与联邦入口”的权威设计入口。目标不是只共享一个本地模型的推理接口，而是把用户闲置节点、平台集群和外部算力池统一成可发现、可报价、可预留、可执行、可验证、可结算的任务级算力网络。各纵切面的最新成熟度统一见 [`current-implementation-status.md`](current-implementation-status.md)。

## 北极星

- 一龙是需求聚合器、调度与验证网络、算力市场及统一结算层。
- 用户节点是自主供给方；只有主动开启共享后才下载节点核心、运行时插件和模型工件。
- 外部公司、云 GPU 和其他矿池通过 Provider Adapter 接入，对上暴露统一 Offer、Lease 和 Receipt。
- 用户下载的开源本地模型既可以在自己的设备运行，也可以把可拆分任务调度到联邦节点执行。
- 公网异构节点优先聚合“任务”，不假装组成一张低延迟虚拟 GPU；真正的张量并行或流水线并行由受管集群内部完成，并作为一个逻辑 Provider 接入。
- 供需双方先使用不可变的期货/远期价格快照结算，随后演进到标准化容量合约、订单簿、持仓和清算。

## 当前事实

> v218 合同修正：历史“v11 Planning Snapshot V2”现以协议阈值 12 解释；work-admission `None` 允许新晋升槽首次重授权，`Some` 才承诺 current head。A1 同事务 projector 已随完整测试目标编译，但无专项运行或 producer；A2 总合同已冻结，测试目标可编译且 targeted fault matrix 5 项通过，但 A2b2 `WindowsDynamic=0/117`、宽范围回归未通过，状态仍为 `implementation_not_dynamically_accepted`。v14 永久 blocked-only，Runtime/Ready/派发不可达。

| 能力 | 当前状态 |
|---|---|
| 节点模型白名单、最大并发、每日 Token 预算与执行租约 | 已实现，是兼容供给入口 |
| 旧节点 LLM 联邦兼容观察 | 真实 `NodeComputeRun` 与认证 `/api/me/node-usage` 已接入附加只读投影；只认 `server_node_llm/node_llm`，固定 `partial/provider_reported_unverified`，原始数组不变。PC 账本源码只按逐字 `source_run_id=run.id` 在原始行内显示兼容观察、未验证计量和旧结算边界；未知或畸形投影整侧不贴标签。源码未编译、未运行、未做浏览器或读屏验证（`passed=0`），不生成 Job、Reservation、Attempt、Lease、Receipt、可信计量或结算 |
| Provider / Offer / Job / Reservation / Lease / Receipt 统一领域合同 | 基础代码与 v169-v201 分段实现已写；Provider/Supply、Offer、Price Snapshot 和 Broker 部分链已有分层验证，其余 Attempt/结算仍有大量 `implementation_uncompiled` 或未接线入口。不能把局部通过描述为整条交易链可用 |
| Provider 本人控制面 | 服务/Store 及进程内 HTTP/MCP 已验证登记、读取、幂等和本人隔离；PC 自助登记已通过严格类型、lint 和生产构建。只生成 `registering/self_declared`，真实 TCP 与浏览器尚未验证 |
| UserNode Provider Binding Root | V279 已冻结并写入私有 Domain/canonical/validator、37列表 migration、Store/API、activation gate 与 authority/acceptance 源码：确定性 binding ID 把既有 `user_node` Provider genesis、node、安装身份和 endpoint installation binding 固定为不可变 identity root，并在 fresh activation 同事务重证中关闭任意 `node_binding_ref`。当前严格为 `source_written/source_review_only/implementation_uncompiled/implementation_unrun`、`passed=0/failed=0`；不生成 Ready、route、Offer、Attempt、Lease、Execution Receipt 或经济效果。见 [`V279 authority`](user-node-provider-binding-authority.md) 与 [`acceptance`](user-node-provider-binding-acceptance.md) |
| CapacityPool 本人控制面 | 服务/Store 及进程内 HTTP/MCP 已验证登记、重放、审计和磁盘重开；PC `/compute-supply` 已通过严格类型、lint 和生产构建。审计健康不等于硬件 verified，真实 TCP 与浏览器尚未验证 |
| CapacityBucket 本人控制面 | 服务/Store 及进程内 HTTP/MCP 已验证同窗口双 meter Bucket、余额读取和磁盘重开；PC 已通过严格类型、lint 和生产构建。窗口和 Bucket 摘要由服务端生成，不发行可交易资产 |
| Capacity Supply 本人控制面 | 服务/Store 及进程内 HTTP/MCP 已验证多 Bucket 原子追加、撤回、幂等重放、显式确认和本人隔离；PC 已通过严格类型、lint 和生产构建。available 仍是 self-declared，不等于 verified 或可交易 |
| 激活证据申请与计划控制面 | v177-v181、v203-v205 已通过 Store/Service、本人/管理员 HTTP/MCP、角色隔离、幂等和文件重开专项；PC `/compute-supply`、`/compute-activation` 已静态生产构建。并发压力、真实 TCP、浏览器和生产库副本仍未验证；流程不发节点命令、不退款、不付款或移动资金，详见 `activation-control-plane-acceptance.md` |
| Offer 草稿、发布与生命周期控制面 | v170、v182-v184 已通过 Store/Service、本人和管理员 HTTP/MCP、角色隔离、幂等重放及文件重开专项；PC `/compute-supply`、`/compute-offers` 已静态生产构建。并发压力、真实 TCP、浏览器和生产库副本仍未验证；写入口不生成 Snapshot、不取消预留或移动资金，详见 `offer-control-plane-acceptance.md` |
| 节点插件治理合同 | Signed Manifest/InstallPlan、双 keyring、authority、受管取数/验证/staging/cleanup 与 v216 install/promotion 双回执已形成；v216 已随节点编译并通过 11 项版本链及既有 69 项 SQLite 回归，其中 3 项为单 Connection 成功路径。v217/v218 work-admission 与 Planning A1 已随完整测试目标编译，但无专项、迁移或 runtime 验证；A2 targeted fault matrix 5 项通过，但 A2b2 `WindowsDynamic=0/117`、宽范围回归未通过。生产 VFS/register/live `sqlite3_file`/process owner/open、A1 producer、root/keyring/time/rollback、Signer、Host、downloader、Sidecar、Runtime、Ready 与 Attempt 接线不可达；v14 永久 blocked-only。精确边界见阅读顺序第 4 项 |
| 节点端点凭据与认证会话权威 | 服务端 v216-v218 已铺 endpoint credential/session、目标绑定 owner 重认证与单次消费；direct-TLS owner API、Windows DPAPI bootstrap、legacy no-downgrade 及 v13 auth-only WSS 均默认关闭。当前批次新增独立 v14 Planning bootstrap profile：固定单一 capability，在同一 exact endpoint generation 内只允许 sharing→preparation→Planning 六消息摘要链；每段观察与下一 intent 同事务重验耐久 session/credential并写 v219 provenance。NodeAgent 始终 `connected=false`，Planning 终点仍 `snapshot_ready=false`；无 signed Plan、work-admission、Runtime、Ready、route、outbox、Lease 或派发。源码尚未编译、测试、运行或执行迁移，状态仍为 `implementation_unwired`。见 `node-endpoint-session-authority.md` |
| Attempt Execution Plan / Gateway | v211-v214 已铺 sealed Plan、Provider-neutral Start、route/credential/actor authority、outbox 与 no-start recovery；v215 又在源码中形成 accepted observation→ACK→v185→actor/LeaseAuthority/commit→application 的 Store-local 原子闭包，并把既有 ACK-null cleanup pair 固定送往 quarantine。服务端及测试源码已编译；内存与临时文件 SQLite 的完整迁移、重复迁移、两次重开、关键对象及冲突 backfill 门卫已通过 4 项专项测试，但 accepted 成功闭包、生产数据库原位升级和生产链路未运行。旧人工激活/Renew/Abort 仍固定失败，可信输入、发送和远端证据 capability 无构造器，也无网络、worker、ACK ingress 或真实派发。见 `attempt-execution-plan-v1.md`、`attempt-execution-gateway-v1.md`、`attempt-delivery-outbox-v1.md` 与 `attempt-gateway-acceptance.md` |
| 节点本机 Attempt 执行合同 | Start / RenewLease / Cancel、Runner typed events 与 Host 盖章事件合同已写；它不是 Provider-neutral wire，尚未编译或接入云端协议 |
| 节点按需插件下载与通用任务执行 | 旧 LLM 已接入内部 Host seam，尚未编译；真实下载器、Sidecar/IPC、动态健康上报、通用任务派发和协议接线仍未实现 |
| 共享 CapacityPool 与追加式容量账本 | v165-v168 Supply/Claim 与 v173 Claim 历史已在 Supply、Offer 和 Broker 定向链中执行临时 SQLite 全量迁移；Broker 测试验证 held 与 release 回流。并发、到期批处理、生产磁盘和真实节点仍未验证 |
| Provider 与 Offer 版本注册表 | v169/v170 当前投影和追加式历史已随 Provider、Offer、Price Snapshot 与 Broker 定向链验证；进程内 HTTP/MCP 与 Offer 文件重开已验证，并发和生产磁盘仍未验证 |
| Price Snapshot 锁价控制面 | v171 Store/Service 已通过临时 SQLite 发布、幂等与审计专项；平台四眼 reference fallback v223/v224 已通过管理员 HTTP/MCP、原子 v171 Snapshot、拒绝零副作用、旧 TTL 触发器升级和文件重开专项，PC `/compute-reference-curves` 已通过跨层契约、严格类型、lint、生产构建和 bundle budget。两条来源都固定为 fallback_curve，不预留容量、不冻结余额，也不代表真实市场价格。真实 TCP、浏览器、并发压力、生产数据库副本与部署仍未验证，状态为 `implementation_partially_verified` |
| Provider Capacity Commitment v225 | `implementation_partially_verified`：immutable `committed` 主事实、唯一 `canceled|expired` terminal receipt、Store 原子 create/read/cancel/expire、generic bypass 与 owner/admin HTTP 已接线；数量/余额复用同一 Claim/ledger。生产目标、临时 SQLite 全量迁移、Store/Service/进程内 HTTP、磁盘重开和 PC `/compute-supply` 静态构建已通过；真实 TCP、生产升级、跨连接并发、浏览器及交付结算未验证 |
| Delivery Allocation v228/v234 | `design_frozen/implementation_partially_verified`：whole-only Grant/Exercise 已有原 3 项 Store/Service 证据；到期恢复又通过完整服务端测试目标编译、fresh/repeat migration 和 7 项管理员/Store/HTTP、worker、公平扫描专项，覆盖退款、容量归还、幂等、公平越过失败项、文件重开与下一 sweep 重试。真实并发 CAS、进程崩溃、历史库升级、真实 TCP 和生产运行仍未验证 |
| ComputeJob 版本注册表 | v172 Job 创建、候选发现、锁价、幂等、CAS 和依赖审计已随 Broker 组合链通过临时 SQLite 测试；项目级 HTTP/MCP、并发、生产磁盘和自动撮合仍未验证 |
| ComputeReservation 版本注册表 | v174 schema、Job/Offer/Price Snapshot/Claim 精确版本绑定、当前投影、不可变历史、消费者幂等、CAS、状态机、完整依赖审计及事务内登记入口已写；HTTP/MCP 可读取本人或当前项目的最新列表与详情，独立写入口不移动容量或资金，v175/v176 Broker 已组合调用 |
| 消费者余额预授权 | v175 Broker 将显式到期预授权与 Job/Claim/Reservation 在同一事务内编排，并要求结果为 `reserved` 且含余额结果；v176 可在 Attempt 尚未激活时按精确预授权 ID 严格退款。仅支持 `platform_balance_cny`，不覆盖运行中任务或实际用量结算 |
| Broker 原子 Reserve 与未执行任务终态 | v172-v176 已通过 2 项成功链/重放/回滚、1 项进程内 HTTP/MCP 身份与确认、2 项独立连接竞争及 1 项两次临时磁盘重开测试；服务端时间不再破坏终态重放。PC `/compute-market` 已静态生产构建；真实 TCP、高并发压力、生产数据库升级、异常断电恢复、派发与结算未验证，状态为 `implementation_partially_verified`。详见 `broker-control-plane-acceptance.md` |
| Attempt 已接受激活回执 | v185 状态推进内核仍负责原子激活 held Claim、reserved Job、active Reservation 与 staging Lease；v215 源码只在 sealed verified ACK 可用时同事务形成 exact application、actor receipt、Lease authority 与 commit outbox。服务端及测试源码已编译，内存/临时文件迁移、两次重开与冲突 backfill 门卫已验证；PC 已静态生产构建并关闭人工 Start。accepted 成功闭包、生产数据库原位升级和生产链路仍未运行，也无可信 ACK producer、生产 Adapter、节点执行或新增扣款 |
| Attempt Lease 状态与续租 | v186 的状态、续租 kernel 与历史读取仍在；Provider HTTP 人工续租写入口现固定 `COMPUTE_ATTEMPT_RENEW_GATEWAY_NOT_READY`，PC 已静态生产构建并禁用人工续租，避免未认证心跳绕过 durable Renew/fencing/recovery。读取仍可保留，真实 Renew 尚未实现 |
| staging Attempt 无用量安全中止 | v187 kernel 与历史读取仍在；Provider HTTP 人工“未执行”中止写入口现固定 `COMPUTE_ATTEMPT_ABORT_GATEWAY_NOT_READY`，PC 已静态生产构建并禁用人工中止。v214 cancel observation 只解锁 reconcile，仍不等于 no-start；只有 rejected 或 final reconcile+tombstone 的 exact authenticated proof 可解 v176，真实零用量补偿/Abort service-actor kernel 尚未实现 |
| running Attempt 累计声明用量 | v188、Provider HTTP 与 PC `/compute-execution` 已写；只读模板从当前合同返回 meter、上一累计值和下一序号，写入口只接受精确 running Lease、完整 meter 集合和不回退累计值，保存 `provider_declared` 与超额标记。v226 已通过 Rust/SQLite 动态验收，候选后只允许精确重放并拒绝新声明，双连接竞争不会产生落后流头。PC 已通过静态生产构建；真实 Gateway/Adapter、HTTP/TCP、浏览器和生产库仍未验证。它不改变状态、容量或资金，也不等于 verified usage |
| Attempt Provider 终态候选 | v189、追加式 Store、Provider HTTP 与 PC `/compute-execution` 已写；第一份候选必须绑定当前 running Lease、最新 v188 快照和服务端返回的 Workload 输出合同。v226 已通过 5 项 Rust/SQLite 动态测试，线性化“当前流头→唯一候选→声明流封口”，拒绝 legacy 漂移并令下游审核失败关闭。PC 已通过静态生产构建；真实 Gateway/Adapter、HTTP/TCP、浏览器和生产库仍未验证。候选不推进状态、不消费容量、不移动资金，也不等于 Execution Receipt |
| Attempt 消费者终态审核 | v190、追加式 Store、消费者 HTTP 与 PC `/compute-reviews` 已写；本人待审核队列按消费者过滤并排除已有审核，第一份 `accepted/rejected/disputed` 必须绑定精确 v189 候选。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。审核只登记消费者证据，不等于平台验证或结算 |
| Attempt 平台终态观测 | v191、追加式 Store、管理员 HTTP 与 PC `/compute-observations` 已写；待观测队列返回已审计候选和最终 Provider meter，管理员可登记完整平台 meter、结果与证据引用并保存差异。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。观测不等于 verified usage、可信终态或结算 |
| Attempt Verification 决定 | v192、追加式 Store、管理员 HTTP 与 PC `/compute-verification` 已写；待验证队列返回重新审计的 v189-v191 证据链，管理员按保守策略登记 verified/compensable usage。新增 native retained read 以 Lease-only 重审 v188-v192、返回 exact-52并允许三种决定，另有两条MCP与条件14等式；该增量仅source-review且不依赖v193。旧 PC 已通过静态生产构建，但不能外推给新增量；决定/read都不生成 Execution Receipt、不改状态和资金 |
| Attempt Execution Receipt | v193、追加式 Store、管理员 HTTP 与 PC `/compute-receipts` 已写；待签发队列只返回重新审计的 accepted Verification 与候选，管理员确认后固定执行身份、工件、用量和证明。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。回执不改状态、容量和资金 |
| Attempt 可信终态与容量收口 | v194、追加式 Store、管理员 HTTP 与 PC `/compute-finalization` 已写；待收口队列重新审计 v193 及当前状态并返回精确提交模板。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器、生产库和发布仍未验证。应用终态会推进状态和容量，但预授权和 Provider 收益不变 |
| Attempt 待结算回执 | v195、追加式 Store、独立 pending 账本、管理员 HTTP 与 PC `/compute-settlement-issuance` 已写；待办队列重审计资金来源并返回金额预览。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器、生产库和发布仍未验证。应用只结清平台内预授权并登记 pending，不是外部资金转移 |
| Attempt 结算挑战 | v196、追加式 Store、消费者待申诉队列、消费者/管理员 HTTP 与 PC `/compute-challenges` 已写；队列重审计本人 72 小时窗口、ledger 腿、pending 与释放门卫。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器、生产库和发布仍未验证。挑战只阻断未来 pending 释放，不退款、不裁决、不移动余额 |
| Attempt 结算生命周期历史 | v195-v199 角色 HTTP 与共享 PC 视图已写；消费者、Provider 和管理员分别读取本人、当前 Provider 与全局全部结算。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器、生产库和发布仍未验证。available 仅指内部余额且不证明外部付款 |
| Attempt 待结算原子释放 | v198、追加式 Store、Release Posting/账本腿与消费者/管理员 HTTP 已写；满 72 小时且挑战允许时，管理员可把内部 pending 原子转入 available。PC `/compute-settlement` 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。open/accepted 阻断，available 不等于提现或外部付款 |
| 到期结算释放队列与管理员批处理 | 管理员 HTTP 可按不透明 keyset 游标读取到期候选，再逐笔复用 v198；v202 保存批次意图和完成回执，PC 展示分页、当前页处理和批次历史。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。批次不是整批原子事务或后台定时器，不提现、不外部付款 |
| Attempt accepted 挑战纠正 | v199、追加式 Store、accepted 待纠正队列、Correction Posting/账本腿、角色 HTTP 与 PC `/compute-corrections` 已写；管理员以整数 fen/micros、守恒预览和双重确认提交平台内向下纠正。PC 已通过静态生产构建；操作级后端回归、真实 TCP、浏览器、生产库和发布仍未验证。纠正不等于外部退款到账 |
| Provider 提款申请与内部冻结 | v200 把 Provider 本人 CNY available 原子转入 withdrawn；与 v201 共用 Store/Service 专项 `3 passed / 0 failed`，PC 静态构建通过。只冻结内部余额，不执行外部付款；真实 TCP、浏览器和生产库未验证。见 [`当前状态`](current-implementation-status.md) 与 [`v200 合同`](settlement-withdrawal-request-api.md) |
| Provider 提款唯一终态 | v201 允许取消、拒绝或登记外部已付款声明；同组专项覆盖返还幂等、重开和声明零资金移动，PC 静态构建通过。它不发起或验证外部付款。见 [`当前状态`](current-implementation-status.md) 与 [`v201 合同`](settlement-withdrawal-terminal-api.md) |
| 结算账户审计视图与提款队列 | Provider 本人 HTTP 可从 v195、v198-v201 不可变账本重建账户和提款生命周期；管理员 HTTP 可重建固定平台账户并读取全局队列。PC 本人收益与管理员结算页面已通过静态生产构建；操作级后端回归、真实 TCP、浏览器和生产库仍未验证。视图不提供平台提款、不移动资金 |
| 外部算力池适配器与统一报价 | V273源码/迁移21/21但runtime未运行；V274/V277/V278为`source_written/source_review_only/implementation_uncompiled/implementation_unrun`、0/0。#13-#18 deny，Provider=`registering`、`eligible_rows=0`。V280六层ABI已冻；market/wire evidence、inventory/actual wire未选，source/runtime 0。见[`V280`](external-pool-service-managed-admission-runner-authority.md)、[`Gateway`](external-pool-service-managed-gateway-session-validator-abi-authority.md)、[`semantic registry`](external-pool-adapter-production-semantic-wire-profile-registry-abi-authority.md)与[`状态`](current-implementation-status.md) |
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

阶段目标与退出门槛见 [`implementation-roadmap.md`](implementation-roadmap.md)，瞬时成熟度见 [`current-implementation-status.md`](current-implementation-status.md)。以下按领域依赖继续阅读：

1. `docs/decisions/distributed-compute-federation-v1.md`：不可随意改变的架构决定。
2. `docs/distributed-compute/architecture.md`：Provider、控制面、数据面和任务状态。
3. `docs/distributed-compute/node-client-and-plugins.md`：客户端按需启用与插件边界。
4. `docs/distributed-compute/node-endpoint-session-authority.md`、`docs/distributed-compute/node-plugin-local-authority.md`、`docs/distributed-compute/node-plugin-manifest-catalog-authority.md`、`docs/distributed-compute/node-plugin-vfs-fault-authority.md`、`docs/distributed-compute/node-plugin-vfs-fault-acceptance.md`、`docs/distributed-compute/node-plugin-planning-snapshot-authority.md`、`docs/distributed-compute/node-ready-capability.md`、`docs/distributed-compute/node-plugin-candidate-cleanup.md` 与 `docs/distributed-compute/windows-compute-namespace-fence-wire-v1.json`：端点 currentness、SQLite 真源、目录/回滚、测试 VFS 故障及动态验收、Planning 投影、短期就绪、失败清理与 Windows hard-fence ABI。
5. `docs/decisions/distributed-compute-capacity-ledger-v1.md` 与 `docs/distributed-compute/capacity-ledger.md`：共享容量池、跨 Offer 防超卖和追加式容量账本。
6. `docs/distributed-compute/market-and-settlement.md`、`docs/distributed-compute/platform-reference-price-curve-authority.md`、`docs/distributed-compute/capacity-instrument-authority.md`、`docs/distributed-compute/capacity-commitment-authority.md` 与 `docs/distributed-compute/delivery-allocation-authority.md`：标准化 SKU、期货锁价、平台参考回退、v238 标准合约采用门、v225 容量承诺、v228 whole-only 交付授权和结算边界。
7. `docs/distributed-compute/provider-api.md`、`docs/distributed-compute/user-node-provider-binding-authority.md` 与 `docs/distributed-compute/user-node-provider-binding-acceptance.md`：Provider 本人登记、user-node 安装/同意 identity root、后续 current reproof 与信任边界。
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
26. `docs/distributed-compute/attempt-verification-api.md`、`docs/distributed-compute/attempt-verification-retained-read-authority.md` 与对应 acceptance：保守 Verification policy、native exact-52 retained read、historical v188-v192、条件14等式与非状态/非结算边界。
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
39. `docs/distributed-compute/external-pool-adapter-post-exec-supervisor-hardening-authority.md` 与对应 acceptance：V267 source/launch capsule、post-exec dumpable、Yama、policy V2、ancillary、cleanup 和历史 evidence 失效边界。
40. `docs/distributed-compute/external-pool-adapter-signed-runtime-compatibility-authority.md` 与对应 acceptance：V268 Profile V2、固定 catalog 约束下的 release-declared public fixture、private runner observation、V237 signed release evidence、五条 admin route 与 atomic activation 后置边界。
41. `docs/distributed-compute/external-pool-adapter-runtime-compatibility-signing-handoff-authority.md` 与对应 acceptance：V269 默认关闭的启动 cgroup custody、同步 private-run caller、管理员 courier signer payload、durable replay 与零新 schema/worker 边界。
42. `docs/distributed-compute/external-pool-adapter-provider-runtime-readiness-authority.md` 与对应 acceptance：V270 production custody、post-cleanup Provider-specific readiness、最长 15 秒 TTL、五条 owner/admin route 与 activation 后置边界。
43. `docs/distributed-compute/external-pool-route-source-projection-bridge-authority.md` 与对应 acceptance：V271 migration-only V221 logical identity→V249 Provider projection exact source bridge、0 fence 开放与 atomic activation NO-GO 边界。
44. `docs/distributed-compute/external-pool-adapter-task-protocol-conformance-authority.md` 与对应 acceptance：V272 Provider-neutral ELTP v1 controlled run、独立 process HMAC custody、14-root/8-exchange ABI、2 表 1 诊断 view、最长 15 秒 TTL、三条 admin route与零扩权边界。
45. `docs/distributed-compute/external-pool-adapter-task-protocol-production-authority.md` 与对应 acceptance：V273 默认关闭 dormant production transport/ingress、8项 production roots、ELTP wire复用、exact 6表、无公开 API与 `eligible_rows=0` 边界。
46. `docs/distributed-compute/external-pool-adapter-provider-active-successor-authority.md` 与对应 acceptance：V274 stable activation root、exact projected-active Provider、2张immutable表+1个非权威view、renewable active evidence与V277前零行边界。
47. `docs/distributed-compute/external-pool-adapter-stable-executor-atomic-activation-authority.md` 与对应 acceptance：V277 stable executor、双时间、1表/0view/0revocation、16+1原子闭包、9/9 fence与V278后置边界。
48. `docs/distributed-compute/external-pool-adapter-route-renewal-reachability-authority.md` 与对应 acceptance：V278 historical/current/runtime分型、1张77列immutable表/0view/0revocation、四UDF、renewal 11+1、outbound 2+1与market fences关闭下`eligible_rows=0`边界。
49. `docs/distributed-compute/external-pool-service-managed-admission-runner-authority.md` 与 `docs/distributed-compute/external-pool-service-managed-admission-runner-acceptance.md`：V280纵切架构；source/migration/fence 0。
50. `docs/distributed-compute/external-pool-service-managed-market-profile-authority.md` 与对应 acceptance：冻结V280 profile schema/canonical ABI、单attempt-slot结构、production transport常量与allocation/lease派生；初始产品载荷和inventory仍未选择，不能构造current authority。
51. `docs/distributed-compute/external-pool-service-managed-admission-receipt-abi-authority.md` 与对应 acceptance：冻结V280 receipt 6-key envelope、7组72个direct keys、单元素25-key bucket与planned 77列immutable schema；table、migration、UDF、trigger与source仍不存在。
52. `docs/distributed-compute/external-pool-service-managed-market-projection-identity-abi-authority.md` 与对应 acceptance：冻结V280 Pool/ledger/Offer/publication/v171 snapshot的deterministic identity、single checked-at与legacy owner digest不改域；writer、migration与production fence源码仍不存在。
53. `docs/distributed-compute/external-pool-service-managed-gateway-session-validator-abi-authority.md`、`docs/distributed-compute/external-pool-service-managed-gateway-session-validator-abi-acceptance.md`、`docs/distributed-compute/external-pool-adapter-production-semantic-wire-profile-registry-abi-authority.md` 与 `docs/distributed-compute/external-pool-adapter-production-semantic-wire-profile-registry-abi-acceptance.md`：冻结Gateway/session与semantic registry元ABI；actual profile/source/caller 0。
54. `docs/distributed-compute/external-pool-service-managed-market-profile-approval-evidence-abi-authority.md` 与对应 acceptance：冻结purpose-specific审批证据JCS/domain、distinct authenticated submitter/approver、无环Profile source映射与compiled exact replay；真实evidence与首个inventory仍未选择。

## 分阶段落地

长期阶段目标、顺序约束和退出门槛统一见 [`implementation-roadmap.md`](implementation-roadmap.md)。当前验证强度只在 [`current-implementation-status.md`](current-implementation-status.md) 更新，README 不再复制易过期的实现明细。

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
