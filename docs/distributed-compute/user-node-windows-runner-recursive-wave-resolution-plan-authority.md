---
title: UserNode Windows Runner Recursive Wave Resolution Plan V1 权威草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-recursive-wave-resolution-plan-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Recursive Wave Resolution Plan V1 权威草案

对应验收见
[Recursive Wave Resolution Plan acceptance](user-node-windows-runner-recursive-wave-resolution-plan-acceptance.md)。逐波 owner、失败与
acquisition receipt见
[Recursive System-Image Acquisition Custody authority](user-node-windows-runner-recursive-system-image-acquisition-custody-authority.md)，
最终反投影与 fixpoint见
[Recursive System-Image Closure authority](user-node-windows-runner-recursive-system-image-closure-authority.md)，wave-zero 计划见
[Launch Context Selection authority](user-node-windows-runner-launch-context-selection-authority.md)，逐 A0/Ak currentness见
[Recursive Policy Currentness authority](user-node-windows-runner-recursive-policy-currentness-authority.md)。

## 1. 本批结论

本批冻结并写入 **source-only canonical per-producer-wave request/resolution plan V1**。它关闭的是 parser evidence与第一项
grant/candidate/lease dispatch之间的 forward-plan 形状缺口，不实现真实 parser、resolver或 backend。

对应 source shape集中在
`resolution/system_closure/acquisition/{plan,plan_digest,plan_forwarder_validation,plan_owner_validation,plan_projection,plan_validation}.rs`；
`acquisition/custody.rs`只把 validated immutable plan evidence接入 whole-owner typestate，不新增 live owner producer。

`A0`继续 exact复用既有 `GrantReadyWindowsRunnerResolutionPlan`，不制造第二份 base plan。只有 recursive acquisition
`Ak (k >= 1)`使用新的 request plan V1与 resolution plan V1：先从 producer wave `k`的 exact source frontier生成 canonical
outgoing requests，再补齐每步 disposition、exact terminal、filesystem dedupe及后续 grant/route-owner/lease request
commitments。最终 sealed graph只能反向验证这些 plans，不能再把 final `parsed_edge_set_digest`或 `wave_digest`本身冒充 forward
plan。

状态严格为 `source_written/source_review_only/implementation_uncompiled/implementation_unrun`、`passed=0/failed=0`、
Windows dynamic=`0`、`migration/table/writer=none/none/none`。feature registry不可用，登记状态保持
`unregistered_feature_workflow_unavailable`。本批未运行编译、Cargo/Rust/source-contract test、migration、SQLite、网络、
设备、Win32 fixture或真实 Runner；静态 source shape不构成 producer证据。

## 2. A0 与 Ak 的唯一分工

acquisition ordinal与 producer wave继续一一对应：

- `A0`消费 GrantReady/base request ranges；其 request/resolution plan digest均引用同一份已验证 GrantReady plan；
- `Ak (k >= 1)`消费 projection wave `k`产生的 exact request ranges，并分别引用本专题的 canonical request plan V1与
  authenticated resolution plan V1；
- nonempty next frontier只允许 `producer k -> target parse wave Some(k+1)`；terminal `A_N`必须为 `None`且 frontier为空；
- 若没有 recursive wave，则 `A0=A_N`，新 plan V1实例数为零；不得为 base或 terminal另外制造空 recursive plan。

`Ak`的 source frontier必须等于前一 acquisition receipt分配给 wave `k`的 parse receipt ordinals。计划不能跳 wave、延迟首次
target、引用 future receipt，或按 backend completion order重新编号。

## 3. Canonical request plan V1

每份 `WindowsRecursiveWaveRequestPlan` V1直接绑定：

- authenticated recursive policy digest、parser policy digest、producer wave ordinal；
- previous acquisition receipt digest与 previous output/input custody digest；前一 receipt ordinal由 `producer k-1`唯一派生，
  不接受 caller另报 scalar ordinal；
- source frontier的 ordered parse receipt ordinals、receipt digests、exact owner refs与 immutable material bindings；
- exact contiguous module-request vector与 module/name/system range bases；paired resolution plan再用 exact searched-name与
  filesystem-system-image vectors封闭各 range终点；
- 每条 outgoing module request的 final/global request ordinal、source parse receipt、importer parsed-image、
  importer graph-edge ordinal、normal/delay/forwarder kind、stage locator、normalized requested name、symbol name-or-ordinal；
- ordered search-step ordinals；不得新增 ambient PATH或按 wave改变 policy；
- stage locator中的 forwarder root/hop evidence refs；
- 独立 `windows_recursive_wave_request_plan.v1` canonical material与 plan digest。

