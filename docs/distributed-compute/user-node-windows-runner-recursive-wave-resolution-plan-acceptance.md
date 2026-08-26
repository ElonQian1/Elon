---
title: UserNode Windows Runner Recursive Wave Resolution Plan V1 验收草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-recursive-wave-resolution-plan-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Recursive Wave Resolution Plan V1 验收草案

权威合同见
[Recursive Wave Resolution Plan authority](user-node-windows-runner-recursive-wave-resolution-plan-authority.md)。

## 1. 本批证据等级

- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- code acceptance: `passed=0/failed=0`；
- Windows dynamic evidence: `0`；
- persistence: `migration/table/writer=none/none/none`；
- registration: `unregistered_feature_workflow_unavailable`；
- real signer/parser/resolver/backend/advancer/sealer/runtime producers: `missing`。

本验收只审阅 source-only typed shape。用户要求架构铺设阶段暂不编译或真实运行；未运行 Cargo/Rust/source-contract test、
migration、SQLite、网络、设备、Win32 fixture或真实 Runner。静态格式、diff、体积与文档门禁不计为 code/runtime passed。

## 2. 静态责任面

| 合同面 | 未运行的静态审阅目标 |
|---|---|
| A0/Ak split | A0只复用 GrantReady；只有 `k >= 1`存在 per-wave plan V1 |
| request plan V1 | prior receipt/output custody、typed source frontier、outgoing edge/request、symbol、ordered search steps、ranges及独立 digest |
| resolution plan V1 | per-step disposition、exact terminal、earliest producer、route-specific refs、filesystem dedupe与 grant/candidate/lease commitments |
| exact projections | 三个旧 `projected_*` scalar不存在；count/depth由 exact vectors和 prior custody派生 |
| DispatchReady | whole plan validation通过后才允许第一项 dispatch；无 scalar permit或 owner拆件 |
| acquisition V2 | Ak receipt/chain按值保留 immutable typed plan vectors，与 final reverse projection逐项 cross-bind |
| version chain | plan V1、receipt/output V2、receipt-set/chain V1、parse/closure V2、profile V3 |
| producer boundary | 所有成功 producers继续 uninhabited，零 runtime/market effect |

## 3. 未运行的验收断言

1. A0 request/resolution digest均 exact复用同一 GrantReady plan；`Ak (k >= 1)`才持有新的两份 plan V1；`N=0`不产生实例。
2. request plan按 source parse receipt和 importer edge canonical排序，逐项直接绑定 prior receipt/output custody、typed frontier receipt
   evidence、module/global ordinal、kind、locator、name、symbol、ordered search-step ordinals与 forwarder证据；admission、Runner、CWD、
   machine与完整 search-policy lineage只经 authenticated ancestors传递绑定。
3. resolution plan为每个 request保存 exact ordered dispositions与唯一 terminal，并完整覆盖 searched-name、filesystem dedupe、
   earliest producer、typed route ref及后续 grant/candidate/lease request commitments。
4. package/preloaded/KnownDLL只复用 exact typed owner ref；API-set只允许一步 non-recursive host；ordinary filesystem与 SxS
   都必须经 filesystem-backed计划，且只有这两类 route可在 post-grant持有 retained candidate。
5. `projected_next_frontier_parse_receipt_count`、`projected_parsed_image_count`、
   `projected_forwarder_hop_depth`不再作为权威字段；所有 totals/depth由 exact vectors及 base/prior retained chains用 checked
   arithmetic派生。
6. A0 retained parsed-image owner使用独立 prelease parsed-image/package-file/postlease parsed-image三坐标与 exact material
   digests；不得以任意 ordinal相等或 postlease ordinal连续前缀替代 cross-binding，canonical owner vector只要求 postlease ordinal
   严格递增。retained+current forwarder chains必须逐 hop验证 root、importer、
   source/target symbol、target node、连续深度与无环。
7. filesystem requests按 earliest use形成 canonical顺序，primary是首 use；每个 filesystem terminal在全计划中恰好有一个匹配
   module/request/route的 use，final projection回查 directory、component image与 positive outcome provenance。
8. API-set edge直接绑定 contract name；若 host走 filesystem/SxS，则 searched-name、filesystem use/request与 candidate必须绑定
   normalized host module key，而非 contract name。
