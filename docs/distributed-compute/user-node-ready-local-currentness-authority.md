---
title: UserNode Ready 本机当前性封印 V1 权威草案
status: draft
reviewed_at: 2026-08-30
owners: node, compute
proposed_feature_id: compute-user-node-ready-local-currentness-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_draft_uncompiled
verification_status: source_review_only
---

# UserNode Ready 本机当前性封印 V1 权威草案

## 1. 目标与适用边界

本合同只关闭 `ProjectedComputeUserNodeReadySourceLineageV1` 四个缺口中的第一个：在一份已经由
handle-bound SQLite VFS 打开的本机权威连接上，用单一只读事务证明 work-admission、共享授权、Ready inventory
revision/policy 与 exact plugin record
仍是 exact current。成功只在模块私有 prover 内产生 transaction-scoped、private-field、non-Clone、non-Serde、
non-Send/non-Sync 的
`CurrentComputeUserNodeReadySourceLineageSeal`。

该 seal 不是 `ComputeReadyCapability`、Host runtime authority、Runtime transition receipt、节点签名或 v15
authenticated session。它不关闭另外三项缺口，也不生成 Provider、route、Offer、Job、Attempt、Lease、Receipt、
计量、结算或资金效果。

## 2. 输入必须全部来自既有 owner

入口必须同时借用：

1. `OpenedComputePluginLocalAuthority`：只能由未来 handle-bound SQLite VFS 消费 pinned root/controller custody 后形成；
2. `ComputePluginFetchProcessFence`：证明当前进程 owner lifetime，且 authority instance、installation 和 process epoch
   必须与 opened authority 及 Store 一致；
3. 一份严格晚于原 Ready observation 的 authenticated `ComputePluginTrustedTimeObservation`；
4. 线性 `DurableWorkAdmittedPluginSlot`；
5. 既有 `ValidatedComputeReadyPublication`；
6. 明确不受信的 `UntrustedComputeUserNodeHostRuntimeObservationV1`。

入口不接受路径、普通 `rusqlite::Connection`、墙钟、裸 inventory、裸摘要、caller-supplied current 布尔值或 v14
Planning custody。`OpenedComputePluginLocalAuthority` 当前没有生产构造器，所以本合同在 handle-bound VFS 落位前仍不可达。新增 open-attempt 草案也只有无生产 owner 的 `RegisteredPending -> OpeningPreConnection` ownership shape，不打开 SQLite、不接受 Connection，不能充当本 seal producer。

## 3. 单一 Deferred 快照顺序

入口在事务前后都重验 opened root/controller、process fence 和 fresh trusted-time liveness。中间只允许一次
`TransactionBehavior::Deferred` + connection-local `query_only=ON` 快照，固定顺序为：

1. 核对 opened authority、process fence、fresh time、work-admission receipt 与 Ready installation/clock epoch；
2. 从 `authority_meta` 重建 canonical authority/inventory，要求 current sharing 仍 enabled，sharing authorization
   ref/revision/digest、policy、node profile、Manifest catalog、双 keyring bundle/revision/digest 与 admitted Plan 完全
   一致，target/Host API 仍与 admitted launch profile 完全一致；
3. 读取 exact plugin work-admission head，要求 ID、digest、generation 与传入线性 owner 相同，并重读完整 predecessor
   chain、source/receipt canonical row 及原 install/promotion durable receipt pair；
4. 允许 authority 从 work-admission 的 stopped/no-health 状态单调推进到 Ready successor，但 current process owner 不得
   更换，且 current inventory revision、policy revision 和 exact plugin record 必须与 `ValidatedComputeReadyPublication`
   相同；
5. 以 fresh authenticated time 重新校验 Ready record、health digest 与 TTL；Ready 原 observation 和 fresh observation
   必须属于同一 installation/clock epoch，fresh monotonic observation/time 还必须严格晚于 retained work-admission 的
   post-rehash barrier 与 trusted-time observation；
6. 复用原 source-lineage builder，重新执行 admission/Ready/Host 的 Plan、grant、Runner、generation、resource、CPU-only
   和时间区间等式；
7. 回读 authority identity/revision/inventory digest/epoch/process/high-water，确认事务内没有观察到漂移，再把投影封入
   transaction-scoped seal；
8. callback 返回后提交只读事务，并再次重验三项外部 custody；callback panic 只在 owner seam 内暂存。当只读 commit
   与 connection-local `query_only` 恢复都成功时，原 panic 才在快照外重新抛出；任一收尾失败时，既有 owner seam
   优先返回 Store 错误，当前源码不承诺保留原 panic payload。

任一步不成立都失败关闭；没有“尽量当前”、降级到历史 head 或返回部分 seal 的分支。

## 4. 合法 Ready successor

