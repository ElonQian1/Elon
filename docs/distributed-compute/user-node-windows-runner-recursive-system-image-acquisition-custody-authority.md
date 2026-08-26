---
title: UserNode Windows Runner Recursive System-Image Acquisition Custody V1 权威草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-recursive-system-image-acquisition-custody-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Recursive System-Image Acquisition Custody V1 权威草案

对应验收见
[Recursive System-Image Acquisition Custody acceptance](user-node-windows-runner-recursive-system-image-acquisition-custody-acceptance.md)。
最终图投影与 fixpoint 见
[Recursive System-Image Closure authority](user-node-windows-runner-recursive-system-image-closure-authority.md)，wave-zero
请求与 GrantReady prefix 见
[Launch Context Selection authority](user-node-windows-runner-launch-context-selection-authority.md)，最终 owner graph 见
[Loader Load-Set authority](user-node-windows-runner-loader-load-set-authority.md)。`Ak (k >= 1)`的 forward plan见
[Recursive Wave Resolution Plan authority](user-node-windows-runner-recursive-wave-resolution-plan-authority.md)，signed envelope与逐波
currentness见
[Recursive Policy Currentness authority](user-node-windows-runner-recursive-policy-currentness-authority.md)。

## 1. 本批结论

本批冻结并写入 **source-only authenticated recursive policy、canonical per-producer-wave plans与 acquisition custody contract**。
它把 final projection envelope 前缺失的线性来源补成 typed shape：base GrantReady acquisition在 frontier非空时产生
wave 1 source owners；每个 producer wave解析本波 source images，再为 outgoing requests取得 exact grants、route-specific
owners与 leases，形成下一 wave frontier；empty frontier 后才允许 final aggregate/sealer消费整图。

signed envelope、typed verification evidence与 A0/Ak point-of-use currentness authorization已形成 source-only shape，但该合同不是
真实签名/验签/currentness、namespace、candidate、lease、parser、advancer、sealer、query、release或 recovery backend。
所有成功 producer继续由 private `Infallible` 保持不可构造。状态严格为
`source_written/source_review_only/implementation_uncompiled/implementation_unrun`、`passed=0/failed=0`、Windows
dynamic=`0`、`migration/table/writer=none/none/none`。本批没有运行编译、Cargo/Rust/source-contract test、migration、
SQLite、网络、设备、Win32 fixture或真实 Runner；`failed=0` 不表示通过。

## 2. Authenticated recursive policy

`AuthenticatedWindowsRecursiveResolutionPolicy` 必须是独立版本化、Control-signed typed source。payload V1直接绑定：

- `launch_context_intent_digest`；
- `preliminary_request_plan_digest`、`parser_policy_digest`、`authenticated_preloaded_module_set_digest`与 exact ordered
  inherited resolution routes；
- 六项 exact limits：`max_wave_count`、`max_parsed_image_count`、`max_module_request_count`、
  `max_searched_name_count`、`max_system_image_request_count`、`max_forwarder_hop_count`；
- `ambient_path_allowed=false`、nested API-set/Shadow positive关闭与 startup-import-only dynamic-load scope；
- control key ID/keyring generation与 policy-scope payload digest。

signed envelope/verification/currentness的完整 signer tuple、validity、Control-ring exact record、policy scope replacement generation、
trusted time/anti-rollback及每波 dispatch coordinates由独立
[Recursive Policy Currentness authority](user-node-windows-runner-recursive-policy-currentness-authority.md)冻结。authenticated policy
binding为V2并按值保留 typed verification evidence；A0与每个Ak只有在 whole plan/limits gate后取得一次性 currentness
authorization才可进入第一项 dispatch。

admission source/receipt、manifest/signed envelope、grant、Runner、process-machine/WOW64与 search policy经已验证的 exact
context-intent/preliminary-plan digests传递绑定，不在 policy payload V1重复铺字段；传递绑定不能被描述成独立重复签名证据。

signed payload不包含自身 envelope、verification receipt或最终 binding digest，避免摘要 fixed point。schema使用独立
domain/version与长度定界的 canonical SHA-256 material；future wire decoder对 unknown version/field、非 canonical值或摘要错绑
均失败关闭。不得静默扩展既有
launch-context payload V1 hash domain；未来若把 policy嵌入其 payload，必须显式升级该 payload版本。