9. next frontier allocation逐项绑定 target node、earliest producer、target wave、owner ref与 parse receipt ordinal；response race不改变顺序。
10. DispatchReady必须先验证 policy/parser/previous custody/frontier、plans/digests、limits与 owner commitments；其本身不证明任何
   dispatch或 positive outcome。
11. 每份 output receipt提交完整 A0 base-owner set及截至本 producer后的累计 direct-root/forwarder-chain set；final graph独立重建，
    下一 wave从 accumulated vectors重算，禁止空集/子集低报 parsed count或 depth。
12. post-grant candidate的 parent-open、code-integrity、servicing与 namespace-currentness evidence随 positive outcome保留并进入
    candidate-set V2；candidate evidence与最终 image必须绑定同一个 parent-relative open receipt，final projection再与 plan逐字段重验。
13. request/resolution plan不引用 current acquisition receipt、closure、profile或 process context；parse receipt只经 producer
   acquisition ordinal与 chain传递绑定 policy。
14. final sealed slices必须逐项反向验证 forward plans；不得继续以 `parsed_edge_set_digest`或 `wave_digest`冒充 plan digest。
15. `Ak` acquisition receipt以 `WindowsRecursiveWaveDispatchPlanEvidence`按值保留完整 immutable typed vectors，chain按值保留
    ordered receipts；不能只留 detached digests，也不能把 live directory/grant/candidate/lease/parser owner塞入 evidence。
16. acquisition receipt/output与 candidate-set使用 V2；receipt-set/chain仍 V1、parse/closure仍 V2、profile仍 V3，任何版本变化都必须与
    canonical material变化一致。
17. pre-dispatch plan failure保留 whole owner；开始 dispatch后的 failure沿既有 definitive/outcome-uncertain whole-graph custody处理。
18. signer/currentness、parser、resolver、backend、positive advancer、sealer/query与 runtime producers继续 `missing`。

以上断言目前只经过源码审阅，全部不能记为 passed。

## 4. 明确未验收矩阵

| 轴 | passed | failed | unrun | 当前结论 |
|---|---:|---:|---:|---|
| Rust compile / Windows link | 0 | 0 | 1 | 未编译 |
| source-contract test | 0 | 0 | 1 | guard未运行 |
| A0/recursive plan split | 0 | 0 | 1 | source shape only |
| canonical request order/ranges | 0 | 0 | 1 | parser producer missing |
| exact disposition/terminal plan | 0 | 0 | 1 | resolver producer missing |
| filesystem dedupe/route refs | 0 | 0 | 1 | external/live owner backend missing |
| exact-vector derived limits | 0 | 0 | 1 | 未运行 overflow/mutation矩阵 |
| DispatchReady custody | 0 | 0 | 1 | no dispatch producer |
| plan→acquisition→final cross-binding | 0 | 0 | 1 | 未运行 mutation matrix |
| receipt/output V2 version chain | 0 | 0 | 1 | source review only |
| grant/candidate/lease/advancer | 0 | 0 | 1 | missing |
| sealer/query/recovery/runtime | 0 | 0 | 1 | missing |
| Ready / v15 / market | 0 | 0 | 1 | 四 gap missing；18 effects none |

## 5. 未来动态故障矩阵

解除架构阶段禁令后至少覆盖：A0伪造 recursive plan、Ak错误复用 GrantReady、frontier/request/range gap或乱序、duplicate
target与非 earliest producer、normal/delay/forwarder顺序漂移、错 symbol/search step/route、terminal/disposition漏项或多项、
package/preloaded/KnownDLL/API-set/SxS/filesystem route错换、filesystem dedupe collision、三个旧 projected scalar夹带、derived
count/depth overflow、DispatchReady前后 owner拆失、plan/final slice mutation、版本错绑与摘要回环。

## 6. 禁止误报

本批禁止：

- 把 canonical plan或 DispatchReady称为真实 parser、resolver、grant/candidate/lease backend或 positive advancer；
- 把 A0复制成 recursive plan，或把 final reverse projection称为 forward plan；
- 把三个 caller-projected scalar继续当成 signed-limit依据；
- 把 parse receipt描述为直接持有 authenticated policy digest；
- 声称编译、测试、Windows、runtime、process、Ready、Provider、route、Offer、Capacity、Execution、Attempt、Lease、usage、
  settlement或 money effect已经完成。

四项 Ready gap保持 `missing`；loader exact 18 effects保持 `none`；Windows dynamic=`0`；
`migration/table/writer=none/none/none`。
