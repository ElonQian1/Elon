---
title: 外部矿池 Adapter runtime launch profile 验收边界
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
implementation_status: implementation_locally_verified
verification_status: targeted_local_verified
---

# 外部矿池 Adapter runtime launch profile 验收边界

## 本批状态

V255 Domain、migration、Store、Service/API 与源码合同已完成本地定向验证。`cargo test --manifest-path server/Cargo.toml --bin elon-server runtime_launch_profile --locked` 实际命中 `12 passed / 0 failed / 1875 filtered out`，正式验证指纹为 `e6919db4d7535bae1e8fc4017e1c7e829a3ad0ce23407e3c29c636a5557c0575`，收据摘要为 `e4a5779153d0f60de92d05d18e037ee80c2547223261b23f9030b796a1835da8`。该命令没有启动 entrypoint/Sidecar，没有解析 secret，也没有连接 resolver backend、IPC/transport、probe、Runner、真实矿池、可信计量或结算。

## 已执行的本地定向矩阵

- 7 项 migration/Store/source tests：fresh database、重复 Store migration、V255 schema objects、DDL/Store ABI、45 字段 server policy、完整 receipt projection、exact roots、append-only lineage、display-only current view 和 V254 18 个 absolute deny 源码逐字保护；
- 2 项 HTTP source contract tests：服务端固定策略、公开投影脱敏、无 process spawn、无 secret/resolver 读取、无 Provider/route/market/usage/settlement 下游写路径；
- 1 项 owner HTTP test：策略读取、fresh create、exact replay、currentness、权限/输入失败关闭、公开响应递归脱敏和零下游表效果；
- 2 项 admin HTTP tests：管理员 linear successor 修复，以及 filesystem drift 后仍可撤销并 exact replay；Provider 始终保持 `registering`，profile 始终为 inert。

## 仍需扩展的失败关闭与环境矩阵

- 无会话 `401`；非 binding owner 或非 platform `admin|owner` 为 `403`；identifier/digest/reason/confirmation/predecessor pair 非法为 `400`；missing binding/candidate/profile 为 `404`；malformed/unknown body 为 `422`；
- body 注入 policy 或任一 policy field、Provider/release/installation/entrypoint/route/service actor、credential/resolver backend、recorded/revoked actor、timestamp/status/effect，一律 `422`；
- path binding/candidate/profile 不一致，candidate/delegation revoked/historical，Provider revision/digest/status/owner 漂移，registry binding/release、installation content 或 live-FS 漂移；
- historical onboarding credential subject 缺失，scheme 不是 `vault_ref`，或 locator commitment 与 V249/Prepared 漂移；
- expected candidate/binding/policy digest 漂移，predecessor pair 半空、遗漏 structural latest predecessor、引用非 latest predecessor、分叉 successor或 replay material/actor 漂移；不得把 predecessor 已撤销误判成永久禁用；
- SQL insert/update/delete/replace、canonical JSON 与物化列漂移、current head 分叉、revocation 重复或撤销后续写；
- 任意 profile/currentness/revoke 路径出现 runtime start、secret resolution、Provider active、v213 authority、market 或 execution/settlement 副作用。

上述失败均须比较事务前后 profile、head 与 revocation 计数，证明没有半写；不能用响应中的 `none` 字段代替数据库差分。

## 零表效应源码断言

每个 create/replay/currentness/revoke 成功场景必须保存写前快照并断言：

- Provider status、policy revision、digest 原样保持 exact `registering`；V254 delegation/candidate 不变；
- v213 Adapter/version、credential/version/revocation、authorization/capability/seal、service actor authorization、route/version 全零；
- `compute_capacity_pools`/versions、Offer/versions、Price Snapshot、Job/versions、Reservation/versions 全零；
- Attempt activation、Execution Plan/Seal、Lease、dispatch command/application/ACK 与 Start outbox/send/remote observation/event 全零；
- usage declaration/snapshot/Execution Receipt、settlement/posting/account balance/ledger leg/release/withdrawal/payment 全零；
- V255 只允许 profile 与 profile revocation 自身发生预期写效果；current head 由 immutable lineage 派生而非 mutable head 表，GET 必须完全只读。

源码合同还须扫描 migration/Store/Service/API，拒绝出现对上述下游表的 INSERT/UPDATE/DELETE/REPLACE，拒绝 process spawn、command execution、network client/listener、credential locator 读取/解析或 Provider activation 调用。V254 18 个 deny trigger 必须以 exact source parity 断言防止被删除、改名或改写。

## HTTP 源码边界

HTTP source tests 分成共享 support 与场景 leaf，每个 leaf 小于 430 行。当前 5 项 HTTP/source contract tests 已实际运行，覆盖 owner/admin、auth/path ownership、unknown-field 注入、confirmation、exact replay、predecessor successor、root drift、live-FS drift、revocation、递归脱敏和零下游表效果。后续增加真实 TCP 或生产环境验收时仍须保留同一公开投影和零副作用边界。

## 仍未验收与后续禁线

已验收本机 Rust compile、fresh/repeat SQLite migration、进程内 Axum HTTP 与测试 fixture filesystem drift。尚未验收停在 exact V254 的磁盘升级副本、SQLite concurrency/crash recovery、真实 TCP HTTP、生产数据库、生产 secret custody/resolver、IPC/Sidecar、probe、runtime readiness、Provider activation、market、execution、usage 或 settlement。因此只能记录 `implementation_locally_verified`，不能记录 production ready。

V255 currentness 不是 runtime currentness/readiness，也不消费 V250/V252/V253 的短 TTL evidence。V254 temporary absolute deny 必须原样保留；后续不得仅因 profile current 就开放 CapacityPool 或 Offer。