上述 prior receipt/output custody、typed frontier receipt evidence、policy/parser digests与 ordered search-step ordinals均为 plan的
**直接绑定**。admission、Runner、CWD、machine与完整 search-policy lineage不在 `Ak`重复展开，继续经 authenticated recursive
policy、exact-context intent及 retained search-directory authority **传递绑定**。earliest producer、already-parsed decision与
filesystem/cache dedupe属于 paired resolution plan的直接绑定，不由 request caller预报。

request vector按 source parse receipt ordinal、再按 importer graph-edge canonical order形成。重复 target只保留 earliest final
module-request producer；所有 request/range ordinal连续且使用 checked arithmetic。该 plan不含 terminal/disposition、真实 grant、
candidate、lease、response bytes或 final graph digest。

## 4. Authenticated resolution plan V1

每份 `AuthenticatedWindowsRecursiveWaveResolutionPlan` V1直接绑定同一 policy/parser/producer wave与 request plan digest，并完整
保存：

- 每个 search step的 exact directory ordinal及 authority binding；borrowed validation回查 earlier whole custody中的 retained typed
  directory authority，不克隆或重取 handle；
- 每条 module request的 ordered per-step disposition与唯一 exact terminal；
- 每个 searched-name ordinal、normalized name、grant request与 disposition binding；
- package content lease、authenticated preloaded module、KnownDLL section、一步 API-set host、SxS host与 ordinary filesystem
  system image的 route-specific terminal refs；其中 ordinary filesystem与 SxS都是 filesystem-backed route；
- filesystem target按 resolved identity/route/material做 canonical dedupe后的 exact request vector、primary/secondary uses、
  expected candidate commitment、lease request commitment与 servicing/section policy expectations；
- filesystem request按 earliest use排序，primary固定为 canonical first use；每个 filesystem terminal恰好出现一次并精确绑定
  module/request/route，非 filesystem terminal不得伪装成 use；
- next-frontier target allocation exact vector，逐项绑定 target node、earliest producer request、target parse wave、owner ref与预分配
  parse receipt ordinal；
- exact terminal set、step-disposition set、filesystem-request set、route-owner set与 resolution-plan digests，以及逐项
  grant-request、candidate-binding和 lease-request commitments。

API-set在 V1中只允许一步落到 exact non-recursive host；原始 edge name必须等于 contract name，而 filesystem/SxS search、use与
candidate name必须等于 normalized host module key，禁止拿 contract name冒充 host搜索。nested API-set DAG继续 fail-closed。`ShadowedByEarlierName` positive path
继续拒绝。计划中的 candidate/lease只是不含 live owner的 commitment；filesystem-backed ordinary/SxS actual retained candidate只能在全部
本 wave searched-name grants完成后进入 post-grant custody。

## 5. Exact vectors 与派生 limits

以下三个旧 scalar projection不再是 policy gate的权威输入：

```text
projected_next_frontier_parse_receipt_count
projected_parsed_image_count
projected_forwarder_hop_depth
```

它们由 exact vectors和 whole accumulated custody派生：

- next-frontier count = canonical target-allocation vector长度；
- cumulative parsed-image count = A0保留的 exact prelease parsed-image/package-file/postlease parsed-image三坐标 typed
  cross-bindings + prior exact parse receipts + 本次唯一 target allocations；任意两类 ordinal不得被假设相等，base owner vector按
  postlease parsed-image ordinal严格递增但不要求从零连续；
- forwarder max depth = base/prior custody保留的 exact chains与当前 request plan root/hop逐链推进后计算的累计最大值；
- module/name/system totals = prior contiguous ranges + 当前 exact vectors长度；
- recursive wave count = producer wave ordinal + nonempty next-frontier transition。

所有加法和 ordinal终点使用 checked arithmetic。任何 external dispatch前必须从上述 exact material重新派生六项 totals并调用同一
signed policy gate；caller不能提交三个 scalar、detached digest或预估值绕过 vectors。

## 6. DispatchReady typestate

只有 borrowed validation逐项证明以下关系后，whole request/resolution owner才可进入
`PolicyCurrentnessPendingWindowsRecursiveWaveGrantCustody`：

1. policy、parser、producer wave、previous receipt/output custody与 source frontier完全一致；
2. requests、ranges、search steps、terminals、dispositions、dedupe与所有子摘要重算一致；
3. target allocations使用 earliest producer并与 already-parsed/cache closure不冲突；A0 owner按独立 parsed/package坐标匹配；
4. exact vectors派生的六项 totals未超过 signed limits；
5. retained+current forwarder chain逐 hop绑定 root、importer、source/target symbol与 target node，且不存在环；
6. A0 base-owner set与截至前一 receipt的完整 forwarder-root set由 versioned output-state digest锚定，不能用自洽子集降低
   parsed count或累计 depth；
7. route-specific commitments不提前持有 actual candidate、grant、lease或 response owner；post-grant candidate的 servicing、
   namespace currentness与 parent-relative open evidence随 positive outcome保留，且 candidate evidence与最终 image必须提交同一个
   parent-relative open receipt，供 final projection重验。

