---
title: 外部矿池 Route source logical-to-projection bridge 权威
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_partially_verified
verification_status: local_migration_and_source_bridge_verified
---

# 外部矿池 Route source logical-to-projection bridge 权威

## 1. 唯一语义：只修复来源身份映射

V271 冻结一个 migration-only 前置批次：只替换 v213 的
`trg_compute_route_authorization_exact_source`，让 `source_kind=external_pool_onboarding` 的 V221
logical Adapter identity 能经 exact V249 Provider binding 映射到其预留的
`route_adapter_projection_id`。它不创建 route，也不证明 Adapter 已实现任何派发能力。

当前 V221 source branch 要求 `source.adapter_id=NEW.adapter_id`。前者是 onboarding 保存的 logical
Adapter ID；未来 Provider-specific v213 Adapter row 必须使用 V249 按 Provider/registering revision、release
与 installation 派生的 projection ID。两个身份域故意不同，因此当前 trigger 无法同时证明 V221 exact source
和 V249 Provider-specific projection。V253 的 `registering -> adjacent active` 只处理 credential receipt
currentness，不修复这个来源外键语义。

V271 保留 route receipt ABI：`source_kind` 仍是 `external_pool_onboarding`，`source_id/source_digest` 仍引用
V221 application；V249 binding 只是 SQL 内部的 exact bridge，不替代 source，不进入新 JSON 字段或 API。

## 2. Exact bridge 矩阵

替换后的 external-pool branch 必须同时满足以下关系：

| 边 | 必须逐项相等的根 |
|---|---|
| route -> V221 | `NEW.source_id/source_digest` 对 application ID/digest；Provider ID/kind/owner、批准人、release、config revision/digest 与 application 一致 |
| V221 -> review/request | 保留 V221 已有 exact review/request join、`approved`、`applied`、owner/reviewer separation 及摘要检查 |
| V221 -> V249 binding | application ID/digest、Provider ID/owner、target Provider policy revision/digest、logical Adapter ID、release、config revision/digest 全部一致 |
| V249 binding -> V254 candidate | binding/release/installation/projection/Provider/logical Adapter/release/config 六组组成根逐项 exact；只接受 structural latest、未撤销 candidate/delegation 与未终止 installation/adoption |
| candidate -> route（V271 historical pre-V275） | `candidate.route_adapter_projection_id=NEW.adapter_id`、service actor exact；当时 dormant shape 还要求`candidate.logical_adapter_binding_digest=NEW.route_binding_digest=NEW.adapter_binding_digest` |
| V249 binding -> projection | `binding.route_adapter_projection_id=NEW.adapter_id` 且不等于 source logical Adapter ID；不得跨 Provider 借用 projection |
| V249 binding -> neutral release | registry release ID/digest、logical Adapter ID、release version 与 implementation exact，且 release 必须 current |

trigger 的公共前半段必须原样保留 v213 对 exact credential、Adapter version、service actor、TTL、revocation
和 allowed route kind 的检查；`provider_activation_application` 与 `provider_recovery_application` 两个 source
branch 也必须保持既有语义。V271 只改 external-pool source branch 的 logical-to-projection 比较。

projected Adapter 的 `supported_capabilities_json` 必须与 V249 release 原文 exact、数组长度为 6，并按 v213
固定顺序包含六个 capability ID；revision 继续来自 release JSON。这个比较只保持两个既有声明域一致，不是
六能力 producer/worker 或 runtime proof。V249 capability-set digest 与 v213 route digest 属于不同 domain，
V271 不把二者直接比较。`logical_projection_compatibility_digest` 也不由 SQLite UDF 重算；trigger 只审计
candidate canonical JSON 投影，并逐项锁定生成该 digest 的 binding/release/logical-binding/projection 组成根。

V275 supersedes上表最后一个digest等式，但不改写V271历史证据：`logical_adapter_binding_digest`只保留V221/V249/
V254 release/credential/source lineage，V275 active route的`NEW.route_binding_digest/NEW.adapter_binding_digest`必须等于
复用`ELON-COMPUTE-ATTEMPT-ADAPTER-BINDING-V1`计算的projection-shaped v211 digest。该v211 canonical shape仍只有
provider ID/kind、route kind、endpoint、Adapter ID/version与config revision/digest，V275只把Adapter ID映射为
`route_adapter_projection_id`；不得虚构Provider revision/digest/status或executor字段。planned active Provider pair与
stable executor由V275 activation-route binding/receipt另行绑定。source仍是`external_pool_onboarding`和exact V221
application，不得改成V275 receipt。

