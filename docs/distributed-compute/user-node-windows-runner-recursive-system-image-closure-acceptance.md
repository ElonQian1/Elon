---
title: UserNode Windows Runner Recursive System-Image Closure V1 验收草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-recursive-system-image-closure-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Recursive System-Image Closure V1 验收草案

权威合同见 [Recursive System-Image Closure authority](user-node-windows-runner-recursive-system-image-closure-authority.md)。
逐波来源合同见
[Recursive System-Image Acquisition Custody authority](user-node-windows-runner-recursive-system-image-acquisition-custody-authority.md)
及其 [acceptance](user-node-windows-runner-recursive-system-image-acquisition-custody-acceptance.md)。forward plan见
[Recursive Wave Resolution Plan authority](user-node-windows-runner-recursive-wave-resolution-plan-authority.md)。

## 1. 本批证据等级

- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- code acceptance: `passed=0/failed=0`；
- Windows dynamic evidence: `0`；
- persistence: `migration/table/writer=none/none/none`；
- registration: `unregistered_feature_workflow_unavailable`；
- policy signature-verifier/currentness/parser/acquisition-backend/positive-advancer/sealer/query/runtime producers: `missing`。

用户要求架构铺设阶段暂不编译或真实运行。本批没有运行 Cargo/Rust/source-contract test、migration、SQLite、网络、设备、Win32
fixture或真实 Runner。rustfmt、diff、体积和文档门禁只属于静态交付卫生；`failed=0` 不表示通过。

## 2. 静态责任面

| 文件/模块 | 未运行的静态审阅目标 |
|---|---|
| `resolution.rs` | `BasePrelease`/`SystemPostLease` stage locator、parsed-image source与 closure field |
| `resolution/system_closure.rs` | 四类 owner、producer-bound parse receipt、wave ranges/limits/fixpoint与 uninhabited producer |
| `system_closure/{digest,projection_digest}.rs` | receipt/wave/closure JCS与从 final edge/name/system-owner slices重算摘要 |
| `system_closure/{source_projection,validation}.rs` | earliest-producer frontier、owner splice拒绝、no-delay、一次性 receipt、prefix+suffix count closure |
| `system_closure/{edge_order,edge_projection}.rs` | deterministic wave merge、global root import、node+symbol forwarder continuity/cycle/depth与 new-owner use |
| `exact_context_plan/recursive_policy.rs`、`system_closure/acquisition{,/custody,/failure,/digest,/validation}.rs` | authenticated policy、nonempty producer→target / terminal None、whole-owner partial custody与 acquisition→projection cross-binding；所有 producer missing |
| `acquisition/{plan,plan_digest,plan_forwarder_validation,plan_owner_validation,plan_projection,plan_validation}.rs` | A0复用 GrantReady、Ak canonical request/resolution plan V1、exact-vector currentness-pending→DispatchReady与 V3 receipt/output |
| `pe_graph_validation/image_source.rs` | owner binding与 immutable byte/section material identity分离，防 parser receipt替换绕过 dedupe |
| `grant_ready/{final_projection,search_projection}.rs` | GrantReady只重证 final base prefix，recursive suffix交由独立 closure反向覆盖 |
| `digest{,/material,/pe_cross_binding}.rs` | PE V2与 resolution profile V3 canonical material |
| `exact_context_plan/lineage.rs` | QueryVerified重新验证 base+recursive closure并投影 profile V3 |
| `runtime_loader_load_set_source_contract_tests.rs` | 新 guard 源码已写但未运行 |
| `runtime_loader_recursive_wave_custody_source_contract_tests.rs` | policy/per-wave custody source-shape guard已写但未运行 |

## 3. 未运行的验收断言

1. wave-zero package-only preliminary/GrantReady entries只能是 `BasePrelease` prefix，suffix只能是 `SystemPostLease`。
2. 每个 recursive parsed image只有一个 receipt；receipt直接绑定 parser policy与 exact byte/section identity，并经
   `producer_acquisition_receipt_ordinal`、plan和 chain传递绑定 authenticated recursive policy；
   `producer_module_request_ordinal`指向上一波 earliest target-producing binding。
3. owner必须逐项匹配 package lease、authenticated preload、KnownDLL record或 resolved filesystem custody，不能从 final graph
   任取同 node owner。
4. frontier由实际 final targets反推；receipt不得延迟、重复、跨 wave使用，terminal frontier必须为空。
5. module/name/system-owner ranges连续、无重叠，并与 final总数双向闭合；future owner ref与未使用 new owner均拒绝。
6. wave edge merge按 source receipt再按 importer graph；direct import与 forwarder locator ordinal/证据/symbol均精确。
7. forwarder root可来自 base、earlier wave或同 wave较早 direct edge；逐跳 module+symbol连续、ordinal无 gap，
   forwarder/future root、symbol cycle与 depth超限拒绝。
8. 每 wave edge/name/system-owner摘要从 final slice重算；system edge包含 binding ordinal与完整 resolution origin。
9. parsed-image source binding、material identity、import-table digest/counts与 final graph一致；相同 material不得换 receipt重解析。
10. loader profile V3包含 closure digest；GrantReady digest、closure digest与 process pre-create required profile不得自引用。
11. recursive plan使用V1，parse receipt使用V2 acquisition ordinal，acquisition output/receipt使用V3并承诺逐波currentness，receipt-set/chain保持
    V1，closure保持 V2；outer profile V3只经 closure digest吸收变化，未静默增加字段或改写其 hash material。
