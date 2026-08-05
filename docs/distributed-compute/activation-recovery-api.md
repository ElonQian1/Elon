---
title: 分布式算力激活隔离恢复控制面
status: current
reviewed_at: 2026-08-05
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力激活隔离恢复控制面

## 1. 当前状态

v204 隔离恢复计划、第二人复核、只读预检和原子应用回执，以及 v205 prepared 计划追加式废止回执、管理员 HTTP 路由和 PC `/compute-activation` 管理界面已写入源码。当前尚未执行 Cargo/TypeScript 编译、v204/v205 迁移、HTTP 调用、并发验证、浏览器验收或发布，状态固定为 `implementation_uncompiled`。

本控制面不是删除 v181 隔离事实，也不是把数据库回滚到隔离前。它在保留原申请、激活计划、复核、应用和隔离回执的前提下，为同一 Provider 准备一个新的 active revision，并在条件满足时追加 `quarantined -> active` Pool 生命周期事件。历史事实保持可审计。

## 2. 恢复流程

1. v181 隔离回执必须存在，当前 Provider 与 CapacityPool 必须仍精确处于该回执绑定的 quarantined 版本。
2. 管理员提交当前 `quarantine_digest`、稳定幂等键、修复说明、1 至 20 项证据引用、已验证硬件摘要、信任层、验证时间，以及可选 Endpoint/Adapter 更新，显式准备恢复计划。
3. Store 在 `BEGIN IMMEDIATE` 事务中重审隔离回执、Provider/Pool 当前版本和目标 Provider 合同，保存绑定精确摘要的 prepared 计划。准备动作不解除隔离。
4. 不同于准备人的第二名 `admin/owner` 按当前 `plan_digest` 独立复核，生成禁止更新或删除的追加式回执。复核动作不解除隔离。
5. 若 prepared 计划已过期或目标内容有误，管理员可按当前 `plan_digest`、原因、稳定幂等键和显式确认生成 v205 废止回执；原计划与已有复核均保留，之后必须重新准备并重新复核。
6. 管理员读取 `compute_federation.activation_recovery_preflight.v1`，查看隔离摘要、Provider/Pool 版本、目标合同、第二人复核及旧 active Offer 是否已退场。
7. 只有预检无阻断项时 PC 才开放应用确认；服务端不信任该快照，写事务仍会重新检查全部条件。
8. 首次成功应用在一个事务中登记计划锁定的 active Provider 下一版本、追加 Pool `quarantined -> active` 事件、把恢复计划改为 applied，并写入不可变恢复应用回执。

## 3. 准备计划

准备请求要求：

- `idempotency_key`；
- 当前 64 位小写 `expected_quarantine_digest`；
- 可选 `endpoint` 和 `adapter` 路由引用，未提供时沿用当前 Provider 对应引用；
- 64 位小写 `verified_hardware_digest`；
- 非 `self_declared` 的 `trust_tier`；
- 合理 UTC `verified_at`；
- 非空 `remediation_summary`；
- 1 至 20 项非空 `evidence_refs`；
- `confirm_prepare=true`。

目标 Provider 保持稳定的 Provider ID、种类、所有者和创建时间，revision 必须是当前 quarantined revision 加一，状态固定为 active。目标更新时间由当前 Provider 更新时间和 `verified_at` 确定性选择，不使用每次请求的当前时钟，因此同一输入重试不会因时间变化生成不同目标摘要。

计划摘要绑定隔离回执、原 Provider 版本、Pool epoch/revision/digest、目标 Provider 规范 JSON、路由摘要、修复说明、证据引用摘要和准备人。相同幂等键或同一当前隔离只能重放完全相同的计划。

## 4. 第二人复核

复核请求提交稳定幂等键、当前 `plan_digest`、可选说明和 `confirm_review=true`。服务端要求计划仍为 prepared，摘要精确匹配，且 `reviewed_by_user_id` 与 `prepared_by_user_id` 不同。

复核回执绑定计划、请求、计划摘要、准备人、复核人、说明、请求摘要和服务端时间。数据库触发器禁止更新或删除；读取、预检和应用都会重新审计摘要及参与者分离。当前只强制准备人与复核人不同，没有要求第三名独立应用人或组织级多级审批。

## 5. 计划废止与重做

废止请求提交稳定 `idempotency_key`、当前 64 位小写 `expected_plan_digest`、非空 `reason` 和 `confirm_supersede=true`。服务端只接受当前摘要精确匹配的 prepared 计划，并在同一 `BEGIN IMMEDIATE` 事务中把计划切换为 superseded、固定服务端时间并写入 `compute_federation.activation_recovery_plan_supersession.v1` 回执。

回执绑定恢复计划、隔离、申请、Provider、Pool、原计划摘要、原因、执行人、请求摘要和服务端时间；数据库触发器禁止更新或删除。幂等重放必须匹配原请求摘要，读取时重新审计计划状态、绑定、时间和双层摘要。该操作的 `provider_effect`、`pool_effect`、`offer_effect`、`node_effect` 和 `money_effect` 均为 `none`。

废止不删除原计划或已有复核，也不会把旧复核转移到新计划。原 prepared 唯一约束释放后，管理员可以用新的幂等键按当前隔离事实重新准备计划；新计划必须生成新摘要并由不同于新准备人的管理员重新复核。

