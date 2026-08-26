---
title: UserNode Windows Runner Recursive System-Image Acquisition Custody V1 验收草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-recursive-system-image-acquisition-custody-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Recursive System-Image Acquisition Custody V1 验收草案

权威合同见
[Recursive System-Image Acquisition Custody authority](user-node-windows-runner-recursive-system-image-acquisition-custody-authority.md)。

## 1. 本批证据等级

- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- code acceptance: `passed=0/failed=0`；
- Windows dynamic evidence: `0`；
- persistence: `migration/table/writer=none/none/none`；
- registration: `unregistered_feature_workflow_unavailable`；
- authenticated policy signer/currentness、selector、prelease/recursive parser、GrantReady/recursive resolver、external-directory
  owner、grant/candidate/lease backend、positive-consuming advancer、sealer/query/reopen/release/recovery及所有 runtime producers:
  `missing`。

本验收只审阅 source-only typed shape。用户要求架构铺设阶段暂不编译或真实运行；本批没有运行 Cargo/Rust/source-contract
test、migration、SQLite、网络、设备、Win32 fixture或真实 Runner。rustfmt、diff、体积和文档门禁只属于静态交付卫生；
`failed=0`不表示通过。

## 2. 静态责任面

| 文件/模块 | 唯一的未运行源码责任 |
|---|---|
| `runtime_loader_load_set/launch_path_discovery/exact_context_plan/recursive_policy.rs` | 独立 signed policy、六项 exact limits、borrow-only projected-total gate、private `Infallible` producer |
| `runtime_loader_load_set/resolution/system_closure/acquisition.rs` | wave request/resolution plan、acquisition receipt、sealed acquisition chain 与 closure cross-binding facade |
| `system_closure/acquisition/custody.rs` | whole accumulated graph、pre-dispatch signed-limit gate 与 request/grant/candidate/lease/same-owner-parse/completed/terminal linear stages |
| `system_closure/acquisition/failure.rs` | post-dispatch definitive 与 outcome-uncertain whole-graph failure custody |
| `system_closure/acquisition/digest.rs` | output/receipt/set/chain及 grant/candidate/parse/terminal canonical digest material |
| `system_closure/acquisition/validation.rs` | producer/target wave、range、frontier、receipt、owner set 与 final projection cross-binding validators |
| `system_closure/validation.rs` | final cumulative signed-limit、parse/frontier/fixpoint 与 acquisition-chain projection validation |
| `runtime_loader_recursive_wave_custody_source_contract_tests.rs` | 未运行 source-shape guard；文件存在不等于测试通过 |

| 合同面 | 未运行的静态审阅目标 |
|---|---|
| authenticated policy | direct context/plan/parser/preloaded/routes/limits fields、经 digest传递的 admission/machine/search lineage与独立签名域 |
| wave coordinates | base producer 0；nonempty时 producer `k`→target `Some(k+1)`，terminal `A_N`→`None` |
| canonical queue | earliest producer、连续 receipt/range分配、backend completion order不影响 final graph |
| whole-owner state | prior graph、namespace session、grants/owners/leases、active/pending refs按值移动，无 scalar重建 |
| route-specific acquisition | package/preloaded/KnownDLL typed reuse；filesystem retained candidate→positive outcome→lease/section |
| same-owner parse | exact immutable owner/material/policy/generation与 producer request绑定，拒绝跨代 splice |
| wave evidence | acquisition receipt以 response/owner-set digests与 projection ranges、parse receipts、next frontier逐项 cross-bind；raw bytes留在 custody |
| failure custody | exact negative、positive+negative/invalid/timeout uncertain、partial owners与 response bytes完整保留 |
| final aggregate | empty frontier后才可封印；完整 namespace原子性属于 final aggregate/query而非递归前预取 |

上述文件存在只证明本批 source shape落点；不构成 producer、compiler、test、runtime或 Windows证据。

## 3. 未运行的验收断言

1. recursive policy使用独立 schema/version/signature链，直接签 exact context-intent、preliminary plan、parser、preloaded、
   ordered routes与六项 limits；admission/manifest/machine/search只经已验证 context/plan digests传递，不重复冒充直接字段。
2. launch-context payload V1未被静默扩展；unknown policy version/field和签名/verification错绑失败关闭。
3. wave count只计 recursive waves；parsed/module/name/system-owner limits覆盖 final cumulative totals；forwarder limit按最大 depth。
4. 若 final projection有 `N`个 recursive waves，chain恰有 `N+1`份 `A0..AN` receipts；A0绑定 base、Ak绑定 wave k，
   base/producer acquisition只有在 next frontier非空时才产生 target wave `Some(k+1)` owner；terminal `A_N`必须为空并使用
   `target_parse_wave_ordinal=None`，`N=0`时 `A0=A_N`，不得制造空 parse wave。
5. target按 earliest final module-request ordinal去重并分配连续 parse-receipt ordinal；response race不能改变顺序。
6. module/name/system-owner ranges连续且使用 checked arithmetic；不同 ordinal domain不能按 vector index互换。
7. every wave advancer消费 whole prior state；没有 Clone/Serde/raw handle/path/成功拆件或 retry scalar。
8. package/preloaded/KnownDLL复用 exact typed owner；只有 ordinary filesystem消费 retained candidate并取得 immutable lease/section。
9. already parsed target不重复 acquisition/parse；cache key绑定不同 identity/route失败关闭。
10. filesystem parse使用 exact leased handle/section；所有 route的 parse receipt绑定同一 authenticated policy与 exact owner material。
11. acquisition receipt绑定 prior state、frontier、requests、terminal/dispositions、response/owner-set digests、owner/lease
    transitions、parse allocation与 next frontier，并与现有 projection wave逐项相等；raw response bytes与 positive owners只留在
    成功线性或失败 custody，不嵌入 receipt。