work-admission receipt 形成时本机 runtime 固定 `stopped`、health absent。当前性检查不得调用只适用于该旧时点的
`validate_current_admission` 来误拒合法 Ready successor，也不得跳过历史 owner 审计。合法 successor 至少满足：

- authority state 与 inventory revision 严格晚于 admission，authority epoch 不回退；
- process owner epoch 与 admission、process fence、current Store 完全相同；
- active slot、release、install/activation generation、last Plan 和 permission grant 不变；
- current Ready record 与原 `ValidatedComputeReadyPublication` 逐字段相同；
- runtime generation 严格晚于 admission 的 pre-Ready generation；
- current health 在 fresh trusted now 仍未过期，且原 health digest 可按 owner 公式重算。

这些条件只证明“当前 Store 与既有 Ready DTO 一致”。由于本批没有 runtime transition writer、Host custody 或真实
Sidecar，它绝不证明 Store 中 Ready 状态的产生过程可信。

## 5. Seal 与不可外逃边界

`CurrentComputeUserNodeReadySourceLineageSeal<'snapshot>` 用不可命名私有字段、transaction lifetime 与 `Rc` phantom
owner 绑定只读快照和当前线程。prover、seal、getter 均保持模块私有，入口只接受
`for<'snapshot> FnOnce(Seal<'snapshot>) -> Result<()>`，不允许返回任意 owned successor；因此 seal 不能被返回、跨线程、
Clone、Serde 或保存到下一事务。callback 只允许做无 I/O、无 writer、无发布/调度/网络/设备效果的纯进程内派生；
它可读取原 untrusted lineage 供未来同事务 owner 使用，但不能把 local-currentness 权威序列化进六键 V1 envelope。
类型系统不证明 callback 纯度，所以任何 future caller 都必须另过 code review。正常收尾成功时，panic recovery 会先恢复
`query_only` 再重新抛出；commit/restore 失败与 callback panic 并发时的 payload 保留仍是未验收缺口，不能据此把回调
副作用变成合法效果。

本批没有调用点。未来消费者必须先显式修改私有边界，并定义 lifetime-bound 固定 successor；不得仅把返回值改回泛型
owned `R`，也不得把当前 prover 直接开放给其他 host 模块。

因此原 `Projected...` envelope 的四项 `authority_gaps` 逐字保持 `missing`。只有持有本 seal 的同事务代码，才可把
`node_local_authority_currentness` 解释为在该快照内已重证；脱离 seal 后仍只剩原 untrusted projection。

## 6. 三个剩余硬缺口

本批之后仍固定缺少：

- `runtime_transition_authority`：exact start/ready/stop/recovery receipt 与 generation successor；
- `host_runtime_authority`：真实 Runner/Sidecar custody、认证 IPC、OS enforcement、活动健康和主动失效；
- `v15_authenticated_session`：独立 endpoint capability、节点签名、append-only session ledger、重放与撤销闭环。

v14 永久保持 `planning_snapshot_bootstrap_only`，不得承载本 seal、Ready 或运行时事实。

## 7. 零效果与状态声明

本合同新增 `migration/table/writer=none/none/none`，不修改 V1 envelope、work-admission、inventory 或 health schema。
read-only callback 不开放 Service、HTTP/MCP、Wire 或控制 WebSocket。全部下游效果固定为 `none`：Ready mint、Provider、
route、Offer、capacity、execution、Job、Attempt、Lease、Receipt、settlement、money。

exact-head helper 当前会计数全部 receipt 并回放完整 predecessor chain，复杂度为 O(history)。本批没有扫描上限、缓存、
checkpoint 或性能证据；在另立有界 owner 合同并动态验收前，不得把该 source seam 接到高频 Ready 发布热路径。

当前状态严格为 `unregistered/draft_frozen/source_written/source_review_only/implementation_uncompiled/
implementation_unrun`、`passed=0/failed=0`。格式化和静态审阅不提高运行成熟度。

## 8. 后续顺序

1. 先完成 A2 动态门：当前 Barrier 与 RegistrationShutdown 均为 `8/8`、RegistryLifecycle 为 `16/16`、Unmap 为 `49/49`，A2b2 为 `81/117`，剩余 36 项全部是 JointClose；clean wide regression `205/205` 已通过，但 Map/Lock 独立 denominator 与 JointClose 未闭合；之后再为既有 open-attempt 两态接入唯一 production owner、VFS/open/close 与本 seal 的真实 producer；
2. 形成 runtime transition 和 Host runtime authority；
3. 新建 v15 authenticated session 与 Node 签名发布；
4. 服务端在 current V279 binding、consent、credential/session 下验证并封存短 TTL Ready authority；
5. 再进入既有 execution plan、outbox/ACK/Lease、usage/ExecutionReceipt/SettlementReceipt 主链。

任何后续实现都必须复用现有 Provider/Offer/Job/Lease/Receipt 合同，不得从本地 seal 复制第二套联邦领域模型。
