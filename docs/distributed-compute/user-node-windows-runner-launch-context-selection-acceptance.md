---
title: UserNode Windows Runner Launch Context Selection V1 验收草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-launch-context-selection-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Launch Context Selection V1 验收草案

权威合同见 [Launch Context Selection authority](user-node-windows-runner-launch-context-selection-authority.md)。

## 1. 本批证据等级

- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- code acceptance: `passed=0/failed=0`；
- Windows dynamic evidence: `0`；
- persistence: `migration/table/writer=none/none/none`；
- real selector/parser/grant-ready resolver/external-directory/grant/lease positive-consuming/final sealer/query/live-machine
  producers: `missing`。

没有运行编译、Cargo/Rust test、source-contract test、migration、SQLite、网络、设备、Win32 fixture 或真实 Runner。所有 guard
只是未运行的源码审阅目标；`failed=0` 不表示通过。

## 2. 静态责任面

| 文件/模块 | 静态审阅责任 |
|---|---|
| `runtime_loader_load_set/launch_path_discovery/exact_context_plan.rs` | uninhabited intent、whole-owner success/failure facade |
| `exact_context_plan/{intent,binding,digest,edge_locator,lineage}.rs` | nested policy重算、typed edge locator、pre/post cross-binding、QueryVerified lineage与 pre-create projection |
| `launch_path_discovery/prelease_pe_material{,/digest,/closure}.rs` | package-image pre-lease base imports + separate forwarder hops/closure与 canonical digest；该阶段本身不解析 system image或预测 lease generation |
| `runtime_loader_load_set/{failure,resolution}.rs`、`resolution/grant_ready{,/validation,/search_projection,/final_projection}.rs` | preliminary→GrantReady exact terminal/disposition→grants/leases→post-lease final 的线性 owner/failure custody与 final projection |
| `resolution/system_closure{,/digest,/edge_order,/edge_projection,/projection_digest,/source_projection,/validation}.rs`、`pe_graph_validation/image_source.rs` | source-only recursive final envelope：producer-bound owner、frontier/fixpoint、deterministic suffix与 final slice重算；真实逐波 producer缺失 |
| `runtime_loader_load_set/{digest,validation,model}.rs` | loader resolution profile V3绑定 selector+request plan+grant-ready plan+recursive closure |
| `runtime_process_custody/{launch_security,model,policy}.rs` | resolution外的 expected-context bridge、V3重算与相等性检查 |
| `runtime_loader_*source_contract_tests.rs` | 未运行 source-shape guards |

## 3. 静态源码审阅目标

1. selector只允许 `PackageRoot` 或 exact `PlanDirectory { ordinal, relative_path }`，没有 package-root/Runner-parent/first-
   directory fallback；authenticated producer以 private `Infallible` 保持不可达。
2. application file、application directory、package root与 CWD 分离；application-directory identity/component-set/receipt来自同一
   package-root或 exact plan-directory candidate；Plan selector复核 ordinal/path/identity/component/receipt。
3. machine/WOW64、empty environment、no inherited handles、creation flags、DLL route/search policy与 launch-security expectation均从
   实际 nested fields重算 digest；expectation不冒充真实 token authority。
4. pre-lease material区分 parsed-image ordinal与 package-file ordinal；每个 image逐 ordinal绑定 plan/evidence/retained
   path/digest/size/FileId；Runner按 package-file ordinal定位后再绑定 parsed root。
5. normal/delay base imports均有 symbol name/ordinal、descriptor/thunk；forwarder作为 exact source-edge 上的独立 hop，具有
   source-export/target/hop evidence；canonical order与 retained package-image reachable/cycle/depth/cache-collision closure齐全，
   不声称 pre-lease阶段已经解析 system image；独立 post-lease recursive final envelope只冻结 source contract。
6. preliminary request plan只为每条 import edge生成 ordered search-step refs与 unresolved状态；external目录只要求 typed owner；
   package lease refs覆盖全部 plan files，system lease refs在 exact terminal resolution与 dedupe前不存在。
7. success/failure整体保留 discovery+intent+PE material；无 Clone/Copy/Serde、raw handle/File/path、成功 `into_parts` 或 retry
   extractor。
8. GrantReady contract必须消费 whole request owner，绑定 exact terminal/disposition/external owner/resolved-system set、KnownDLL
   mapping+generation、non-recursive API-set host、leaf digests与 exact final search projection；`ShadowedByEarlierName` producer
   fail-closed，其真实 producer当前 `missing`，request plan不能进入 name-grant dispatch。
