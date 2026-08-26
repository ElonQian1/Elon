---
title: UserNode Windows Runner Recursive Policy Signature Verification And Dispatch Currentness V1 验收草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-recursive-policy-currentness-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Recursive Policy Signature Verification And Dispatch Currentness V1 验收草案

权威合同见
[Recursive Policy Currentness authority](user-node-windows-runner-recursive-policy-currentness-authority.md)。

## 1. 本批证据等级

- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- code acceptance: `passed=0/failed=0`；
- Windows dynamic evidence: `0`；
- persistence: `migration/table/writer=none/none/none`；
- signature-verifier producer/currentness backend/grant-dispatch producers: `missing`。

没有运行编译、Cargo/Rust test、source-contract test、migration、SQLite、网络、设备、Win32 fixture或真实 Runner。所有 guard只是
未运行的源码审阅目标；`failed=0`不表示通过。

## 2. 静态责任面

| 文件/模块 | 静态审阅责任 |
|---|---|
| `exact_context_plan/recursive_policy.rs` | authenticated policy façade、payload V1校验、binding V2与 limits gate |
| `recursive_policy/signature.rs` | signed envelope V1、signer tuple与 typed verification evidence |
| `recursive_policy/currentness.rs` | A0/Ak point-of-use linear authorization与 coordinates |
| `recursive_policy/{digest,validation}.rs` | canonical evidence/digest重算、signer/keyring/time/policy-generation fail-closed校验 |
| `resolution/grant_ready/policy_currentness.rs`、`resolution.rs` | GrantReady → PolicyCurrent A0 pre-dispatch typestate、policy-current outer namespace与 A0 linear split边界 |
| `system_closure/acquisition/{custody,digest,validation}.rs` | Ak currentness-pending → DispatchReady、authorization whole custody、receipt/output V3与 chain monotonicity |
| `runtime_loader_recursive_policy_currentness_source_contract_tests.rs` | 未运行 guard：私有性、Infallible、无 Clone/Serde/retry extractor、A0/Ak/receipt source markers |

## 3. 静态源码审阅目标

人工 review目标如下，不能解释为测试通过：

1. signed payload直接绑定 policy scope、generation、validity、exact signer record/SPKI与 signing keyring generation；unsigned
   JCS material digest与 signature-bytes digest分域、无环，并冻结 exact Ed25519 domain+digest消息；
2. authenticated policy按值拥有 typed signature verification evidence，binding domain为V2；
3. signer key ID/record/SPKI/signing generation/policy generation在 envelope、verification、currentness三处 exact相等；
4. current keyring generation允许合法前进但禁止回退，并证明 exact record active；policy scope generation exact current；
5. trusted time位于 `[not_before, not_after)`且 typed observation/anti-rollback evidence完整；
6. A0与每个Ak都在第一次副作用前取得独立 authorization，绑定 exact receipt/wave/input/plan coordinates；
7. Ak必须先完成 whole plan/limit validation再进入 currentness-pending，不能提前发行宽泛 permit；
8. A0 authorization从 PolicyCurrent GrantReady按值进入 outer policy-current namespace；inner GrantAcquired/PreFinal保持
   policy-free，A0 sealer唯一拆分 namespace→accumulated、policy→accumulated、authorization→receipt；recursive authorization同样随
   success与 failure whole custody移动，receipt按值保留完整 evidence；
9. receipt/output schema升级V3；chain验证 nonce唯一、keyring generation、trusted-time observation与
   `trusted_time_attestation_sequence`均不回退；
10. receipt-set/chain V1、closure V2、profile V3只消费 versioned child digest且保持无环。

## 4. 明确未验收矩阵

| 验收面 | passed | failed | not_run | 状态 |
|---|---:|---:|---:|---|
| signed envelope parse/JCS/Ed25519 verification | 0 | 0 | 1 | real verifier missing |
| active Control-ring exact record/revocation currentness | 0 | 0 | 1 | backend missing |
| policy scope replacement generation currentness | 0 | 0 | 1 | backend missing |
| trusted-time validity与anti-rollback | 0 | 0 | 1 | dispatch observation backend missing |
| A0 PolicyCurrent transition | 0 | 0 | 1 | producer missing |
| Ak currentness-pending → DispatchReady | 0 | 0 | 1 | producer missing |
| cross-wave nonce/generation/time monotonic chain | 0 | 0 | 1 | source review only |
| failure/retry whole-custody dynamic matrix | 0 | 0 | 1 | recovery/backend missing |
| source-contract guards | 0 | 0 | 1 | user-requested architecture phase |
| compile/Rust tests/Windows dynamic | 0 | 0 | 1 | not run |

## 5. 负向验收门

以下任一情况未来都必须失败关闭：

- 接受 caller `is_current`、wall clock、detached fingerprint或裸 policy digest；
- envelope/verification/currentness signer tuple任一项不等；
- observed keyring generation小于 signing generation或相对前 receipt回退；
- exact key record已撤销、替换、SPKI变化，或 policy scope generation不再相等；
- trusted time在有效区间外、observation回退或anti-rollback receipt缺失；
- A0遗漏授权、Ak在 plan/limits gate前授权，或 authorization coordinates与 receipt不符；
- nonce跨 wave/retry/plan复用；
- failure custody丢失 authorization，或暴露 scalar retry permit；
- inner GrantAcquired/PreFinal重复持有 policy/authorization，导致 namespace、accumulated与 A0 receipt需要 Clone或重复签发；
- receipt只保存 authorization digest而没有完整 typed evidence；
- acquisition canonical material变化但仍宣称V2。

## 6. 下一验收门

先由真实 Control-ring resolver、trusted-time observation及anti-rollback authority产生非零 signature-verification/currentness证据，再接
retained-handle parser；随后才验收 per-wave resolver、external-directory currentness、grant/candidate/lease backend与 positive
advancer。final sealer/query/recovery和 Windows fault matrix继续后置。

四项 Ready gap逐字保持 `missing`；loader 18项 effect逐字保持 `none`。本批不得声称 policy已可生产、Runner已可启动、
Runtime/Ready已接线或存在任何 Provider/市场/资金效果。