12. A0只复用 GrantReady；Ak forward plan与 final reverse slices独立并逐项 cross-bind，三个旧 `projected_*` scalar由 exact
    vectors派生；whole-plan验证后必须进入 currentness-pending并消费 point-of-use authorization；Ak receipt/chain按值保留完整
    immutable typed plan evidence，DispatchReady不构造 backend authority。
13. nested API-set DAG与 Shadow positive path仍 fail-closed；一步 API-set terminal不等于 nested DAG。
14. policy、canonical plans与逐波 custody source contract已写；policy signature verifier/currentness backend、parser/resolver、acquisition backend、positive advancer、closure producer、sealer/query与
    所有 runtime producer仍不可构造。

这些断言目前只经过源码审阅，全部不能记为 passed。

## 4. 明确未验收矩阵

| 轴 | passed | failed | unrun | 当前结论 |
|---|---:|---:|---:|---|
| Rust 编译 / Windows 链接 | 0 | 0 | 1 | 未编译，类型/可见性/Win32 feature未由 compiler证明 |
| source-contract Rust test | 0 | 0 | 1 | guard已写但未运行 |
| wave-zero prefix / recursive suffix | 0 | 0 | 1 | source validator已写，无构造 producer |
| parse receipt / exact producer owner | 0 | 0 | 1 | source projection已写，无 retained-handle recursive parser |
| frontier no-delay / receipt one-use / empty fixpoint | 0 | 0 | 1 | 未运行错 wave、gap、duplicate与 nonempty frontier矩阵 |
| deterministic edge merge | 0 | 0 | 1 | 未运行跨 importer/wave乱序矩阵 |
| forwarder root + node/symbol chain | 0 | 0 | 1 | 未运行 base-root、earlier-wave、gap、cycle与 limit矩阵 |
| final edge/name/system-owner digest recompute | 0 | 0 | 1 | source recompute已写，无 mutation test |
| byte/section material dedupe | 0 | 0 | 1 | 未运行 alias、same bytes/different receipt与 generation drift |
| per-wave grant/candidate/lease custody | 0 | 0 | 1 | source contract written；backend/positive advancer producer missing |
| canonical request/resolution plans / currentness-pending→DispatchReady | 0 | 0 | 1 | source written；currentness/parser/resolver producer missing |
| recursive sealer/query/currentness | 0 | 0 | 1 | missing |
| nested API-set / Shadow positive | 0 | 0 | 1 | fail-closed，未实现 |
| live OS / post-create machine | 0 | 0 | 1 | sealed echoes不是 live observation |
| pre-resume / dynamic `LoadLibrary` | 0 | 0 | 1 | enforcement missing |
| runtime Store / recovery | 0 | 0 | 1 | migration/table/writer均 none |
| Ready / v15 / Provider / market / money | 0 | 0 | 1 | 四 gap missing，loader 18 effects none |

## 5. 未来动态故障矩阵

解除架构阶段禁令后，至少覆盖：

- base与suffix stage互换、ordinal gap/overlap、错误 prefix count与 future-wave edge；
- missing/duplicate receipt、错 authenticated policy/limit、错 producer/target wave或 producer request、owner splice、同 node不同 owner、same material不同 receipt；
- frontier提前/延迟/重复、target多路径 earliest-producer选择、terminal nonempty与 wave/image/request limit；
- package/preloaded/KnownDLL/filesystem owner的 wrong identity/generation/section/receipt，partial acquisition与 positive+negative同返；
- wave merge乱序、descriptor/thunk重复、normal/delay/forwarder交错、root指向 forwarder或未来 edge；
- base-root、earlier-wave与 same-wave-earlier-direct forwarder，错 source export/target symbol、hop gap/duplicate、
  forwarder/future root、symbol cycle与 depth上限；
- edge/name/system-owner slice字段 mutation、binding ordinal/origin/ref omission与 closure/profile digest mutation；
- nested API-set、Shadow、cache collision、servicing/namespace generation drift、release/recovery与 crash parking；
- live OS currentness、post-create machine query、pre-resume/dynamic-load drift和真实 Windows process path。

## 6. 禁止误报

本批禁止：

- 把 final projection envelope称为真实 recursive resolver、完整 acquisition authority或 runtime proof；
- 把 SHA-shaped receipt、profile V3或 pre-create echo称为 live OS/currentness evidence；
- 把逐 wave source contract称为真实 grant/candidate/lease/negative backend、positive advancer或运行 custody；
- 把一步 API-set host称为 nested API-set DAG，或把 rejected Shadow variant称为 positive path；
- 声称编译、Rust test、SQLite、Windows、网络、设备、真实 Runner或生产验收已完成；
- 生成或声称 Runtime、Ready、Provider、route、Offer、Attempt、Lease、usage、settlement、money effect。

四项 Ready gap保持 `missing`；loader exact 18 effects保持 `none`；Windows dynamic=`0`；
`migration/table/writer=none/none/none`。