## 3. Migration 与历史语义

V271 作为 V270 后的新 migration 登记，不回写 V221 或 V249 历史 migration。当前源码在一个
`BEGIN IMMEDIATE` 中只执行以下动作：

1. 若已有 `source_kind=external_pool_onboarding` 或 `provider_kind=external_pool` route authorization row，要求
   独立 backfill 设计并让 migration 失败关闭，不静默重解释历史；
2. 从 `sqlite_master` 确认全部 18 个具名 V254 deny trigger 都存在；
3. `DROP TRIGGER IF EXISTS trg_compute_route_authorization_exact_source`；
4. 以同名 trigger 重建完整 v213/V221 语义，并加入第 2 节的 V249/V254 projection bridge；
5. commit；repeat migration 应得到同一 trigger definition，不追加业务 row。

已有非 external-pool route、V221 application 与 V249/V254 receipt 均保持不可变历史；V271 不 backfill、不
reinterpret、不 update 或 delete。即使 migration 已安装，V254 projection fences 仍会阻止任何 reserved projection Adapter/version、
credential、authorization/capability/seal 被插入，所以生产数据效果仍为零。

## 4. 严格零扩权边界

V271 不新增 table、view、index、SQLite UDF、receipt、currentness、Store method、domain type、Service、HTTP/MCP/
PC route、配置项或 runtime。它不读取 Secret、文件树或 V270 readiness，不启动 child、连接 upstream、领取
outbox 或接收 ACK/event。

V254 的 18 个 absolute deny 必须逐字保留；本批打开的 fence 数是 `0`。Provider 继续
`registering`，`activation_ready=false`。Provider、Adapter、credential、route、activation、execution、usage、
market、settlement effect 均为 `none`；没有新 durable effect 字段，也不得用 migration 成功冒充这些效果。

## 5. Atomic activation 继续 NO-GO

V271 只消除一个 schema-level P0，不足以创建 v213 route 或推进 Provider。后续 atomic activation 至少仍有
四个独立 P0：

- 六项 v213 capability 的真实 production producer/worker 与 authenticated ACK/event、prepare、idempotent
  commit、cancel-no-start、reconcile 协议仍不存在；V249/V268 的六能力声明不是运行实现；
- V275文档已冻结稳定`executor_id`/binding与projected v211 digest，但Rust/DDL/Store尚未实现；projection ID、service
  actor或短时process identity仍不能临时代替；
- V275文档已冻结active V253与fresh V274 successor/restart路径，但尚未实现/编译/运行；registering evidence仍不能
  被长期复用；
- V254 18 fences只能由V275 exact pending-plan UDF把#1/#5-#12九项改为permit；#2-#4/#13-#18九项保持absolute deny。
  在fresh/repeat/reopen、并发、crash、revocation、expiry、direct-SQL与完整失败原子性通过前不能由V271删除或旁路。

因此实现与production atomic activation仍为NO-GO。V275 final transaction必须在`evidence_checked_at`消费fresh
V270-equivalent/V272 Store-private authority、建立完整route/runtime closure、UPDATE既有Provider到adjacent active并
写version；#2 active Provider INSERT永久deny，任何失败整体回滚。V276才负责route renewal/reachability。

## 6. 计划实现边界

最窄代码落点为：

- `server/src/store_migrations/compute_external_pool_adapter_route_source_projection.rs`：V271 migration、
  existing-row/fence precheck 与同名 trigger replacement；
- `server/src/compute_federation/external_pool_adapter_release_api_tests/route_source_projection_migration_source_contract_test.rs`：静态 source contract；
- `server/src/compute_federation/external_pool_adapter_release_api_tests.rs`：只登记上述测试模块；
- `server/src/store_migrations.rs`：只登记 migration 271 及 module。

不得修改 `server/src/store.rs`、route/domain ABI、API/router 或 V254 fence 文件。当前源码已表达该合同并随
完整 Windows 产品目标与 WSL2 test target 编译；Windows 受管 `v271_` 6/6 已覆盖 fresh/repeat/reopen、
missing-fence/existing-route 失败关闭、唯一 projection 正向和 8 类跨根漂移拒绝。完整 current fence 写入、
legacy branch 动态、并发、崩溃和旧库升级矩阵仍未运行；证据强度见
[`external-pool-route-source-projection-bridge-acceptance.md`](external-pool-route-source-projection-bridge-acceptance.md)。
