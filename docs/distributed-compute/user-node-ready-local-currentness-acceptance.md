---
title: UserNode Ready 本机当前性封印 V1 验收草案
status: draft
reviewed_at: 2026-08-27
owners: node, compute
proposed_feature_id: compute-user-node-ready-local-currentness-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_draft_uncompiled
verification_status: source_review_only
---

# UserNode Ready 本机当前性封印 V1 验收草案

## 1. 本批结论

本批只写入不可达的 source seam 与静态合同 guard。用户要求架构铺设阶段不编译、不运行、不执行 migration、
SQLite、设备或网络验证，因此固定：

- implementation=`source_written/source_review_only/implementation_uncompiled`；
- runtime=`implementation_unrun`；
- code acceptance=`passed=0/failed=0`；
- persistence=`migration/table/writer=none/none/none`；
- production producer=`missing_handle_bound_vfs_open`。

`failed=0` 仅表示没有执行失败项，不表示通过。

## 2. 文件与责任

| Owner | 文件 | 责任 |
|---|---|---|
| local authority | `local_authority/ready_source_currentness.rs` | 私有无调用点 prover、custody、Deferred/query-only 快照、Ready successor/currentness 审计和 lifetime/thread seal |
| work-admission owner | `work_admission_store/planning.rs`、`current.rs` | exact current head/chain/readback 与历史 install/promotion receipt 审计，不套用 stopped-only current validator |
| Ready owner | `ready_capability.rs` | 用 fresh authenticated time 重验 exact Ready publication record 和 health TTL |
| static guard | `user_node_ready_source_lineage_source_contract_tests/currentness.rs` | 未运行的源码合同边界 |
| authority | 本页与对应 authority | 输入、顺序、缺口、零效果和后续门 |

## 3. 静态审阅目标

源码应满足：

1. 唯一 prover 属于 `OpenedComputePluginLocalAuthority`，保持模块私有、无调用点、固定返回 `Result<()>`，不接受路径或裸
   Connection；
2. 显式要求同 authority instance 的 process fence 与 fresh authenticated trusted time；
3. source guard 同时钉住 `with_deferred_read` owner 的 `query_only=ON`、Deferred、commit、restore marker；currentness
   模块没有 `with_immediate`、SQL INSERT/UPDATE/DELETE 或 schema 变更；这些仍只是未运行的源码证据；
4. current work-admission head、generation、digest、完整 predecessor chain 与传入线性 owner exact 相同；
5. 原 install/promotion receipt pair 仍由 owner table 重审；
6. current sharing authorization、policy/profile/catalog/keyrings/target/Host API、process epoch、inventory revision 与 exact
   Ready record 逐字段闭合，fresh time 严格晚于 retained admission post-rehash barrier；
7. Ready health 在 fresh trusted now 下重算 digest并保持未过期；
8. source-lineage builder 在同一事务内重新执行 Host/resource/CPU-only 等式；
9. guard 精确检查 private module route、截取 seal definition 检查字段私有、拒绝 currentness 文件任意 `pub*` item 与
   descendant module，并截取 prover signature 钉住固定 `Result<()>` 与 root 文件名称引用计数；seal 无 Clone/Serde，
   以 HRTB lifetime 禁止外逃，并以 `Rc` phantom owner 禁止 Send/Sync；
10. 原 V1 envelope 和四项 gap 不修改，剩余三项仍明确 missing；
11. 无 Ready/execution capability/dispatch 构造器，无 Provider/route/Offer/Job/Lease/Receipt/economic effect；
12. Opened authority 仍无生产构造器，本批不伪造测试或 path-open producer。

这些目标当前只有 source-written 审阅状态；Rust guard 不执行就不记通过数。

## 4. 未运行矩阵

| 轴 | passed | failed | unrun | 当前结论 |
|---|---:|---:|---:|---|
| Rust 编译 | 0 | 0 | 1 | 未编译 |
| source-contract Rust guard | 0 | 0 | 1 | 仅写入源码 |
| Deferred/query-only SQLite | 0 | 0 | 1 | 无运行证据 |
| callback panic + commit/restore fault | 0 | 0 | 1 | 正常收尾路径仅有源码形状；双故障 payload 不承诺保留 |
| head/chain/receipt tamper | 0 | 0 | 1 | 无故障注入 |
| sharing/policy/process/inventory drift | 0 | 0 | 1 | 无故障注入 |
| Ready health expiry/time rollback | 0 | 0 | 1 | 无故障注入 |
| handle-bound VFS/open | 0 | 0 | 1 | producer 不存在 |
| runtime/Host/v15 | 0 | 0 | 1 | 三项 authority 缺失 |
| Provider/route/Offer/Job/Lease/Receipt | 0 | 0 | 1 | effects=none |
| device/network/production | 0 | 0 | 1 | 未运行 |

## 5. 负向验收

以下任一情况均为失败：

- 用 v14 Planning projection/custody 构造本 seal；
- 接受普通路径、Connection、墙钟、裸 digest 或 caller boolean；
- 把 admission 时的 stopped-only current validator 当作 Ready successor validator；
- work-admission head 已前移仍接受历史 pair，或跳过 predecessor/install/promotion owner audit；
- process owner、sharing authorization、policy、inventory 或 exact Ready record 漂移后仍返回 seal；
- 把 `Untrusted...HostRuntimeObservation` 当 Host authentication/enforcement；
- 让 seal 逃出 callback、可 Clone/Serde，或把 currentness 写进可外逃的 V1 envelope；
- 把私有 prover/getter/seal 对其他模块开放、增加生产调用点、恢复泛型 owned 返回值，或允许 seal 跨线程；
- callback 在 post-check 前执行 I/O、writer、发布、调度、网络或设备副作用；或声称 commit/restore 失败时仍保留原 panic；
- 未建立扫描上限/缓存/checkpoint 和性能证据前，把 O(history) exact-chain 回放接入高频发布热路径；
- 构造 Ready、execution capability、Provider、route、Offer、Job、Attempt、Lease、Receipt 或资金效果；
- 宣称本批已编译、测试、运行、迁移或生产验收。

## 6. 下一验收门

解除架构阶段禁令后，先补 handle-bound VFS/open producer，再以真实 SQLite 运行 exact head、合法 Ready successor、
authorization/policy/process/inventory drift、health expiry/time rollback、transaction callback 不外逃、panic/query-only 恢复、
panic 与 commit/restore 双故障和前后 custody 失效矩阵。
随后仍必须独立完成 runtime transition、Host runtime 和 v15/session，才能进入 server-owned Ready verifier。
