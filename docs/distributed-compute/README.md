---
title: 一龙任务级分布式算力联邦
status: current
reviewed_at: 2026-08-09
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

| 能力 | 2026-08-05 状态 |
|---|---|
| 节点模型白名单、最大并发、每日 Token 预算与执行租约 | 已实现，是兼容供给入口 |
| Provider / Offer / Job / Reservation / Lease / Receipt 统一领域合同 | 基础代码、v169-v176 注册表、Broker Reserve/未执行任务终态、v185-v201 Attempt 激活、证据链、可信终态、容量收口、待结算、挑战、决议、纠正、pending 释放与 Provider 提款流程已写，状态为 `implementation_uncompiled`，尚未迁移、完整接线和运行验证 |
| Provider 本人控制面 | HTTP/MCP 已可登记、读取和列出本人 `user_node` 或 `managed_cluster` 的脱敏视图，PC 本人收益页已写入自助登记表单；只生成 `registering/self_declared` 记录，不接受路由、凭据、适配器或验证证据，尚未编译和运行验证 |
| CapacityPool 本人控制面 | HTTP/MCP 已可在本人 Provider 下登记、读取、列出和审计 `registering` Pool，并按稳定序号分页读取脱敏账本历史；PC `/compute-supply` 已写入列表、登记、审计健康和事务双分录分页源码。审计健康不等于硬件 verified，历史省略消费者和业务因果字段；尚未编译和运行验证 |
| CapacityBucket 本人控制面 | HTTP/MCP 已可在本人当前 Pool 版本下创建 open、零发行余额 Bucket，并读取当前余额；PC 已写入交付窗口登记和余额列表源码。窗口和 Bucket 摘要由服务端生成，不发行容量、不预留、不交易；尚未编译和运行验证 |
| Capacity Supply 本人控制面 | HTTP/MCP 已可显式确认后向同一窗口的多个 open Bucket 原子追加 self-declared 供给，或把尚在 available 的供给原子撤入 retired；PC 已写入单 Bucket 追加/撤出源码。服务端固定首次时间并复用现有双分录账本，available 不等于 verified 或可交易；尚未编译和运行验证 |
| 激活证据申请与计划控制面 | v177-v181、本人 HTTP/MCP、管理员审核/废止、申请/计划预检、不可变计划、原子应用与紧急隔离回执已写；v203 强制 prepared 激活计划第二人复核，v204 增加隔离恢复计划、第二人复核、预检和原子应用回执，v205 增加恢复计划追加式废止和重做入口。PC `/compute-supply` 已写入本人申请、历史、预检和取消，`/compute-activation` 已写入激活、隔离、恢复和恢复计划废止管理。恢复前旧 active Offer 必须先退场且不会自动重发；所有流程均不发送节点命令、不退款、不付款或移动资金，状态为 `implementation_uncompiled` |
| Offer 草稿、发布与生命周期控制面 | HTTP/MCP 已可创建、精确修订和撤销本人 draft Offer；管理员 HTTP 可原子发布 active、转为 draining，并在无 pending/active Reservation 时终结。v182-v184 保存追加式回执和依赖索引，所有写入口均不移动资金，状态为 `implementation_uncompiled` |
| 节点插件治理合同 | Signed Manifest/InstallPlan、双 keyring、authority v3/PlanApply、受管取数、候选验证、ZIP 安全物化及 staging/健康/quarantine Store 均已写。显式共享策略 ACK 后，云端已在同一认证 session 内串行下发 v10 preparation 与 v11 Planning Snapshot V2，并由 v209/v210 追加式账本保存 exact observation；节点凭据/account 变化会先推进不可丢 epoch并撤销旧 Bootstrap custody。该链仍默认阻断：节点固定 `context_ready=false`、`snapshot_ready=false`，generation outcome 仅允许 `signer_unavailable/rejected`，不会伪造 signed Plan。本机 authority v3-v6 已通过 8 项版本链、4 项 policy-binding、5+6 项 v5 Store/恢复判定、3+7 项 v6 binding Store/恢复判定、5 项真实 Ed25519 目录候选、15项SQLite纯策略及one-shot registry共34项测试；进程包装已有OS随机nonce类型、碰撞重试、互斥精确路由与RAII callback lease，私有file-custody把真实main、journal/WAL sidecar或WAL-main+SHM句柄与exact route和lease不可拆地绑定。受控操作门面要求offset I/O、main锁、SHM map/lock/barrier/unmap及close先取得exact callback lease，不暴露文件句柄或SHM裸地址；4项file-custody Windows测试已覆盖rollback/WAL真实关闭、路由操作与临时目录清理，另有process-owner测试验证失败custody先于route隔离永久保留。通用文件回执不能绕过main锁域专用关闭，但该状态仍不可由惰性ABI构造，没有生产实例、live `sqlite3_file`、raw state协议、SQLite ABI callback或VFS注册。prepared work及v5/v6内层恢复证据已验证，但外层bind/adopt生产接管、生产磁盘迁移及Host接线仍未验证。这不撤销 Ready/Attempt，也不让旧安装在新授权下自动恢复；句柄型 authority open、真实 planning snapshot producer、signed reauthorization、独立 work-admission epoch、生产 root/keyring、Signer/KMS 与可信时间仍缺。失败候选已有 cleanup authorization Store，其后 sealed topology 与首对象 intent/disposition/absence/namespace-durability 共五个独立 typed Store；sequence 1–4 已通过 32 项清理测试，minifilter wire 与首方特权组件 shape 分别通过 9 项和 5 项测试。真实 WDK 驱动、首方/Windows Catalog 验签、安装/transport、安全 fence 构造器、evidence v2、后续 ordinal、terminal journal、跨重启物理恢复、完整事务夹具与 Host 均缺，因此当前仍不产生生产可达的 completion/retry、installed、ready 或商业 Verification；HTTPS downloader、Sidecar/IPC/探针、installed/promotion、ReadyCapability 和 Attempt 接线也仍缺 |
| Attempt Execution Plan / Gateway | v211 Provider-neutral Start、endpoint/server-adapter 路由、远端 ACK 首次盖章重放及 ACK→v185→application 单事务账本已写；v212 又补充 capability/ArtifactAccess receipt、数值 ResourceGrant、不可变 plan/seal producer 与 command exact 门。旧人工激活入口失败关闭，所有可信输入与 Start/ACK capability 仍无构造器；无真实派发、ACK ingress、节点/矿池 Adapter 或恢复 worker，尚未编译或执行迁移。见 `attempt-execution-plan-v1.md`、`attempt-execution-gateway-v1.md` |
| 节点本机 Attempt 执行合同 | Start / RenewLease / Cancel、Runner typed events 与 Host 盖章事件合同已写；它不是 Provider-neutral wire，尚未编译或接入云端协议 |
| 节点按需插件下载与通用任务执行 | 旧 LLM 已接入内部 Host seam，尚未编译；真实下载器、Sidecar/IPC、动态健康上报、通用任务派发和协议接线仍未实现 |
| 共享 CapacityPool 与追加式容量账本 | 领域合同、v165-v168 schema、隔离 Store、Store-canonical Supply/Claim 请求摘要、事务内 Claim kernel、只读审计、到期批处理、状态门卫和 epoch 轮换已写；v173 追加 Claim 完整历史，Hold V2 固定 causal binding，Reservation Claim 强制绑定 Offer/Job/Reservation，Finish 继承原 held 绑定；尚未编译、执行迁移或接线 |
| Provider 与 Offer 版本注册表 | v169/v170 schema、Provider/Offer 当前投影、追加式历史版本、规范摘要和容量引用审计已写；尚未编译、执行迁移或接线 |
| Price Snapshot 锁价控制面 | v171 不可变 Registry 及本人 HTTP/MCP 已写；PC `/compute-supply` 已写入报价历史和 active Offer 显式发布入口，可生成规范化 fallback_curve 快照。不预留容量、不冻结余额，也不代表真实市场价格，状态为 `implementation_uncompiled` |
| ComputeJob 版本注册表 | v172 schema、Workload/范围/预算合同校验、当前投影、不可变历史、幂等、CAS、状态机和依赖审计已写；项目级 HTTP/MCP 可创建 Job、发现并绑定 Offer/Price Snapshot，v175/v176 Broker 已组合写入；尚未编译、执行迁移或接入自动撮合 |
| ComputeReservation 版本注册表 | v174 schema、Job/Offer/Price Snapshot/Claim 精确版本绑定、当前投影、不可变历史、消费者幂等、CAS、状态机、完整依赖审计及事务内登记入口已写；HTTP/MCP 可读取本人或当前项目的最新列表与详情，独立写入口不移动容量或资金，v175/v176 Broker 已组合调用 |
| 消费者余额预授权 | v175 Broker 将显式到期预授权与 Job/Claim/Reservation 在同一事务内编排，并要求结果为 `reserved` 且含余额结果；v176 可在 Attempt 尚未激活时按精确预授权 ID 严格退款。仅支持 `platform_balance_cny`，不覆盖运行中任务或实际用量结算 |
| Broker 原子 Reserve 与未执行任务终态 | v175/v176 schema、不可变回执、严格请求重放与历史绑定审计已写；Reserve 单事务完成预算、容量、Reservation 和 Job，Finish 单事务完成退款、held Claim Release/Expire 与 Job/Reservation 终态。v211 后，只要 Reservation 存在未 ACK 或 accepted/quarantined Start，Finish 就失败关闭，须等待明确 rejected 或未来 cancel/no-start 证明。HTTP/MCP 与 PC `/compute-market` 源码已接线；状态为 `implementation_uncompiled`，尚未迁移、构建或运行验证 |
| Attempt 已接受激活回执 | v185 状态推进内核仍负责原子激活 held Claim、reserved Job、active Reservation 与 staging Lease；v211 安装反向 trigger 后只允许 exact provisional Adapter ACK 在同一事务调用。旧 Provider HTTP/PC 人工确认写入口现固定失败，读取可保留；无生产 Adapter、节点执行或新增扣款，尚未编译、迁移或运行验证 |
| Attempt Lease 状态与续租 | v186、Provider HTTP 与 PC `/compute-execution` 已写；本人 Provider 可按更新时间列出当前 Lease，再按 Lease ID 读取状态，并在精确 revision/digest/fencing 栅栏下登记外部心跳声明、延长软期限。列表只读且逐条审计；续租不验证心跳签名、不发送节点命令、不改变容量或资金，尚未编译、执行迁移或运行验证 |
| staging Attempt 无用量安全中止 | v187、Provider HTTP 与 PC `/compute-execution` 已写；仅当前 revision 1、无心跳的 staging Lease 可在显式无执行声明下单事务全额退款、归还 active Claim、终结 Job/Reservation/Lease。它不发送取消命令、不验证外部中止证明，尚未编译、执行迁移或运行验证 |
| running Attempt 累计声明用量 | v188、Provider HTTP 与 PC `/compute-execution` 已写；只读模板从当前合同返回 meter、上一累计值和下一序号，写入口只接受精确 running Lease、完整 meter 集合和不回退累计值，保存 `provider_declared` 与超额标记。它不改变状态、容量或资金，也不等于 verified usage，尚未编译、执行迁移或运行验证 |
| Attempt Provider 终态候选 | v189、追加式 Store、Provider HTTP 与 PC `/compute-execution` 已写；第一份候选必须绑定当前 running Lease、最新 v188 快照和服务端返回的 Workload 输出合同。页面支持 succeeded 工件或 failed/canceled 原因，但不推进状态、不消费容量、不移动资金，也不等于 Execution Receipt，尚未编译、执行迁移或运行验证 |
| Attempt 消费者终态审核 | v190、追加式 Store、消费者 HTTP 与 PC `/compute-reviews` 已写；本人待审核队列按消费者过滤并排除已有审核，第一份 `accepted/rejected/disputed` 必须绑定精确 v189 候选。页面只登记引用和消费者证据，接受仍不等于平台验证或结算，尚未编译、执行迁移或运行验证 |
| Attempt 平台终态观测 | v191、追加式 Store、管理员 HTTP 与 PC `/compute-observations` 已写；待观测队列返回已审计候选和最终 Provider meter，管理员可登记完整平台 meter、结果与证据引用并保存差异。观测仍不等于 verified usage、可信终态或结算，尚未编译、执行迁移或运行验证 |
| Attempt Verification 决定 | v192、追加式 Store、管理员 HTTP 与 PC `/compute-verification` 已写；待验证队列返回重新审计的 v189-v191 证据链，管理员按保守策略登记 verified/compensable usage。决定不生成 Execution Receipt、不改状态和资金，尚未编译、执行迁移或运行验证 |
| Attempt Execution Receipt | v193、追加式 Store、管理员 HTTP 与 PC `/compute-receipts` 已写；待签发队列只返回重新审计的 accepted Verification 与候选，管理员确认后固定执行身份、工件、用量和证明。回执不改状态、容量和资金，尚未编译、执行迁移或运行验证 |
| Attempt 可信终态与容量收口 | v194、追加式 Store、管理员 HTTP 与 PC `/compute-finalization` 已写；待收口队列重新审计 v193 及当前状态并返回精确提交模板，管理员逐笔确认后才在单事务推进 Lease/Job/Reservation/Claim，并按 meter mode 消费或归还容量。预授权和 Provider 收益不变，尚未编译、执行迁移、接口联调、页面验收或发布 |
| Attempt 待结算回执 | v195、追加式 Store、独立 pending 账本、管理员 HTTP 与 PC `/compute-settlement-issuance` 已写；待办队列重审计完整资金来源并复用权威价格函数返回精确模板和金额预览，管理员逐笔确认后才在单事务结清消费者 CNY 预授权、退款、登记 Provider/平台 pending 收益并推进 Job settled。pending 不可提现，不是外部资金转移，尚未编译、执行迁移、接口联调、页面验收或发布 |
| Attempt 结算挑战 | v196、追加式 Store、消费者待申诉队列、消费者/管理员 HTTP 与 PC `/compute-challenges` 已写；队列只返回本人仍在固定 72 小时窗口内且未挑战、未释放的 v195 回执，并逐条重审计消费者 ledger 腿、pending 状态和释放门卫。消费者可提交一份不可覆盖的挑战；它只阻断未来 pending 释放，不退款、不裁决、不移动余额，尚未编译、执行迁移、接口联调、页面验收或发布 |
| Attempt 结算生命周期历史 | v195-v199 角色 HTTP 与共享 PC 视图已写；消费者、Provider 和管理员分别读取本人、当前 Provider 与全局全部结算。正常路径重审计 Settlement/Release，申诉路径复用 Challenge/Resolution/Correction/Release 证据链；available 仅指内部余额且不证明外部付款，尚未编译、执行迁移、接口联调、页面验收或发布 |
| Attempt 待结算原子释放 | v198、追加式 Store、Release Posting/账本腿与消费者/管理员 HTTP 已写；满 72 小时且挑战为 none/rejected/withdrawn/accepted_corrected 时，管理员可把 Provider/平台原金额或纠正净额从 pending 原子转入 available。open/accepted 阻断，available 不等于提现或外部付款，尚未编译、执行迁移或运行验证 |
| 到期结算释放队列与管理员批处理 | 管理员 HTTP 可按不透明 keyset 游标读取已满 72 小时且尚无 v198 Release 的候选与总数，再逐笔复用 v198；v202 追加保存批次意图和完成回执，幂等重试复用原候选页，PC 展示分页、当前页处理和批次历史。`incomplete` 不代表此前没有单笔释放；它不是整批原子事务或后台定时器，不提现、不外部付款，尚未编译、迁移或运行验证 |
| Attempt accepted 挑战纠正 | v199、追加式 Store、accepted 待纠正队列、Correction Posting/账本腿、角色 HTTP 与 PC `/compute-corrections` 已写；候选逐条重审计 v195-v197 并排除已有 Correction/Release。管理员以整数 fen/micros、守恒预览和双重确认提交向下金额纠正，原子退款消费者平台内余额并冲减 Provider/平台 pending；纠正后 v198 只释放净额，但不等于外部退款到账，尚未编译、执行迁移、接口联调、页面验收或发布 |
| Provider 提款申请与内部冻结 | v200、追加式 Store、Withdrawal Request Posting/账本腿与 Provider 本人 HTTP 已写；从当前 Provider 回执校验所有权和结算账户，把 CNY available 原子转入 withdrawn 保留区。它只冻结内部余额，不执行或证明外部付款，尚未编译、执行迁移或运行验证 |
| Provider 提款唯一终态 | v201、追加式 Store、Terminal Posting/账本腿与 Provider/管理员 HTTP 已写；取消或拒绝会全额返还 withdrawn，外部已付款声明只保存证据引用和摘要且不移动余额。PC 管理页已接入拒绝与外部已付款证明登记源码。它不发起或验证外部付款，尚未编译、执行迁移、运行或页面验证 |
| 结算账户审计视图与提款队列 | Provider 本人 HTTP 可从 v195、v198-v201 不可变账本重建账户和提款生命周期，并按 Provider/状态读取本人队列；管理员 HTTP 可重建固定平台账户的 pending/available，并按状态读取全局队列。PC 已写入本人收益与管理员结算两套页面源码。视图和队列只读、不提供平台提款、不移动资金，尚未编译或运行验证 |
| 外部算力池适配器与统一报价 | 已接受设计，尚未实现 |
| 多源验证、期货曲线与真实结算 | 已接受设计，尚未实现 |
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
4. `docs/distributed-compute/node-plugin-local-authority.md`、`docs/distributed-compute/node-plugin-manifest-catalog-authority.md`、`docs/distributed-compute/node-ready-capability.md`、`docs/distributed-compute/node-plugin-candidate-cleanup.md` 与 `docs/distributed-compute/windows-compute-namespace-fence-wire-v1.json`：节点 SQLite 真源、签名目录与回滚边界、短期技术就绪、失败候选清理边界与首个 Windows hard-fence 固定 wire ABI。
5. `docs/decisions/distributed-compute-capacity-ledger-v1.md` 与 `docs/distributed-compute/capacity-ledger.md`：共享容量池、跨 Offer 防超卖和追加式容量账本。
6. `docs/distributed-compute/market-and-settlement.md`：标准化 SKU、期货锁价和结算回执。
7. `docs/distributed-compute/provider-api.md`：Provider 本人登记、查询和信任边界。
8. `docs/distributed-compute/capacity-pool-api.md`：本人共享物理资源边界及摘要隐私合同。
9. `docs/distributed-compute/capacity-bucket-api.md`：交付窗口 Bucket 登记、余额读取和窗口不变量。
10. `docs/distributed-compute/capacity-supply-api.md`：本人供给追加、撤回、幂等和信任边界。
11. `docs/distributed-compute/activation-evidence-api.md`：证据申请、人工审核、版本复核和“批准不等于激活”边界。
12. `docs/distributed-compute/activation-recovery-api.md`：隔离恢复计划、第二人复核、显式废止重做、旧 Offer 退场门卫和追加式恢复边界。
13. `docs/distributed-compute/offer-api.md`：Offer 本人规范化草稿、管理员发布、安全退场与资金边界。
14. `docs/distributed-compute/price-snapshot-api.md`：Offer 派生 fallback_curve 报价、候选效果与无资金效果边界。
15. `docs/distributed-compute/broker-api.md`：Job、报价与预留 HTTP/MCP 控制面。
16. `docs/distributed-compute/attempt-execution-plan-v1.md`：可信 capability、ArtifactAccess、数值 ResourceGrant、不可变 Plan 与 v211 exact 门。
17. `docs/distributed-compute/attempt-execution-gateway-v1.md`：Provider-neutral Start、Adapter ACK、本地原子激活与 provisional 远端边界。
18. `docs/distributed-compute/attempt-activation-api.md`：v185 激活内核、fencing、原子状态变化与无节点命令效果边界。
19. `docs/distributed-compute/attempt-lease-api.md`：Lease 状态投影、受控续租、过期不可复活与无执行效果边界。
20. `docs/distributed-compute/attempt-abort-api.md`：staging 无用量中止、容量归还、退款和外部声明边界。
21. `docs/distributed-compute/attempt-usage-api.md`：running Attempt 累计声明用量、单调性、超额标记与无结算效果边界。
22. `docs/distributed-compute/attempt-terminal-candidate-api.md`：Provider 首次终态候选、输出合同、不可覆盖与无状态/无资金效果边界。
23. `docs/distributed-compute/attempt-consumer-review-api.md`：消费者首次终态审核、证据引用、不可覆盖与非验证/非结算边界。
24. `docs/distributed-compute/attempt-platform-observation-api.md`：平台首次终态观测、累计 meter 差异、不可覆盖与非验证/非结算边界。
25. `docs/distributed-compute/attempt-verification-api.md`：保守 Verification policy、verified/compensable usage、不可覆盖与非状态/非结算边界。
26. `docs/distributed-compute/attempt-execution-receipt-api.md`：accepted Verification 的执行回执、完整源证据重审计与非状态/非结算边界。
27. `docs/distributed-compute/attempt-finalization-api.md`：精确 Execution Receipt 的可信终态、容量消费/归还与资金不变边界。
28. `docs/distributed-compute/attempt-settlement-api.md`：CNY 双价格腿、消费者预授权结清与 Provider pending 收益边界。
29. `docs/distributed-compute/attempt-settlement-challenge-api.md`：72 小时消费者挑战、不可覆盖记录与无余额移动边界。
30. `docs/distributed-compute/attempt-settlement-challenge-resolution-api.md`：消费者撤回、管理员裁决与释放门卫边界。
31. `docs/distributed-compute/attempt-settlement-correction-api.md`：accepted 挑战向下金额纠正、消费者退款和 pending 冲减边界。
32. `docs/distributed-compute/attempt-settlement-release-api.md`：72 小时后纠正净额从 pending 到 available 的原子释放、账本与非提现边界。
33. `docs/distributed-compute/settlement-withdrawal-request-api.md`：Provider available 提款申请、withdrawn 内部冻结与非付款边界。
34. `docs/distributed-compute/settlement-withdrawal-terminal-api.md`：提款取消、拒绝、外部已付款声明和唯一终态边界。
35. `docs/distributed-compute/settlement-account-view-api.md`：Provider 结算账户账本重建与管理员提款队列边界。
36. `docs/distributed-compute/settlement-release-batch-api.md`：到期候选、逐笔 v198 释放、部分失败报告与非后台自动化边界。
37. 现有兼容实现：`docs/decisions/node-compute-sharing-supply-v1.md`。

