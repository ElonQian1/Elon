---
title: UserNode Windows Runner Recursive Policy Signature Verification And Dispatch Currentness V1 权威草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-recursive-policy-currentness-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Recursive Policy Signature Verification And Dispatch Currentness V1 权威草案

对应验收见
[Recursive Policy Currentness acceptance](user-node-windows-runner-recursive-policy-currentness-acceptance.md)。上游 exact-context、
preliminary request plan与 policy payload见
[Launch Context Selection authority](user-node-windows-runner-launch-context-selection-authority.md)，逐波 plan与 whole-owner custody见
[Recursive Wave Resolution Plan authority](user-node-windows-runner-recursive-wave-resolution-plan-authority.md) 和
[Recursive Acquisition Custody authority](user-node-windows-runner-recursive-system-image-acquisition-custody-authority.md)。

## 1. 本批结论

本批冻结一条仍不可生产的私有 typed chain：Control-signed recursive policy envelope → exact signer verification evidence →
authenticated policy binding V2 → A0/Ak point-of-use currentness authorization → acquisition receipt/output V3 → receipt-set/chain
V1。它关闭的是“policy digest内部自洽即可进入 pre-dispatch gate”的 source-shape 缺口，不实现控制面签名服务、节点真实
signature verifier、current Control-ring snapshot、trusted-time/anti-rollback currentness backend或任何 grant/candidate/lease
dispatch。

节点只验证 Control-plane envelope；节点侧不得保存私钥、冒充 signer、重新签发或续期 policy。真实 verifier/currentness producer
继续由 private `Infallible` 阻断。证据严格为
`source_written/source_review_only/implementation_uncompiled/implementation_unrun`、`passed=0/failed=0`、Windows dynamic=`0`、
`migration/table/writer=none/none/none`。本批没有运行编译、Cargo/Rust/source-contract test、migration、SQLite、网络、设备、
Win32 fixture或真实 Runner。

## 2. Signed envelope 与唯一 signer tuple

signed envelope V1直接绑定原 policy payload V1的 scope digest，并至少保存：

- policy generation、`not_before/not_after`有效区间；
- signer key ID、key-record digest、public-key/SPKI digest；
- signing Control-ring generation；
- JCS canonicalization、SHA-256 signed-material digest、Ed25519 signature、signature-bytes digest及 envelope digest。

policy payload V1继续绑定 exact launch-context intent、preliminary request plan、parser policy、authenticated preloaded set、完整
route order与六项 recursive limits。envelope不得改变或收窄这些字段，也不得把 admission、machine、search lineage从两个既有
authenticated digests旁路重造。signature domain必须独立，不能复用 Manifest、InstallPlan、keyring bundle、Attempt或市场合同的
签名域。

签名消息必须无环且唯一：`signature_material_digest = SHA256(JCS(unsigned envelope material))`，Ed25519验证消息固定为
`ELON_WINDOWS_RECURSIVE_POLICY_SIGNATURE_V1 || 0x00 || decoded(signature_material_digest)`。raw signature bytes不进入 unsigned
material；它们另由 `ELON_WINDOWS_RECURSIVE_POLICY_SIGNATURE_BYTES_V1` 域承诺，envelope digest再同时承诺 material digest与
signature-bytes digest。不得让 signature digest反向进入自身被签 material，也不得省略 raw signature的独立承诺。

verification evidence必须逐项证明下列 signer tuple完全相同：

```text
(signer key id,
 signer key-record digest,
 signer public-key/SPKI digest,
 signing Control-ring generation,
 policy generation,
 signed payload digest,
 signature material/message digests)
```

authenticated policy按值保留完整 typed verification evidence；不能只保存 caller给出的 `verified=true`、fingerprint、policy
digest或 signature receipt digest。policy authenticated binding升级为 V2并直接承诺 verification receipt material。旧的
V1 policy payload digest保持 V1；payload无字段变化时不得联动升级。

## 3. 使用时 currentness，不是启动时一次验证

签名在 policy创建时有效，不等于 dispatch时仍 current。A0及每个 Ak第一次副作用 dispatch前，都必须重新取得一份一次性、
线性的 `WindowsRecursivePolicyDispatchAuthorization`。每份授权直接绑定：

- authenticated policy digest与 signature-verification receipt；
- 完整 signer tuple；
- 当前 Control-ring binding/snapshot与 revocation-set digest；
- observed Control-ring generation及 exact active key-record/SPKI；
- policy scope的 current generation；
- typed trusted-time observation与 anti-rollback receipt；
- acquisition receipt ordinal、producer wave ordinal；
- exact input-custody digest、pre-dispatch plan-evidence digest；
- 唯一 dispatch nonce与 authorization digest。

currentness必须同时满足：

1. observed Control-ring generation不得小于 signing generation，也不得相对前一 receipt回退；合法 keyring轮换不要求二者相等；
2. 当前 snapshot中 exact signer record仍 active，key ID、record digest与SPKI digest全部相等；仅“同 key ID存在”不够；
3. policy scope current generation必须等于 signed policy generation；仅 key未撤销不能证明 policy未被替代；
4. typed trusted time位于 `[not_before, not_after)`，且 observation sequence/time不得相对前一 receipt回退；
5. authorization coordinates必须精确等于本 A0/Ak的 whole input与 validated plan evidence；nonce不得跨 wave、retry或plan复用。

普通 wall clock、persisted milliseconds、caller boolean、detached digest或“最新 generation”标量都不能构造 currentness。

## 4. A0 与 Ak typestate