12. policy V1、parse receipt V2/owner sets及 plan/prior custody共同单向进入 acquisition receipt/set/chain V1，再进入 closure V2
    与既有 profile V3；parse receipt只保存 producer acquisition ordinal，acquisition digest不反向引用 closure/profile/required
    context，摘要 DAG无环。
13. dispatch前 failure才能返回 borrow-only owner；dispatch后所有 failure保留 whole graph、active/pending与 returned outcomes。
14. definitive negative必须与 exact session/attempt/request/nonce/candidate/material匹配且无 positive；其余均 uncertain。
15. positive+negative、positive-invalid、timeout、missing/malformed response不能丢弃 positive owner或 response bytes/digest。
16. empty frontier前不能形成 final closure/profile；per-wave contract不构造 sealer/query/runtime producer。
17. nested API-set DAG、Shadow positive、runtime module load与所有 Runtime/Ready/market效果继续 fail-closed或 missing。

以上断言目前只经过源码审阅，全部不能记为 passed。

## 4. 明确未验收矩阵

| 轴 | passed | failed | unrun | 当前结论 |
|---|---:|---:|---:|---|
| Rust compile / Windows link | 0 | 0 | 1 | 未编译，type/borrow/Win32 feature未由 compiler证明 |
| source-contract Rust test | 0 | 0 | 1 | guard已写但未运行 |
| signed recursive policy provenance | 0 | 0 | 1 | source contract存在，signer/currentness producer missing |
| six exact policy limits | 0 | 0 | 1 | 未运行 zero/overflow/base+suffix/depth mutation矩阵 |
| producer-wave→target-wave cross-binding | 0 | 0 | 1 | source shape only；无 positive advancer |
| earliest-producer receipt allocation | 0 | 0 | 1 | 未运行 multi-target/race/permutation矩阵 |
| base A0→wave 1 / terminal None | 0 | 0 | 1 | nonempty才到 wave 1；selector/GrantReady/grant/lease producers missing |
| per-wave grants / candidates / leases | 0 | 0 | 1 | custody contract source-written；backend/positive transitions missing |
| route-specific owner reuse | 0 | 0 | 1 | 未运行 package/preloaded/KnownDLL/filesystem矩阵 |
| same-owner recursive parse | 0 | 0 | 1 | parser producer missing；未运行 handle/section/generation splice矩阵 |
| partial acquisition quarantine | 0 | 0 | 1 | 未运行 wrong negative、positive+negative、timeout与 crash parking |
| acquisition→projection cross-binding | 0 | 0 | 1 | source validator已写，无 mutation test |
| terminal empty frontier / final aggregate | 0 | 0 | 1 | final sealer/query producer missing |
| release / recovery | 0 | 0 | 1 | explicit authorized backend不存在 |
| nested API-set / Shadow positive | 0 | 0 | 1 | fail-closed，未实现 |
| live OS / pre-resume / dynamic load | 0 | 0 | 1 | missing |
| Runtime Store / Ready / v15 / market | 0 | 0 | 1 | migration/table/writer none；四 gap missing；effects none |

## 5. 未来动态故障矩阵

解除架构阶段禁令后，至少覆盖：

- policy schema/version、signature/key generation、admission/context/search/machine/parser错绑及六项 limit逐字段 mutation；
- base producer冒充 recursive wave、producer/target off-by-one、future owner、delayed receipt与 terminal nonempty；
- 多 target首次到达、同 target多路径、response乱序、receipt permutation、range gap/overlap/overflow；
- package/preloaded/KnownDLL/filesystem route错换、already-parsed重复取得、cache-key collision；
- wrong owner/session/attempt/request/nonce/candidate/FileId/section/generation negative与 positive-invalid；
- positive+negative同返、timeout、partial acquisition、parser failure、crash parking及 response bytes/digest mutation；
- same handle/section、servicing/lease generation drift、writable mapping、rename/swap/reparse/hardlink；
- acquisition receipt字段/range/frontier mutation、projection wave mismatch及 closure/profile digest mutation；
- final aggregate session/generation drift、release/recovery、nested API-set、Shadow与 runtime load fail-closed。

## 6. 禁止误报

本批禁止：

- 把 source-only policy称为 real signer/current policy或可用配置；
- 把 custody type/acquisition receipt称为 grant、candidate、lease、parser、advancer、sealer或 query backend；
- 把 per-wave source contract称为真实 recursive resolver、runtime closure或 Windows proof；
- 把 final projection `wave_digest`称为 acquisition custody evidence；
- 声称 nested API-set DAG、Shadow positive、live OS、pre-resume或 dynamic-load authority已实现；
- 声称编译、Rust test、SQLite、Windows、网络、设备、真实 Runner或生产验收已完成；
- 生成或声称 Runtime、Ready、Provider、route、Offer、Capacity、Execution、Attempt、Lease、usage、settlement、money effect。

四项 Ready gap保持 `missing`；loader exact 18 effects保持 `none`；Windows dynamic=`0`；
`migration/table/writer=none/none/none`。