## 分阶段落地

### F0：统一语言和合同

版本化的 Provider、Offer、Workload、Job、Reservation、Attempt Lease、Execution Receipt、Settlement Receipt 和 Price Snapshot 基础合同已经写入代码。节点侧还形成了带 `fencing_generation` 的 Start / RenewLease / Cancel Attempt 命令、Runner typed events 和 Host 盖章事件合同；这些代码均尚未编译、接线或运行验证。现有 `LlmStreamRequest` 继续工作，不在首批协议变更中制造强制升级。

### F1：用户节点成为可插拔 Provider

节点内部已经形成 Plugin Host 兼容 seam、Signed Manifest、InstallPlan、双槽安装/切换/回滚 lifecycle 和 ReadyCapability 合同骨架，并写入真实 Ed25519/JCS 验证、Manifest 语义校验及本机 InstallPlan 准入内核；节点供应链源码已随 `elon-pc-node` 编译通过，但未启动或生产接线。云端还形成 v169 版本化 Provider Registry、v170 追加式 Offer Registry及 v182-v184 发布/生命周期控制；本人可创建规范化 draft Offer，平台管理员可发布 active、转为 draining，并在无活动预留时终结为 expired/revoked。云端入口尚未编译、执行迁移或运行验证。ReadyCapability 只是有明确过期时间的本机技术就绪证据，不包含市场价格、可预留容量或账户授权，**不等于 Compute Offer**。