A0顺序冻结为：

```text
GrantReadyWindowsRunnerResolutionPrerequisite
→ whole GrantReady borrowed validation
→ exact Control-ring/trusted-time currentness query
→ PolicyCurrentGrantReadyWindowsRunnerResolutionPrerequisite
→ first base searched-name / launch-component dispatch
→ base grants
→ PolicyCurrentPreFinalWindowsLoaderNamespaceGrantSet
→ base candidates/leases/parse
→ A0 receipt V3 retaining the whole authorization evidence
```

A0 authorization coordinates固定为 `acquisition_receipt_ordinal=0`、`producer_wave_ordinal=0`，input custody等于 exact
GrantReady plan digest，pre-dispatch evidence等于 canonical `BaseGrantReady` evidence digest。没有 current wrapper的
GrantReady owner不能进入 dispatcher。

`GrantAcquiredWindowsRunnerResolutionLeaseCustody`与内层 `PreFinalWindowsLoaderNamespaceGrantSet`必须保持 policy-free；policy与
A0 authorization只存在于外层 `PolicyCurrentPreFinalWindowsLoaderNamespaceGrantSet`。A0 sealer未来必须一次消费该 wrapper，线性
移动 namespace→recursive accumulated root、policy→同一 accumulated policy、authorization→A0 receipt。不得 Clone、重复签发、
`Option::take`或把同一 authority同时留在 base namespace与 receipt中。

Ak顺序冻结为：

```text
WindowsRecursiveWaveResolvedPlanCustody
→ whole plan / limits borrowed validation
→ PolicyCurrentnessPendingWindowsRecursiveWaveGrantCustody
→ exact Control-ring/trusted-time currentness query
→ DispatchReadyWindowsRecursiveWaveGrantCustody
→ grants/candidates/leases/same-owner parse
→ Ak receipt V3 retaining the whole authorization evidence
```

currentness发生在 canonical request/resolution plan及 exact-vector limit gate完成之后，因此授权能绑定最终
`validated_plan_evidence_digest`；它不能在 resolver之前预签一个宽泛 wave permit。DispatchReady仍只表示允许尝试第一项
side-effecting dispatch，不表示任一 backend成功。

## 5. 线性 custody、失败与 retry

authorization不得 `Clone`、`Copy`、Serde、Default、`From`，不得提供成功 `into_parts`、scalar permit、raw key、path、handle或
retry extractor。A0 authorization按值进入 policy-current GrantReady owner，并随 base grant及 policy-current namespace/lease
failure custody移动；Ak
authorization按值随 DispatchReady、candidate、lease、parse及 failure custody移动。最终 acquisition receipt按值保留完整
authorization evidence，不能只保留 detached digest。

signature/currentness borrowed validation失败必须归还 intact envelope/policy/input/plan whole owner，且没有 dispatch authority。
任一 dispatch发生后，authorization只能留在成功 whole custody或既有 definitive/outcome-uncertain failure custody；transport
error、response缺失、超时、错绑或 conflicting outcome不得产生新 nonce或可重试 permit。未来 retry必须重新取得 currentness，
并消费 whole parked graph。

## 6. Receipt、chain 与版本边界

每个 A0..AN receipt V3完整承诺 authorization canonical material。chain validation必须逐 receipt重算：

- authorization自身 digest与 policy/signature tuple；
- receipt/wave/input/pre-dispatch coordinates；
- nonce全链唯一；
- observed keyring generation、trusted-time observation与 `trusted_time_attestation_sequence`均不回退；
- output-custody digest及 receipt digest。

版本固定为：

```text
recursive policy payload V1
→ signed envelope / signature verification / dispatch currentness V1
→ authenticated recursive policy binding V2
→ recursive acquisition output-custody / receipt V3
→ receipt-set / acquisition-chain V1
→ recursive closure V2
→ loader resolution profile V3
```

receipt-set/chain继续只消费 ordered versioned receipt digests，closure/profile继续只消费 versioned child digest，因此无需无字段
变化联动升版。currentness不重复塞入 parse receipt、closure或 profile；外层通过 acquisition receipt/chain digest传递绑定。

## 7. 明确非目标与生产可达

本合同不实现 selector、prelease/recursive parser、GrantReady/recursive resolver、external-directory authority、grant/candidate/
lease backend、positive-consuming advancer、nested API-set、Shadow positive、final sealer/query/reopen/recovery、live OS、process、
Runtime、Ready、v15、Provider、route、Offer、Capacity、Execution、Attempt、usage、settlement或 money。

真实 `recursive_policy_signature_verifier_producer` 与 `recursive_policy_dispatch_currentness_backend` 均保持 `missing`。四项 Ready
gap逐字保持 `missing`：`node_local_authority_currentness`、`runtime_transition_authority`、`host_runtime_authority`、
`v15_authenticated_session`。loader exact 18 effects逐字保持 `none`：`runtime_phase`、`runtime_generation`、`runtime_start`、
`runtime_resume`、`runtime_store`、`health`、`readiness`、`node`、`provider`、`route`、`offer`、`capacity`、`execution`、
`attempt`、`lease`、`usage`、`settlement`、`money`。

后续生产可达顺序保持：接入真实 Control-ring/trusted-time/anti-rollback verifier与每波 currentness → retained-handle parser →
per-wave resolver/external-directory currentness → grant/candidate/lease backend与 positive advancer → final sealer/query/recovery →
live OS/process/Runtime/Ready/市场。任何 source shape或人工 review都不能升级 loader predecessor。
