---
title: 外部矿池 Adapter runtime launch profile 验收边界
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter runtime launch profile 验收边界

## 本批状态

V255 的目标是 Domain、migration、Store、Service/API 与源码合同；本批明确禁止编译、执行 migration、运行测试或启动服务。未启动 entrypoint/Sidecar，未解析 secret，未连接 resolver backend、IPC/transport、probe、Runner、真实矿池、可信计量或结算。实际执行证据固定为 `passed=0`；下列是待运行矩阵与源码断言，不是通过声明。

## 待运行正向矩阵

- fresh database、V254→V255 upgrade、重复 migration、文件重开与 V249/V254 历史逐字兼容；V254 的 18 个 market absolute deny trigger 名称和 SQL body 不变；
- fresh create/successor 以 current V249 Prepared + current V254 candidate/delegation + server policy root 原子追加；owner/admin fresh `201`，exact actor-bound replay `200`；
- create exact replay 在 upstream/current policy 后续换代时仍只按历史 exact profile + fresh Prepared 恢复同一 receipt，不把新 policy 混入旧结果；
- owner/admin policy GET 返回相同 server-fixed summary/digest；policy builder 零数据库访问，endpoint 只有 historical candidate auth/path read 且零写，不能标记 candidate current；create 才精确消费该 digest 并重验 current roots，旧 digest 失败关闭；
- optional predecessor pair 全空创建首条，随后 exact structural latest pair 线性创建 successor；latest 即使已撤销也可在重新通过 current V249/V254/policy 后恢复 successor，旧 profile 保留历史，只有新 head 为 `launch_profile_current_inert`；
- owner/admin GET 重新审计 live filesystem 并在 Store 同次检查 binding、candidate、Provider、policy、predecessor/head，返回 inert currentness；
- owner/admin revoke fresh `201`、exact replay `200`；fresh revoke 只要求历史 exact profile/candidate 与 structural latest/unrevoked，不被 upstream/FS/policy 后续失效阻断，追加后 currentness 失败关闭；
- profile/revocation 的 `adapter_effect=none`、`runtime_effect=none`、`usage_effect=none`，且 Domain/DDL 中 Provider、credential、route、execution、market、settlement 等其余 fixed effects 也全部为 `none`；响应只保留这些稳定摘要，并递归隐藏 raw path/locator（包括 `entrypoint_relative_path`）、resolver backend root、credential、actor、幂等和 confirmation。

## 待运行失败关闭矩阵

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

HTTP source tests 分成共享 support 与场景 leaf，每个 leaf 小于 430 行。场景至少覆盖：owner/admin 双面三操作、auth/path ownership、unknown-field 注入、confirmation、exact replay、predecessor successor、root drift、live-FS drift、revocation、递归脱敏和完整零表效果。测试可定义为 Rust async tests，但本批不得运行；只允许静态检索、源码结构、行数、禁词/禁表和 diff 审核。

## 仍未验收与后续禁线

未验收 Rust compile、SQLite migration/upgrade/reopen/concurrency/crash、进程内或真实 TCP HTTP、生产数据库、真实 filesystem drift、secret custody、resolver、IPC/Sidecar、probe、runtime readiness、Provider activation、market、execution、usage 或 settlement。因此只能记录 `implementation_uncompiled / implementation_unrun / passed=0`。

V255 currentness 不是 runtime currentness/readiness，也不消费 V250/V252/V253 的短 TTL evidence。V254 temporary absolute deny 必须原样保留；后续不得仅因 profile current 就开放 CapacityPool 或 Offer。