目标流程仍是：共享关闭时不下载重型组件；开启后按硬件和任务选择签名插件、运行时与模型工件；只有用户开启并确需 cleanup hard-fence 时，才按独立首方特权组件合同下载并经 UAC 安装驱动，普通插件签名无权授权内核代码。本机耐久真源已选定为独立 SQLite，根签名双 keyring、原子计划应用、候选所有权、三段式下载认领、候选级全工件验证和可信时间边界见 `docs/distributed-compute/node-plugin-local-authority.md`。raw verified 只把本机槽推进到 `verifying`；staged 候选可经健康失败 quarantine 进入 `failed`，再进入 cleanup authorization。固定句柄旧执行器和 completion Store 内核已编译；中间路线已能封存 topology 并写入首对象 intent、disposition、absence 与 namespace durability，sequence 1–4 的 32 项清理测试均已通过，但 sequence 3/4 只覆盖 builder 链和 SQLite exact-row。mutation fence 是 exact scope/authority-bound 的线性租约；其签名 minifilter wire/供应链 shape 已通过 9+5 项独立合同测试，但真实驱动、首方信任/fingerprint、Windows catalog、安装、transport、safe constructor、evidence v2 与显式 release 仍未实现，后续对象/terminal producer 也缺。只有完整 journal exact readback 后返回的不透明终态能力才能进入 completion，并恢复为 `NotCreated` 或 exact `Completed`。因此当前完整清理链仍不可达，不会因内存物理证据或首对象完成直接释放 owner。候选观察值仍由 Host 调用方提供，尚未证明真实 Sidecar 已运行；生产时间权威、真实下载器、Sidecar/IPC/探针调度、完整清理事务夹具、跨重启恢复、Host 接线、installed/promotion、云端 capability gate 和通用 Attempt 协议仍未实现。