9. name-grant、content-lease、borrow-only与 namespace-query failure分别持有其 exact whole-stage owner；typed positive receipt保留
   request/nonce/response bytes+digest，但 name/system positive-consuming transition仍由 `Infallible` 阻断；final resolution只在全部
   grants/leases之后出现。
10. GrantReady只对应 final base prefix；recursive envelope以 stage-explicit suffix、earliest producer owner、frontier/fixpoint与
    final edge/name/system-owner slice重算闭合其余 projection，但逐 wave custody/advancer/sealer producer仍 missing。
11. loader profile V3包含 selector、preliminary request-plan、grant-ready plan与 recursive closure digest，不包含 required process digest；process
    required-context V3绑定 selector+final resolution，expected digest由 resolution外 launch-security bridge携带并比较。
12. prelease parsed image/import edge到 postlease parsed image/import binding具有 exact typed cross-binding；QueryVerified lineage到
    process的输出命名为 pre-create projection，不是实际 OS machine/WOW64 query receipt。
13. 四项 Ready gap仍 `missing`，loader exact 18 effects仍 `none`。

## 4. 明确未验收矩阵

| 轴 | passed | failed | unrun | 当前结论 |
|---|---:|---:|---:|---|
| Rust compile/check | 0 | 0 | 1 | 未编译 |
| Rust/source-contract tests | 0 | 0 | 1 | guards未运行 |
| selector signature/provenance/currentness | 0 | 0 | 1 | producer missing |
| exact CWD ordinal/path/identity membership | 0 | 0 | 1 | 未运行 |
| application-directory/package-root/CWD separation | 0 | 0 | 1 | 未运行 |
| pre-lease normal/delay parse + separate forwarder hops | 0 | 0 | 1 | parser producer missing；仅 package closure |
| symbol/descriptor/thunk/forwarder-hop evidence | 0 | 0 | 1 | 未运行 |
| reachable/cycle/depth/canonical merge/cache closure | 0 | 0 | 1 | 未运行 |
| ordered route/search/name/component unresolved request plan | 0 | 0 | 1 | 未运行 |
| exact terminal/disposition/external owner GrantReady contract | 0 | 0 | 1 | source written；resolver producer missing |
| complete package lease refs/system terminal dedupe | 0 | 0 | 1 | source review / resolver missing |
| positive name/system outcome consumption | 0 | 0 | 1 | typed receipt/custody written；consuming transitions `Infallible` |
| pre/post image + import-edge cross-binding / QueryVerified lineage | 0 | 0 | 1 | source review only；sealer/query producers missing |
| recursive system-image final projection envelope | 0 | 0 | 1 | prefix/suffix、owner/frontier/fixpoint/digest source written；parser/逐波 custody/advancer/sealer missing |
| post-create process machine/WOW64 query-back | 0 | 0 | 1 | missing；pre-create projection不是 actual receipt |
| failure whole-owner custody | 0 | 0 | 1 | 未故障注入 |
| acyclic selector→request→grant-ready→final→process digest chain | 0 | 0 | 1 | source review only |
| Windows retained-handle dynamic matrix | 0 | 0 | 1 | 未运行 |
| grants/leases/post-lease final seal/query/reopen | 0 | 0 | 1 | `out_of_scope_missing` |
| process/Runtime/Ready/Provider/economic effects | 0 | 0 | 1 | `out_of_scope_missing` |

## 5. Ready gaps 与 effects

四项 gap 必须逐字为 `missing`：

```text
node_local_authority_currentness
runtime_transition_authority
host_runtime_authority
v15_authenticated_session
```

loader 18 项 effects 必须逐字为 `none`：

```text
runtime_phase, runtime_generation, runtime_start, runtime_resume, runtime_store,
health, readiness, node, provider, route, offer, capacity, execution, attempt,
lease, usage, settlement, money
```

## 6. 负向验收与下一门

本批不得声称 selector/parser producer、selected runtime CWD、GrantReady resolver、external directory authority、grant、
lease、真实 system recursive producer/nested API-set DAG、final PE graph/launch path/resolution、actual process machine context、process
create/resume、Runtime、Ready 或结算已实现。源码铺设可继续；生产可达必须
先完成 extraction-share/discovery Windows 动态证据，再按 selector+parser → request plan → exact terminal/disposition resolver →
grants → leases → same-handle post-lease seal → query/reopen/currentness 顺序留下独立动态验收。