`WindowsRecursiveWaveRequestCustody`与 `WindowsRecursiveWaveResolvedPlanCustody`分别按值保留 request与 authenticated resolved
plan；未来 consuming validator transition只能把完整 accumulated owner与完整 plan evidence一起推进为 currentness-pending。
后者再取得绑定本 wave/input/plan的线性 currentness authorization，才形成
`DispatchReadyWindowsRecursiveWaveGrantCustody`。该 validated plan evidence与 authorization随后按值留在
grant/candidate/lease/parse stages，不能
拆成 scalar permit，也不提供 Clone/Copy/Serde、成功 `into_parts`、raw handle、path或 retry extractor。它只表示可以尝试第一项
side-effecting dispatch，不表示 dispatch、resolver、grant、candidate、lease、positive advancer或 wave完成成功。

## 7. 摘要版本与无环关系

本批版本链固定为：

```text
authenticated recursive policy binding V2 + point-of-use currentness V1
+ parse receipt V2 + exact prior custody
→ recursive request plan V1
→ recursive resolution plan V1
→ acquisition output-custody V3 + acquisition receipt V3
  （含 base-owner-set / cumulative-forwarder-chain-set commitments）
→ receipt-set V1 → acquisition chain V1
→ recursive closure V2 → loader resolution profile V3
```

acquisition receipt/output在 forward-plan批次从V1升级到V2；本批因完整 point-of-use currentness evidence进入 canonical material
再升级到V3。receipt-set/chain canonical material仍只消费 ordered versioned receipt digests，
closure仍只消费 versioned chain digest，profile仍只消费 versioned closure digest，因此三者分别保持 V1/V2/V3，不得无字段变化
而联动升版。

`Ak` receipt不只保存两项 scalar digest；它按值持有
`WindowsRecursiveAcquisitionPlanEvidence::RecursiveWave { plan: WindowsRecursiveWaveDispatchPlanEvidence }`，其中完整保留 typed
source frontier、request、module resolution、search disposition、filesystem request与 route-owner exact vectors及其摘要。chain再按值
持有 ordered receipts，供 final projection逐项重算。该 evidence只有 immutable plan material与 typed ordinal refs，不持有 live
directory/grant/candidate/lease/parser owner；`A0` evidence仍只复用 GrantReady digest。

每份 receipt的 output state另提交同一个完整 A0 base parsed-owner set，以及处理完该 producer range后的累计 direct-root/
forwarder-chain set。final validator从 pre/post cross-binding与最终 edge prefix独立重建两项 set digest；下一 wave再从 accumulated
typed vectors重算并与前一 receipt相等比较。因此空集或合法子集不能降低 pre-dispatch parsed/depth totals。

parse receipt V2只保存 producer acquisition ordinal，不直接保存 policy或 current acquisition digest；policy通过 request/resolution
plan与 acquisition chain传递绑定。plan不得引用 current acquisition receipt、final closure/profile或 required process context，
避免摘要回环。

## 8. Producer、失败与零效果边界

policy signature-verification/currentness source shape已写；真实 signature verifier/currentness backend、prelease/recursive parser、
GrantReady/recursive resolver、external-directory owner、
grant/candidate/lease backend、positive-consuming advancer、final sealer/query/reopen/release/recovery仍全部 `missing`。request plan、
resolution plan和 DispatchReady的成功 producer继续由 private `Infallible`保持不可构造。

纯 borrowed validation失败返回未 dispatch的 whole owner；进入 DispatchReady并开始任一 dispatch后，失败必须继续走 acquisition
authority定义的 whole-graph definitive或 outcome-uncertain custody，保留 active/pending及全部 returned outcomes。plan validation
不能新增 scalar retry authority，也不能把 malformed/错绑 response降格为 borrow-only failure。

四项 Ready gap逐字保持 `missing`：`node_local_authority_currentness`、`runtime_transition_authority`、
`host_runtime_authority`、`v15_authenticated_session`。loader exact 18 effects逐字保持 `none`：`runtime_phase`、
`runtime_generation`、`runtime_start`、`runtime_resume`、`runtime_store`、`health`、`readiness`、`node`、`provider`、
`route`、`offer`、`capacity`、`execution`、`attempt`、`lease`、`usage`、`settlement`、`money`。

## 9. 后续顺序

1. 接入真实 policy signature verifier/currentness backend与 retained-handle recursive parser，使 canonical request plan拥有真实输入；
2. 实现 authenticated per-wave resolver与 external-directory currentness，形成真实 DispatchReady owner；
3. 按既有 acquisition合同实现 grant/candidate/lease/negative backend与 positive advancer；
4. empty frontier后实现 final sealer/query/reopen/recovery及 Windows fault matrix；
5. 再进入 live OS、pre-resume/dynamic-load、process、Runtime、Ready与市场接线。