NodeRuntime 已挂载默认关闭的 Compute Bootstrap，只派生 installation/data-root/authority 身份与路径并绑定节点状态目录实例锁 weak witness；该 witness 只作为进程存活前提，不能替代工件根锁。现有 `pin_compute_plugin_root()` 已能返回同时持有 canonical pinned root 与 share-none 锁句柄的非 Clone capability，但 Bootstrap 尚无 sharing-on transition，因此管理状态仍如实显示根锁未取得，生产 trusted-time、生产回滚见证、root pin、authority open 和 process fence 也不可用。数据根变化要求重启；默认关闭不会打开数据库、执行下载或启动 Sidecar。后续 Runner/节点 Attempt 持久化将使用 schema v4 的独立执行 fence 域，不把高频 run 状态塞入 inventory，也不复用下载 cancellation。

### F2：Broker、验证和真实结算

共享 CapacityPool 与追加式容量账本已形成 checked-i128 reducer、v165-v168 schema 和隔离 Store：多 meter 发行/撤出、Claim hold/release/expire 均保存 causal binding，Reservation Claim 绑定 Offer、Job 与同主体 Reservation；公开 standalone 方法拥有事务，组合 kernel 不自行提交。v169-v174 形成 Provider、Offer、Price Snapshot、Job、Claim 历史与 Reservation Registry；v175 在一个 `BEGIN IMMEDIATE` 中组合余额预授权、Claim Hold、Reservation 与 Job 并保存不可变回执，任何依赖或资金步骤失败均整体回滚。v176 仅对未出现未解决 Start 的 Reservation 执行 Release/Expire；存在无 ACK、accepted 或 quarantined command 时不得退款或归还容量，需未来 durable cancel/no-start proof。