## 6. 恢复预检

`compute_federation.activation_recovery_preflight.v1` 是只读快照，检查：

- 恢复计划仍为 prepared；
- v181 隔离摘要及申请、应用、Provider、Pool、epoch 绑定未变化；
- 当前 Provider 仍是精确 quarantined 版本；
- 目标 Provider 身份、下一 revision、路由、verified 摘要、验证时间和信任层满足恢复合同；
- 当前 Pool 仍是计划绑定的 quarantined epoch/revision/digest；
- 当前 Provider 下不存在 active Offer；
- 第二人复核存在、摘要匹配且参与者分离有效。

稳定阻断码包括 `recovery_plan_not_prepared`、`quarantine_digest_changed`、`quarantine_binding_changed`、`provider_version_changed`、`provider_not_quarantined`、`target_provider_identity_changed`、`target_provider_revision_invalid`、`target_provider_not_ready`、`pool_provider_changed`、`pool_version_changed`、`pool_not_quarantined`、`active_offers_remaining`、`plan_review_missing`、`plan_review_digest_changed` 和 `plan_review_separation_invalid`。

只有阻断列表为空时 `ready_for_apply=true`。该值不是锁、授权、SLA 或执行结果；`recovery_effect` 固定为 `none`。

## 7. 管理员 HTTP

全部入口只允许平台 `admin/owner`，不向本人页面或 MCP 开放恢复写能力。

| 方法 | 路径 | 作用 |
|---|---|---|
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan` | 读取并审计当前恢复计划 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan` | 显式准备恢复计划，不解除隔离 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan/preflight` | 返回当前恢复阻断项和 active Offer 数量 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan/supersession` | 读取并审计最新计划废止回执 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan/supersession` | 以当前计划摘要、原因和显式确认追加废止回执 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan/review` | 读取并审计第二人复核回执 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan/review` | 不同于准备人的管理员复核精确计划 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan/application` | 读取并审计恢复应用回执 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan/application` | 以精确摘要和显式确认原子应用恢复 |

## 8. 应用效果

恢复应用返回：

- `provider_effect=active`；
- `pool_effect=active`；
- `offer_effect=none_active_offers_required`；
- `node_effect=none`；
- `money_effect=none`。

应用前，Provider 下所有旧 Offer 必须先通过既有 Offer 生命周期入口退出 active。恢复流程不会自动 draining/retire Offer，也不会重新发布任何旧 Offer。成功后商户或管理员如需恢复市场供给，必须基于新 Provider 版本单独准备并发布新合同。

恢复不发送节点启动、联网、升级或重连命令，不验证外部执行器在线，不取消既有业务合同，不退款、不付款、不修改结算账本，也不提交 Sui 或其他链上交易。

## 9. PC 管理界面

PC `/compute-activation` 在 v181 隔离回执下显示独立恢复区。准备弹窗可保留现有路由，或补充 Endpoint/Adapter 引用，并要求修复说明、证据引用、verified 摘要和显式确认。准备账号不显示复核按钮；第二名管理员复核后，界面展示不可变复核摘要和恢复预检阻断项。

应用按钮仅在 `ready_for_apply=true` 时可用，并显示当前 active Offer 数量。prepared 计划同时提供“废止计划”操作，复用原因和显式确认弹窗；成功后展示最新废止回执并重新开放准备入口。应用成功后展示恢复应用摘要和目标 Provider revision。前端快照不代替服务端事务检查。

## 10. 并发与历史边界

- 恢复计划、复核、废止和应用均使用稳定幂等键；相同键不能改变已绑定输入。
- 同一隔离回执同时只允许一份 prepared 恢复计划。
- 同一恢复计划最多一份复核；终态只能追加一份废止回执或一份应用回执，两者通过 prepared 状态 CAS 互斥。
- 废止释放 prepared 唯一约束但不复用旧复核；重新准备形成新的计划与复核链。
- 应用使用 `BEGIN IMMEDIATE`，任一 Provider、Pool、隔离、复核或 Offer 条件变化都会整笔失败。
- 原激活申请保持 activated，原计划保持 applied，v181 隔离回执不删除；恢复是追加事实，不伪装为隔离从未发生。
- 历史恢复回执审计绑定当时的 Provider 历史版本和 Pool 生命周期事件，不要求资源永远保持 active。

## 11. 尚未实现

- Cargo/TypeScript 编译、v204/v205 迁移、HTTP、权限、并发、浏览器和发布验证；
- prepared 恢复计划自动过期、按策略自动替换或无人值守废止；当前只支持管理员显式废止后重做；
- 恢复后再次隔离并形成第二个恢复周期；当前 v181 每份原激活应用最多一份隔离回执；
- 自动清退旧 Offer、自动发布新 Offer 或批量恢复市场供给；
- 第三名独立应用人、组织级审批、现实身份核验和证据签名链验证；
- 节点命令、真实派发、外部矿池联调、退款、付款、多币种或链上结算。

上游激活与隔离合同见 `docs/distributed-compute/activation-evidence-api.md`，Offer 安全退场见 `docs/distributed-compute/offer-api.md`。
