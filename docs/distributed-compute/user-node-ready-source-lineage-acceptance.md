---
title: UserNode Ready 源谱系 V1 验收草案
status: draft
reviewed_at: 2026-08-25
owners: node, compute
proposed_feature_id: compute-user-node-ready-source-lineage-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_draft_uncompiled
verification_status: source_review_only
---

# UserNode Ready 源谱系 V1 验收草案

## 1. 本批结论

本批是 **unregistered source draft**，状态固定为：

- design: `draft_frozen`；
- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- code acceptance: `passed=0/failed=0`；
- persistence: `migration/table/writer=none/none/none`。

用户要求在架构铺设阶段不编译、不运行、不执行 migration 或真实验证。因此源码合同 guard 只随源码写入，未作为
Rust test 执行；格式化、文本检索、diff、体积和模块化检查即使通过，也只能作为静态交付卫生，不能提高运行成熟度。

## 2. 本批文件与责任

| Owner | 文件 | 责任 |
|---|---|---|
| shared contract | `server/src/compute_federation/user_node_ready_source_lineage.rs` 及叶模块 | 六键 envelope、Host observation 自一致摘要、来源等式、CPU-only 与九项下游零效果 |
| node owner adapter | `server/src/node_agent_compute_plugin_host/ready_source_lineage_projection.rs` | 从线性 work-admission 与 Ready-health owner token 读取来源，不生成 Ready |
| server source review | `server/src/compute_federation/user_node_ready_source_lineage_source_contract_tests.rs` | 未运行的源码合同 guard；固定 ABI、owner source、负边界和证据状态 |
| authority | `user-node-ready-source-lineage-authority.md` | 权威、等式、缺口与后续顺序 |

同一个 shared contract 源文本由 server 的 `compute_federation` 正常路由，并由 Node Host 通过显式叶文件 `#[path]`
复用；没有复制第二份 schema 或摘要实现。两个 binary/逻辑 module 会各自编译该文本，Rust 类型身份不可跨 target 直接
互换，未来 wire 仍必须使用 canonical envelope。

## 3. 静态源码审阅目标

源码应满足：

1. envelope 只有 schema、kind、digest、canonicalization、algorithm、lineage 六键；
2. lineage 和 untrusted Host observation 使用不同 domain-separated JCS/SHA-256；
3. parser 要求 canonical JSON、精确 digest 与 `deny_unknown_fields`；
4. Node adapter 同时读取 `DurableWorkAdmittedPluginSlot` 与 `ValidatedComputeReadyPublication`；
5. Host runtime 输入逐字命名 `Untrusted...`，输出逐字命名 `Projected...`；
6. work-admission、Ready health、Runner、grant、generation、inventory 与时间区间逐字段闭合；
7. CPU-only 不虚构 accelerator，Host 自报资源不能超过 signed grant；
8. local currentness、runtime transition、Host runtime、v15 session 四项 authority gap 与九项下游 zero-effect 不可省略；
9. 无 Ready/Verified execution capability 构造器；
10. 无 Store、SQL、migration、API、MCP、Wire 或状态写入。

这些目标当前只有 source-written 状态；在用户解除架构阶段禁令并运行相应测试前，不记通过数。

## 4. 明确未验收矩阵

| 轴 | passed | failed | unrun | 当前结论 |
|---|---:|---:|---:|---|
| Rust 编译 | 0 | 0 | 1 | 未编译 |
| source-contract Rust test | 0 | 0 | 1 | 仅写入源码 |
| migration / SQLite | 0 | 0 | 1 | 本批无 migration/table/writer |
| Node Host runtime | 0 | 0 | 1 | 无 Sidecar/IPC/enforcement producer |
| endpoint v15 session | 0 | 0 | 1 | 尚无 capability/session/ledger |
| Ready builder/signature/upload | 0 | 0 | 1 | 不存在 |
| server Ready verifier/Store | 0 | 0 | 1 | 不存在 |
| HTTP/MCP/Wire | 0 | 0 | 1 | 本批无入口 |
| Provider/route/Offer/Attempt/Lease | 0 | 0 | 1 | 全部 effect=none |
| device/network/production | 0 | 0 | 1 | 未运行 |

`failed=0` 只表示没有执行失败项，不表示通过。

## 5. 负向验收

以下任一声明均为失败：

- 把 `Projected...Lineage` 称为 ReadyCapability；
- 把 Host observation 的 JCS digest 称为 Host authentication 或 enforcement receipt；
- 用 v14 Planning bootstrap 传 Ready；
- 只凭 activation request 中的 64hex 摘要生成 server authority；
- 忽略 V279 current binding、consent、credential 或 session currentness；
- 将 CPU-only 节点改写成虚假 GPU 节点；
- 让 source projection 激活 Provider、生成 route/Offer、预留 capacity、创建 Attempt/Lease 或移动资金；
- 宣称本批已编译、测试、运行、迁移或生产验收。

## 6. 下一验收门

下一阶段不是给本 DTO 加一个公开 constructor，而是形成可动态验收的 Host runtime authority。其最小故障矩阵至少包括：

- Runner 启动前/后崩溃与 custody 恢复；
- IPC 双向认证、重放、乱序、超时和响应大小限制；
- CPU/内存/显存/磁盘/进程/network enforcement 的成功与失败关闭；
- health TTL、generation、drain、disable、revoke、restart 后主动失效；
- CPU-only 与 accelerator target 的分别验收；
- trusted time 回退、Store currentness 漂移和跨重启恢复。

随后才进入 v15 authenticated session、Node 签名发布、server verifier/Store 与 current V279 binding 的组合验收。