v185 保留唯一 Attempt 激活状态推进 kernel：单事务把 Claim `held -> active`、Job `reserved -> running`，更新 Reservation 并保存 staging Lease/回执。v211 已关闭 Provider 所有者人工确认激活入口，新 activation 只能由不可构造的 sealed Gateway capability 把 exact Adapter ACK、v185 与 application 同事务提交；Offer `draining` 仍按 Reservation 历史版本履约，Provider 当前 route 每次重验。v186 的人工心跳续租和 v187 的无用量中止仍是旧路径；真实 Adapter 启用前必须失败关闭，待 durable Renew/Cancel command、认证 ACK、fencing 和恢复账本落地。v185-v187 不发送节点命令，也不验证外部接受、心跳或中止证明。

登录用户 HTTP/MCP 可读本人 Job/Reservation 并发起 Reserve、Release、Expire；旧 Attempt 激活 POST 已稳定失败，历史参与方仍可读激活回执与 Lease。上述路径仅支持 `platform_balance_cny`，均为 `implementation_uncompiled`，未执行迁移、接口/并发验证、真实派发、超时归还或实际用量结算，不能视为完整算力交易系统。

v187 继续补齐最窄的 staging 无用量中止：只有激活回执对应的首版、无心跳 Lease 才能由 Provider 所有者显式声明外部执行器未开始执行，并在一个事务内全额退回预授权、把 active Claim 归还 available、推进 Job/Reservation/Lease 终态并保存追加式回执。它不验证 `executor_abort_ref`、不发送取消命令，也不覆盖已开始执行、部分扣费、自动超时、调度重试或最终结算，状态仍为 `implementation_uncompiled`。

