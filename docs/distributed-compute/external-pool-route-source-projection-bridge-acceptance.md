---
title: 外部矿池 Route source logical-to-projection bridge 验收
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_compiled
verification_status: source_review_only
---

# 外部矿池 Route source logical-to-projection bridge 验收

## 1. 当前证据强度

V271 当前完成权威合同与 migration/source-contract 源码静态复核，并已随完整 Windows `elon-server` 产品
目标和 WSL2 `elon-server` test target 编译；没有执行 V271 migration，也没有运行其 source-contract、SQLite、
并发、重开或 runtime 专属验证。正式动态计数为 `passed=0 / failed=0`，状态为
`source_review_only / implementation_compiled / implementation_unrun`。

本页定义 V271 migration 的验收门；源码存在不等于 migration 已执行或 trigger 已安装。语义唯一来源是
[`external-pool-route-source-projection-bridge-authority.md`](external-pool-route-source-projection-bridge-authority.md)。

## 2. 必须通过的 migration 合同

| 验收面 | 必须证明 |
|---|---|
| fresh upgrade | V270 schema 升级到 V271 前确认无既有 external-pool route row且 18 个 fence 齐全；升级后只替换 `trg_compute_route_authorization_exact_source`，无新 table/view/index/UDF |
| repeat upgrade | 第二次应用得到同一 trigger SQL，不产生业务 row 或漂移 |
| exact positive bridge | 先在 disposable 数据库完整执行 V270→V271 并确认 18 fences 在位，再只在该测试连接移除会阻止 route fixture 的相关 V254 inert fences；exact V221 application + exact V249 binding 只允许其唯一 `route_adapter_projection_id` |
| logical-ID rejection | `NEW.adapter_id=source.adapter_id` 但不等于 V249 projection ID 时失败关闭 |
| cross-root negative matrix | 任一 application/provider/owner/policy revision/provider digest/logical adapter/release/config/registry release/V254 candidate/delegation/projection/service actor/logical-binding 漂移都拒绝 |
| legacy branch regression | provider activation/recovery 两个 source branch 与公共 credential/Adapter/actor/revocation/TTL 检查保持原行为 |
| full-current fence audit | 完整 V271 schema 上 projection Adapter/version、credential、authorization/capability/seal 仍被 V254 拒绝，相关 row count 保持零 |
| reopen/direct SQL | 文件数据库重开后 trigger 仍 exact；直接 SQL 不能绕过 source bridge或任一 V254 fence |

positive bridge 必须使用完整应用 V271 后派生的 disposable trigger-only fixture，因为完整 current schema 上 V254
fence 正确地让 route projection 不可写。测试只能在隔离测试连接中移除相关 inert fence 后单独行使 source
trigger；不得为了得到“成功插入”而在生产 migration 中删除、disable 或缩窄 fence。

## 3. 静态边界检查

实现评审必须确认：

- V271 是 `store_migrations` module 与 migration registry 变更，不新增 Store/domain/API/router 入口；
- `source_kind/source_id/source_digest` ABI 不变，source 仍是 V221 application；
- trigger 不调用 SQLite UDF，不选择“latest”，不以 logical ID 或 release ID 代替 Provider-specific projection；
- projected Adapter capability JSON 只与 V249 release 原文/固定六 ID 顺序 exact，不跨 domain 比较 capability-set digest，也不证明六能力生产实现；
- 现有 route row 与 V221/V249 receipt 不 backfill、不 update/delete、不重新摘要；
- V254 18 个 trigger 名及 SQL 均无 diff，本批打开 fence 数为 `0`；
- 文档和状态页明确 Provider=`registering`、`activation_ready=false`、全部业务 effect=`none`。

## 4. 不属于 V271 的验收

以下均保持未实现，不能记入 V271 passed：

- 六能力 production producer/worker、authenticated ACK/event 与 prepare/commit/cancel/reconcile 网络协议；
- stable external-pool executor authority root；
- active Provider 的 V249/V254/V255/V258/V259/V270 refresh/successor currentness；
- V270 readiness 动态验收、atomic Provider activation、任一 V254 replacement guard；
- Pool/Offer admission、真实派发、计量、market、settlement 或部署。

## 5. 正式结论

当前 V271 只能声明“logical-to-projection source bridge 已写入并随完整目标编译，静态合同已复核”。在通过
第 2 节全部动态矩阵前，不得声明 migration 已验收；即使全部通过，也只能消除一个来源映射 P0。Atomic
activation 继续 NO-GO，Provider、route 与所有经济效果继续为零。