六项 limit逐值由 authenticated payload绑定，不能只把 `context_intent_digest`写入 closure后让 sealer自行选择数值。
wave count只计 recursive waves；parsed/module/name/system-owner limits覆盖 final base+recursive cumulative totals；
forwarder limit表示最大 hop depth而非 edge总数。任何会产生外部副作用的 dispatch前必须用 checked arithmetic验证本次
投影不会越限。

recursive policy payload V1必须 exact复用 selected launch-context中的 route order与由 context/plan传递的 machine/search binding，且保持
`ambient_path_allowed=false`；不得按 wave更换 policy、增加 ambient目录或把 fail-closed route打开。未来若允许显式收窄，必须
升级 policy版本并冻结 canonical semantics。所有 recursive parse、acquisition与 final closure逐项绑定同一 authenticated policy
binding digest。

## 3. 两套 wave 坐标

现有 final projection 的 wave `k`持有本波 source parse receipts，以及这些 parsed images产生的 module request、searched-name
与 new filesystem-owner ranges。后者供下一 wave解析，因此 acquisition合同必须同时保存：

- `producer_wave_ordinal`：产生 outgoing request的 projection wave；base GrantReady为 0；
- `target_parse_wave_ordinal`：next frontier非空时为 `Some(producer + 1)`；terminal `A_N`的 empty frontier必须为 `None`；
- `producer_module_request_ordinal`：该 target在 producer range内的 earliest final request。

base acquisition消费 GrantReady wave-zero requests；frontier非空时形成 wave 1 source owners，空时 A0本身就是 terminal
`A_N`。producer wave `k`先从前一 acquisition的 exact source owners做 same-owner parse，再按 canonical edge order形成 outgoing
requests并取得下一 frontier owners。
不得把 request acquisition与随后 parse含糊地标成同一 wave ordinal，也不得把 future owner提前移动进本波 receipt。

若 final projection含 `N=waves.len()`个 recursive waves，acquisition chain必须恰有 `N+1`份 receipts：`A0`消费 base request
range；`Ak (1≤k≤N)`消费 projection wave `k`的 request ranges；最后的 `A_N`必须产生 empty frontier。当 `N=0`时，
`A0=A_N`。不得缺 receipt、追加 detached terminal receipt或让一个 receipt覆盖两个 producer ranges。

若 base没有 recursive target，允许零 recursive wave直接形成 empty-frontier custody；不得制造空 wave。若存在 target，
receipt不得延迟、跳 wave或引用 future producer。

## 4. Canonical work queue 与 ordinal

每个 producer range先按 final module-request ordinal扫描；同 target多次到达只保留 earliest producer。新 target按该 earliest
ordinal分配连续 recursive parse-receipt ordinal；后台响应完成顺序不得影响 ordinal、wave merge或 final graph。

每 wave 的 module-request、searched-name、new filesystem-owner ranges从 base count之后连续追加，使用 checked arithmetic；
不得 gap、overlap、重新编号或把 package/system/parse/lease coordinates按 vector index猜测。already parsed target复用 exact typed
owner/ref，不重新取得 grant、candidate、lease或 parse receipt。相同 module-cache key绑定不同 node/identity/route时失败关闭。

existing closure仍按 source parse-receipt顺序合并 edges；本合同额外保证这些 receipt ordinals不是调用方或 final sealer任意
选择的排序输入。

`A0`不生成 recursive plan，而是 exact复用同一 GrantReady request/resolution plan digest。只有 `Ak (k >= 1)`持有独立
request plan V1与 resolution plan V1；它们逐项保存 outgoing request、search-step disposition、exact terminal、filesystem
dedupe与后续 grant/candidate/route-owner/lease commitments。final `parsed_edge_set_digest`和 `wave_digest`只用于反向
cross-binding，不能冒充上述 forward-plan digests。

## 5. Per-producer-wave linear custody

每个 wave advancer按值消费 whole prior state，至少持有 admission、authenticated policy、namespace session、base与 earlier-wave
grants/owners/leases、当前 source owners、active attempt及 pending refs。类型不得实现 `Clone`、`Copy`、Serialize或
Deserialize，不暴露 raw handle、`File`、path、detached digest constructor、成功 `into_parts`或 scalar retry permit。