v188 再补齐 running Attempt 的累计声明用量证据：Provider 所有者只能在当前 Lease 精确 revision/digest/fencing 下追加完整 meter 快照，序号严格递增、累计值不得回退；高于预留合同的 meter 被保留并标记为 overage。回执明确为 `unverified_provider_declaration`，该阶段不更新 Lease/Job/Reservation/Claim，不消费容量、不扣款，也不产生 Provider 收益。后续 v191-v194 已分别写入平台观测、Verification、Execution Receipt 与可信终态，但真实 Host 事件接线和结算仍未实现。

v189 再保存 Provider 首次终态候选：当前 running Lease 必须已有最新 v188 用量快照，且 Lease、Job、Reservation、Claim 版本和摘要完全一致；`succeeded` 结果按 Workload 输出合同校验，`failed/canceled` 不得携带伪最终产物。候选只保存为 `unverified_provider_declaration`，不更新状态、容量或资金，也不等于 Execution Receipt。

v190 再保存消费者第一份终态审核证据：只有 v189 候选绑定的 Job 消费者可提交 `accepted/rejected/disputed`，并固定候选事件摘要和完整因果链；拒绝或争议必须提供证据引用。该记录固定为 `consumer_attestation_only`，不产生 Verification 决定，不更新状态、容量或资金，也不会因消费者接受而自动付款。

