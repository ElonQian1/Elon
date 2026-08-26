---
title: UserNode Ready 源谱系 V1 权威草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute
proposed_feature_id: compute-user-node-ready-source-lineage-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_draft_uncompiled
verification_status: source_review_only
---

# UserNode Ready 源谱系 V1 权威草案

## 1. 状态与目标

本合同是一个未登记、API-free 的 source draft。它冻结未来 `user_node` 技术就绪证明之前必须保留的三类本机来源：

1. v217 本机 `DurableWorkAdmittedPluginSlot` 所持有的 work-admission source/receipt；
2. 既有 `ValidatedComputeReadyPublication` 所核对的 inventory、active runtime 与短 TTL health；
3. 尚无生产权威的 Host runtime/resource/model observation。

输出只能叫 `ProjectedComputeUserNodeReadySourceLineageV1`。它不是 `ComputeReadyCapability`，也不是服务端
`VerifiedComputeExecutionCapability`。本批不新增 Store、migration、API、MCP、Wire、Provider 激活、route、Offer、
Attempt、Lease、计量或结算下游效果。

## 2. 为什么先铺源谱系

现有 Ready V1 只有 DTO、health 校验和线性 `ValidatedComputeReadyPublication`。它尚无 builder/parser，也没有在
同一本机 authority Store 中消费 current work-admission head、Host enforcement、真实 Sidecar/Runner、活动健康及
节点会话的完整入口。服务端 activation 仍只保存调用方给出的 `ready_capability_digest` 字符串；V279 只把
`node_binding_ref` 收紧为 durable identity binding。

因此不能直接补一个接受普通字符串的 Ready builder。必须先逐字段保留来源和等式，并把尚未存在的权威写进
输出状态；后续生产 verifier 才有可审计的输入，而不是从一个 opaque digest 倒推来源。

## 3. 六键 envelope 与摘要域

Envelope 固定六键：

- `schema=compute_federation.user_node_ready_source_lineage.v1`；
- `lineage_kind=user_node_ready_source_lineage_v1`；
- `lineage_digest`；
- `canonicalization=rfc8785_jcs`；
- `digest_algorithm=sha256`；
- `lineage`。

摘要固定为
`sha256("ELON-COMPUTE-USER-NODE-READY-SOURCE-LINEAGE-V1" || 0x00 || JCS(envelope-with-empty-lineage_digest))`。
反序列化只接受 canonical JSON、精确 schema、精确摘要和 `deny_unknown_fields`。该摘要只能证明投影内容未被静默
改写，不能证明来源真实、当前或已认证。

## 4. Work-admission owner source

Node owner adapter 只能从 `DurableWorkAdmittedPluginSlot` 读取：

- exact source/receipt digest 与 `work_admission_id`；
- installation、plugin、slot、release；
- install/promotion receipt pair；
- signed Plan、PlanApply receipt 与 grant；
- work-admission 的 clock epoch、admitted time、policy revision、authority state/epoch、process owner 与 inventory digest fence；
- install、activation、pre-Ready runtime、work-admission generation；
- authority Store 的 inventory revision；
- admitted Runner、task kinds、target accelerator kind；
- CPU、内存、显存、磁盘、进程、Sidecar uptime 和 network-egress 上限。

这些字段仍是“允许在该上限内尝试启动”的来源，不是已启动、已 enforcement、已健康或可调度证明。

## 5. Ready-health owner source

Node owner adapter 同时读取不可由普通 DTO 构造的 `ValidatedComputeReadyPublication`，保留：

- 同一 installation、plugin、active slot 与 release；
- inventory/policy revision 及 install、activation、runtime generation；
- permission-grant、Runner 与 health observation digest；
- 规范排序的 health reason codes；
- health observed/expires 时间；
- trusted-time clock epoch、authority、attestation、sequence、signing-key fingerprint 与 trusted now。

该 token 证明既有 inventory snapshot 在一次 authenticated time observation 下通过了局部 Ready-health 校验；它
尚未完成发布前 fresh Store read、CAS/fencing、Host enforcement 或主动失效链。

## 6. Host observation 明确不受信