逻辑顺序固定为：

```text
base GrantReady requests
→ whole GrantReady borrowed validation + exact Control-ring/trusted-time currentness query
→ PolicyCurrent GrantReady
→ base name grants + package leases + route-specific system owners
→ A0 composite same-owner evidence：package pre/post same-handle cross-binding + base-target parse set
→ [frontier 非空时] wave 1 source-owner custody
→ producer wave k same-owner parse
→ canonical outgoing unresolved request plan V1
→ exact terminal + per-step dispositions + movable external-owner refs resolution plan V1
→ whole-plan / exact-vector limit validation
→ PolicyCurrentnessPending typestate + exact point-of-use authorization
→ DispatchReady typestate
→ same-session searched-name / required search-directory grants
→ route-specific owner/candidate/content-lease acquisition
→ next-wave source-owner custody
→ empty frontier
→ final aggregate + recursive closure + resolution profile
```

“candidate”与“lease”不是所有 route的伪统一步骤：

1. `PackageContentLease`复用 exact package file与既有 immutable lease；
2. `AuthenticatedPreloadedModule`复用 exact preloaded ordinal、section identity与 evidence；
3. `KnownDllSection`复用 exact authority record、Object Manager section及 immutable mapping receipt；
4. `ResolvedFilesystemSystemImage`才消费 parent-relative retained candidate，取得 authenticated positive outcome、
   servicing-generation-bound immutable content lease与 section mapping。

candidate file被 positive transition消费后，positive outcome仍按值保留不含 handle的 candidate resolution evidence：parent
directory、normalized host/name、component/file、parent-relative open、code-integrity、servicing generation/receipt、namespace
currentness与 candidate binding。candidate evidence与消费后 image必须绑定同一个 parent-relative open receipt；final projection必须
重验这些字段，不能只凭 lease image或 detached candidate digest补写。

API-set是到 exact host的 terminal indirection，不是可解析 image owner；SxS最终仍必须落到上述 exact host owner。
相同 owner跨 edge/wave只用 typed ordinal/ref引用，不能 clone线性 handle、lease或 section owner。

## 6. Same-owner parse 与 acquisition receipt

filesystem-backed ordinary/SxS image只能从 exact acquired lease所持 retained handle/immutable section解析；package、preloaded与 KnownDLL
也必须从其 exact immutable owner material解析。parse receipt V2直接绑定 parse ordinal、target parse wave、
`producer_acquisition_receipt_ordinal`、earliest producer request、source-owner ref/binding、material identity、parser policy、
import-table digest/counts与 same-owner receipt digest；lease/servicing generation经 exact owner/material commitment传递。它不直接保存
authenticated-policy digest或 acquisition-receipt digest；acquisition validator用 producer ordinal、同一 parser policy与 owner projection
把它接回对应 receipt，从而避免摘要回环。

每个 producer wave另有独立 acquisition receipt V3，至少绑定：

- prior/output whole-state digest、policy/parser binding、producer/target coordinates与 exact input frontier；
- 完整 A0 prelease/package/postlease parsed-owner set digest，以及处理完该 producer range后的累计 direct-root/forwarder-chain
  set digest；下一 wave从 accumulated typed vectors重算，final validator从最终 cross-binding/edge prefix独立重建；
- A0的 exact GrantReady plan digest，或 `Ak (k >= 1)`按值保留的 immutable
  `WindowsRecursiveWaveDispatchPlanEvidence`（typed source frontier、request/resolution exact vectors、V1 digests与 exact ranges）；
- authenticated positive searched-name grant set、包含 retained servicing/currentness evidence的 filesystem candidate set V2、
  immutable lease set与 same-owner parse-set digests；
- 按值保留的完整 policy dispatch-currentness authorization evidence；
- next-frontier parse-receipt ordinals与 output custody digest。