v191 再保存平台第一份终态观测证据：平台 `admin/owner` 可登记 control plane、transport gateway 或 server metering 的完整累计 meter，并固定与最终 v188 快照不同的 meter。该记录固定为 `unverified_platform_observation`；差异或一致都不产生 Verification 决定，也不更新状态、容量或资金。

v192 首次保存平台 Verification 决定：管理员精确绑定 v189-v191 后，`conservative_min_v1` 仅在消费者接受且 Provider/平台 outcome 一致时允许 accepted；verified usage 逐 meter 取双方较小值，compensable usage 再受 Reservation 预留上限约束。rejected/disputed 记录零用量。回执不可覆盖，但不生成 Execution Receipt、不推进状态、不消费容量、不移动资金。

v193 再基于 accepted v192 签发 Execution Receipt：回执重新审计 Attempt 激活、Job/Reservation 历史及 v188-v192，固定 executor、Offer、runner/plugin/model、输入输出、四类用量、三方证明和 Verification。回执不可覆盖，但不推进 Lease/Job、不消费 Claim/Reservation、不移动资金。

v194 再基于由 accepted Verification 签发的精确 v193 Execution Receipt 应用可信终态：单一事务把 Lease 推进为 terminal、Job 推进为 `verification_pending`、Reservation/Claim 推进为 consumed；consumable meter 消费 compensable usage 并归还余量，reusable meter 全量归还。回执不可覆盖，预授权与 Provider 收益仍不变。