第三类输入的名字固定为 `UntrustedComputeUserNodeHostRuntimeObservationV1`。它可记录 executor、Runner/runtime、
Host enforcement 引用、resource profile、task kinds、model/tokenizer、precision、观测资源、technical concurrency 和
短 TTL。它拥有独立 JCS/SHA-256 自一致摘要，但没有 Host signing key、进程 custody、真实 Sidecar/IPC、OS enforcement
receipt 或服务端 challenge，因此不得改名为 `Validated`、`Verified`、`Authorized` 或 `Ready`。

Host observation 必须位于 work-admission grant 内。它不能通过自报扩大 CPU、内存、显存、磁盘、进程或并发，也
不能增加未获准的 task kind。

## 7. 必须闭合的来源等式

投影只在以下条件全部成立时构造：

1. work-admission 与 ready-health 的 installation、plugin、slot、release 完全相同；
2. Ready `last_plan_id`、policy revision 与 work-admission 的 exact Plan/policy 相同；
3. Ready trusted-time clock epoch 与 work-admission clock epoch 相同，health 时间严格晚于 admitted time；
4. install/activation generation 完全相同；
5. Ready permission-grant digest 等于 work-admission grant digest；
6. work-admission、Ready health 和 Host observation 的 Runner digest 完全相同；
7. Ready runtime generation 只声明严格晚于 work-admission 时记录的 pre-Ready runtime generation，不能据此推导 exact successor；
8. Ready inventory revision 严格晚于 work-admission inventory revision；
9. Host task kinds 与 signed launch profile 完全相同；
10. Host observation 时间覆盖 Ready-health 的完整有效区间；
11. Host 自报资源不超过 signed grant。

这些等式只说明三份材料在结构上可共同审阅。它们不补出缺失的 state transition receipt，不把自报 Host observation
升级为可信证据，也不证明节点在线。

## 8. CPU-only 必须诚实表达

work-admission 已允许 CPU-only 节点保留 `max_vram_bytes=0`。本合同继续保持该语义：target 未声明 accelerator 时，
Host observation 必须同时给出 `accelerator_count=0` 与 `vram_bytes=0`，不得虚构 accelerator 来满足服务端现有
`accelerator_count>0` 假设。target 声明 accelerator 时，observation 才必须显式给出正数 count。

本投影不修改 execution-plan 的 numeric ceiling validator。CPU-only 到 Provider-neutral execution capability 的映射必须
在后续独立合同中显式版本化，不能在 source lineage 中暗改 F0 ABI。

## 9. 四个硬缺口

每份 lineage 固定：

- `projection_status=missing_node_currentness_runtime_transition_host_runtime_and_v15_session_authority`；
- `authority_gaps.node_local_authority_currentness=missing`；
- `authority_gaps.runtime_transition_authority=missing`；
- `authority_gaps.host_runtime_authority=missing`；
- `authority_gaps.v15_authenticated_session=missing`。

本机 currentness 缺口只能由发布前 fresh Store read、current work-admission head、authority epoch/process owner、共享状态、
CAS/fencing 与主动失效链关闭。Runtime transition 缺口只能由未来独立启动/停止/恢复 receipt 冻结 exact successor；当前
`runtime_generation > prior` 只是结构排序，不是 transition proof。Host runtime 缺口只能由真实 Sidecar/Runner custody、
认证 IPC、OS enforcement、活动健康和主动失效形成的 Host authority 关闭。v15 缺口只能由独立 endpoint capability、
authenticated session、append-only ledger、重放与撤销闭环关闭。v14 永久是 blocked-only compatibility profile，不得添加
Ready 分支。

独立的 [`UserNode Ready 本机当前性封印 V1`](user-node-ready-local-currentness-authority.md) 已把第一个缺口铺成
source-written、未编译/未运行的 transaction-scoped seam：只有 handle-bound opened authority、同一 process fence、fresh
authenticated time、exact current work-admission head/chain、相同 inventory revision/policy 与逐字段相同的 exact
plugin record 同时成立，才在
`for<'snapshot>` callback 内形成 private-field、non-Clone/non-Serde seal。原六键 envelope 仍把四项 gap 全部写为
`missing`；脱离该 seal 不得声称 local currentness 已成立。该增量没有生产 open 构造器、表、写入或下游效果。