acquisition receipt必须与 final projection wave的 module/name/system-owner ranges及 next frontier逐项 cross-bind。chain按值保留
ordered receipts，因此 final validator能从完整 plan vectors重算上述 forward evidence，而非只信 detached digests。
raw authenticated response bytes、positive owners与 retained handles不嵌入 plan evidence；它们留在成功线性 custody或完整失败
custody，receipt只额外承诺其 digests/owner sets。
现有 projection `wave_digest`只证明 final slice，不能冒充 forward plan或 acquisition custody。三个旧 scalar
`projected_next_frontier_parse_receipt_count`、`projected_parsed_image_count`、
`projected_forwarder_hop_depth`已由 exact request/terminal/frontier/forwarder vectors及 whole prior custody派生；dispatch前
signed-limit gate不得接受 caller projection。摘要 DAG固定为：

```text
authenticated policy binding V2 ────────────┐
parse receipt V2 digests + owner sets ───────┼→ request/resolution plans V1
exact prior custody + canonical vectors ─────┘       ↓
                         point-of-use currentness V1
                                      ↓
                                      acquisition output/receipt V3
                                      → receipt-set/chain V1 → recursive closure V2 → resolution profile V3
```

parse receipt只保存 producer acquisition ordinal，不保存 policy或 acquisition digest；policy经 plans与 chain传递绑定。
acquisition receipt/output在 forward-plan V1批次曾升级到V2；本批因完整 currentness evidence进入 canonical material再升级到V3。
receipt-set/chain material仍只消费 ordered versioned receipt digests，closure/profile仍只消费 versioned child digest，因此分别保持
V1/V2/V3。plans与 receipt不得反向引用 final closure/profile或 required process context。

## 7. Failure、partial acquisition 与 quarantine

只有第一次 grant/candidate/lease dispatch前的纯 borrowed validation失败可以返回 intact borrow-only owner。任一 dispatch消费
retry authority后，failure必须保留 whole prior state、全部既得 grants/owners/leases、active attempt、pending refs及所有 returned
response，且不提供 retry extractor。

`DefinitiveRejected`只在 authenticated negative与 exact owner/session/attempt/request/nonce/candidate/FileId/material全部匹配，
且没有同时 positive outcome时成立。timeout、transport error、response缺失/畸形/错绑、positive-but-invalid或
positive+negative同返均为 `OutcomeUncertain`；positive owner/receipt与 response bytes不能丢弃。parser failure也保留 exact
immutable source owner和整份 earlier-wave graph。

当前没有 per-wave release/recovery backend。persistent grant、lease、section、session或 root-lock不得由 ordinary Drop、session
disconnect或局部 definitive negative擅自释放；future recovery必须消费 whole parked graph并显式证明 authorized release。

## 8. Final aggregate 与明确非目标

只有 policy current、所有 producer waves完整、ranges/cross-binding闭合且 terminal frontier为空时，future final sealer才可消费
whole custody形成 recursive closure与 resolution profile。完整 base+recursive namespace的“原子覆盖”只指最终同一 session/
generation aggregate与 consuming currentness query，不表示递归开始前一次性取得尚未知的 names。

本合同不冻结或实现 nested API-set DAG、`ShadowedByEarlierName` positive authority、runtime `LoadLibrary`、live Windows
KnownDLL/API-set/SxS currentness、post-create machine query、pre-resume enforcement、process create/resume、IPC、Store、Ready、
v15、Provider、route、Offer、Capacity、Execution、Attempt、Lease、usage、settlement或 money。

canonical request/resolution plan、policy verification/currentness与 DispatchReady source shape虽已写，真实 authenticated policy
signature verifier/currentness backend、selector、
prelease/recursive parser、GrantReady/recursive resolver、external-directory
owner、grant/candidate/lease backend、positive-consuming advancer、sealer/query/reopen/release/recovery与所有 runtime producer均
`missing`。source contract的存在不能升级 loader predecessor或 process reachability。

## 9. Ready 与零效果

四项 Ready gap逐字保持 `missing`：`node_local_authority_currentness`、`runtime_transition_authority`、
`host_runtime_authority`、`v15_authenticated_session`。

loader exact 18 effects逐字保持 `none`：`runtime_phase`、`runtime_generation`、`runtime_start`、
`runtime_resume`、`runtime_store`、`health`、`readiness`、`node`、`provider`、`route`、`offer`、`capacity`、
`execution`、`attempt`、`lease`、`usage`、`settlement`、`money`。