v195 再基于精确 v194/v193、Broker 预授权和 Price Snapshot 生成不可变 Settlement Receipt：消费者价格腿使用 verified usage 并按快照舍入到人民币分，Provider 价格腿使用 compensable usage；单事务扣结预授权、退回未用余额、登记 Provider/平台 pending 收益并把 Job 推进为 `settled`。首版仅支持 CNY 基础组件，pending 不可提现，不调用真实支付或链上网络。v196 允许消费者在回执创建后的固定 72 小时内提交一份不可覆盖挑战；v197 再把撤回、接受或驳回保存为唯一终态。两者都不改写结算或余额。v199 对 accepted 挑战追加向下金额纠正，原子退款消费者并冲减 Provider/平台 pending；v198 在 72 小时窗口结束且挑战门卫允许时，用独立 Release Receipt 和四条账本腿把原金额或纠正净额从 pending 原子转入 available。管理员现可读取有界到期候选并逐笔复用 v198；这是人工触发的部分成功批处理，不是后台定时清算。v200 再允许 Provider 所有者把本人 available 原子转入 withdrawn 提款保留区；v201 为申请增加取消、拒绝或外部已付款声明的唯一终态。取消/拒绝返还内部余额，付款声明只保存证据，不调用或验证外部资金网络。
Offer 所有者 HTTP/MCP 可发布服务端规范化的 fallback_curve Price Snapshot；项目级 HTTP/MCP 可创建 submitted Job、发现当前有效候选，再把当前 revision/digest 锁定到所选报价。候选返回价格合同和最小 Provider 摘要，不返回节点路由、凭据或适配器配置。报价发布、候选发现和锁价均不移动资金或容量；真实价格源、批量报价和自动撮合仍未实现。

Provider 本人控制面由 `docs/distributed-compute/provider-api.md` 维护，Pool、Bucket 和 Supply 控制面分别由 `docs/distributed-compute/capacity-pool-api.md`、`docs/distributed-compute/capacity-bucket-api.md`、`docs/distributed-compute/capacity-supply-api.md` 维护；激活证据申请、计划、第二人复核与“内部激活不等于市场可交易”边界由 `docs/distributed-compute/activation-evidence-api.md` 维护，隔离后的追加式恢复由 `docs/distributed-compute/activation-recovery-api.md` 维护。PC `/compute-supply` 承载本人供给、证据和 Offer 草稿入口，仅管理员可见的 `/compute-activation` 承载审核、计划准备、第二人复核、应用、废止、隔离和恢复，`/compute-offers` 承载 Offer 发布和安全退场，所有登录用户可见的 `/compute-market` 承载本人 Job、锁价和预留，`/compute-execution` 承载本人 Provider 的 Attempt 激活、Lease、用量与终态候选，`/compute-reviews` 承载消费者待验收队列和第一份审核证据，仅管理员可见的 `/compute-observations` 承载待观测队列和第一份平台证据，`/compute-verification` 承载待验证证据链、保守计量预览和第一份人工决定，`/compute-receipts` 承载 Execution Receipt 确认，`/compute-finalization` 承载可信终态确认，`/compute-settlement-issuance` 承载首份待结算回执确认；这些入口当前均为 `implementation_uncompiled`。Offer 规范化草稿与无市场效果边界由 `docs/distributed-compute/offer-api.md` 维护；Job、报价和预留控制面由 `docs/distributed-compute/broker-api.md` 维护；Attempt 激活、续租、中止、声明用量、Provider 候选、消费者审核、平台观测、Verification、Execution Receipt、可信终态、待结算、挑战、决议、纠正与释放边界见本页“阅读顺序”第 16 至 30 项。

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