2026-08-26 的 Windows Runner 草案只写入单一 loader successor、完整 by-value seam、FileId leases/reopen receipts、
五类 failure custody、GrantReady wave-zero prefix、authenticated recursive policy/per-producer-wave custody contract、post-lease
recursive system-image final projection envelope、suspended-child/atomic-Job/pre-create currentness与 unconfirmed whole-graph parking；
真实 selector/policy signature verifier/currentness backend、prelease/recursive parser、resolver、grant/candidate/lease backend/positive advancer、
sealer/query、exact PE/launch/live-OS、launch-security/currentness/
release-recovery producers 均缺。extraction-share 与 Runner/package-root/全部 plan-directory retained handle-chain candidate
discovery 已形成 typed source seams，但没有真实 selected CWD/component grant，两层 Windows 动态矩阵仍缺；pre-resume/dynamic-load、
`ResumeThread`、IPC/enforcement/Store blocked。因此
`runtime_transition_authority` 与 `host_runtime_authority` 仍均为 `missing`；精确
边界见 [`recursive acquisition custody`](user-node-windows-runner-recursive-system-image-acquisition-custody-authority.md)、
[`recursive system-image closure`](user-node-windows-runner-recursive-system-image-closure-authority.md)、
[`loader load-set`](user-node-windows-runner-loader-load-set-authority.md) 与
[`process custody`](user-node-windows-runner-process-custody-authority.md)，目录 seam 另见
[`extraction share custody`](user-node-windows-runner-extraction-directory-share-custody-authority.md)，候选发现见
[`launch-path discovery`](user-node-windows-runner-launch-path-discovery-authority.md)。

只有四项 source authority 均形成后，服务端 verifier 才可在 current V279 binding、current consent/credential、节点签名
和 session witness 上重新验证，并生成新的 server-owned Ready authority。它也仍不会自动创建 route、Offer、容量、
Attempt 或 Lease。

## 10. 固定零下游效果

成功投影仅有 `projection_effect=untrusted_source_projection_only`。以下全部固定 `none`：

- readiness；
- Provider；
- route；
- Offer；
- capacity；
- execution；
- Lease；
- settlement；
- money。

原六键 projection 模块自身没有表、Store/SQL resolver、migration、writer、Service、HTTP/MCP、节点上报或控制
WebSocket；独立 local-currentness seam 只读既有 authority/work-admission 表，没有 migration 或 writer。canonical parser
也只能恢复 untrusted envelope，不能恢复 owner custody 或构造 `Projected...`。

## 11. 后续顺序

1. 实现 handle-bound SQLite VFS/open，并动态验收独立 local-currentness seal 的 exact head、drift、TTL 与 custody 矩阵；
2. 沿 Windows Runner 草案动态验证 source-written extraction-share 与 launch-path discovery，并实现真实 authenticated CWD
   selector/recursive-policy signature verifier/currentness backend、prelease/recursive parser、GrantReady resolver、base grants/leases、逐 producer
   wave candidate/grant/lease backend与 positive advancer、same-owner reparse及 exact recursive PE/launch/resolution
   seal/final aggregate/query/reopen、launch-security/private desktop、
   live-OS/pre-create/pre-resume currentness、explicit release/recovery、dynamic-load、
   IPC/enforcement/Store、受控 resume 与健康/撤销，形成真实 Host runtime authority；
3. 新建 v15 endpoint session，不修改 v14 blocked-only；
4. 由 Node owner 消费 local-currentness、Runtime/Host authority 与 v15 session，完成签名发布；
5. 服务端在 current V279 binding/session/credential 下验证并封存短 TTL Ready authority；
6. 再把 server-owned Ready 与独立 route/hardware authority 投影为 execution capability；
7. 市场 Offer、容量预留和任务 Lease 继续走各自门卫。

任何一步都不能把本批 source draft 的存在描述为用户节点已经可被联邦 Broker 调度。
